use std::{fs, path::Path, ptr};

use clang_sys::*;

use crate::parsing::{
    model::CppOperatorToken,
    parser::{cursor_file_path, cxstring_to_string},
};

pub(super) fn is_operator_spelling(spelling: &str) -> bool {
    spelling.trim().starts_with("operator")
}

pub(super) fn operator_token(spelling: &str) -> CppOperatorToken {
    match spelling.trim().strip_prefix("operator").map(str::trim) {
        Some("+") => CppOperatorToken::Plus,
        Some("-") => CppOperatorToken::Minus,
        Some("*") => CppOperatorToken::Multiply,
        Some("/") => CppOperatorToken::Divide,
        Some("%") => CppOperatorToken::Modulo,
        Some("==") => CppOperatorToken::Equal,
        Some("=") => CppOperatorToken::Assign,
        Some("+=") => CppOperatorToken::PlusAssign,
        Some("-=") => CppOperatorToken::MinusAssign,
        Some("*=") => CppOperatorToken::MultiplyAssign,
        Some("/=") => CppOperatorToken::DivideAssign,
        Some("%=") => CppOperatorToken::ModuloAssign,
        Some("!=") => CppOperatorToken::NotEqual,
        Some("<") => CppOperatorToken::Less,
        Some("<=") => CppOperatorToken::LessEq,
        Some(">") => CppOperatorToken::Greater,
        Some(">=") => CppOperatorToken::GreaterEq,
        Some("<=>") => CppOperatorToken::Spaceship,
        Some("&") => CppOperatorToken::Amp,
        Some("|") => CppOperatorToken::Pipe,
        Some("^") => CppOperatorToken::Caret,
        Some("~") => CppOperatorToken::Tilde,
        Some("!") => CppOperatorToken::Not,
        Some("&=") => CppOperatorToken::AmpAssign,
        Some("|=") => CppOperatorToken::PipeAssign,
        Some("^=") => CppOperatorToken::CaretAssign,
        Some("<<") => CppOperatorToken::LessLess,
        Some(">>") => CppOperatorToken::GreaterGreater,
        Some("<<=") => CppOperatorToken::LessLessAssign,
        Some(">>=") => CppOperatorToken::GreaterGreaterAssign,
        Some("&&") => CppOperatorToken::AndAnd,
        Some("||") => CppOperatorToken::OrOr,
        Some(",") => CppOperatorToken::Comma,
        Some("->") => CppOperatorToken::Arrow,
        Some("->*") => CppOperatorToken::ArrowStar,
        Some("[]") => CppOperatorToken::Array,
        Some("()") => CppOperatorToken::Func,
        Some("++") => CppOperatorToken::Increment,
        Some("--") => CppOperatorToken::Decrement,
        Some("new" | "delete" | "new[]" | "delete[]") => {
            CppOperatorToken::Unsupported(spelling.to_string())
        }
        Some(other) if !other.is_empty() && is_conversion_operator_target(other) => {
            CppOperatorToken::Conversion(other.to_string())
        }
        Some(other) => CppOperatorToken::Unsupported(other.to_string()),
        None => CppOperatorToken::Unsupported(spelling.to_string()),
    }
}

pub(super) fn has_header_definition(cursor: CXCursor) -> bool {
    if source_scan_has_header_definition(cursor) {
        return true;
    }
    if cursor_has_body_tokens(cursor) {
        return cursor_file_path(cursor)
            .as_deref()
            .is_some_and(is_header_path);
    }

    // SAFETY: Parser traversal keeps this operator cursor's translation unit alive.
    let definition = unsafe { clang_getCursorDefinition(cursor) };
    // SAFETY: The definition came from libclang; comparing a null cursor is supported.
    if unsafe { clang_equalCursors(definition, clang_getNullCursor()) } != 0 {
        return false;
    }

    let Some(definition_path) = cursor_file_path(definition) else {
        return false;
    };
    if !is_header_path(&definition_path) {
        return false;
    }

    let Some(declaration_path) = cursor_file_path(cursor) else {
        return false;
    };
    same_path(&definition_path, &declaration_path)
}

fn is_conversion_operator_target(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
}

fn source_scan_has_header_definition(cursor: CXCursor) -> bool {
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    if !is_header_path(&path) {
        return false;
    }
    let Some(offset) = cursor_file_offset(cursor) else {
        return false;
    };
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    first_signature_terminator_is_body(&source, offset as usize)
}

fn first_signature_terminator_is_body(source: &str, offset: usize) -> bool {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let start = source
        .char_indices()
        .find(|(index, _)| *index >= offset)
        .map(|(index, _)| index)
        .unwrap_or(source.len());
    let mut chars = source[start..].chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                for comment_ch in chars.by_ref() {
                    if comment_ch == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '\0';
                for comment_ch in chars.by_ref() {
                    if previous == '*' && comment_ch == '/' {
                        break;
                    }
                    previous = comment_ch;
                }
            }
            '"' | '\'' => {
                let quote = ch;
                let mut escaped = false;
                for literal_ch in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if literal_ch == '\\' {
                        escaped = true;
                    } else if literal_ch == quote {
                        break;
                    }
                }
            }
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' if paren_depth == 0 && bracket_depth == 0 => return true,
            ';' if paren_depth == 0 && bracket_depth == 0 => return false,
            _ => {}
        }
    }

    false
}

fn cursor_has_body_tokens(cursor: CXCursor) -> bool {
    // SAFETY: Parser traversal keeps the cursor's translation unit alive.
    let translation_unit = unsafe { clang_Cursor_getTranslationUnit(cursor) };
    if translation_unit.is_null() {
        return false;
    }

    let mut tokens = ptr::null_mut();
    let mut token_count = 0;
    // SAFETY: The cursor extent belongs to this live translation unit, and the output
    // pointers are writable for libclang's token allocation and its length.
    unsafe {
        clang_tokenize(
            translation_unit,
            clang_getCursorExtent(cursor),
            &mut tokens,
            &mut token_count,
        );
    }
    if tokens.is_null() {
        return false;
    }

    // SAFETY: Libclang returned a non-null, aligned buffer of token_count initialized
    // tokens, kept allocated until after this slice's final use.
    let slice = unsafe { std::slice::from_raw_parts(tokens, token_count as usize) };
    let has_body = slice.iter().any(|token| {
        // SAFETY: Each token belongs to this live translation unit and token buffer;
        // the newly returned spelling is consumed and disposed exactly once.
        unsafe { cxstring_to_string(clang_getTokenSpelling(translation_unit, *token)) == "{" }
    });
    // SAFETY: This allocation came from clang_tokenize for this live translation unit.
    // Release every non-null buffer exactly once, including zero-token allocations.
    unsafe { clang_disposeTokens(translation_unit, tokens, token_count) };
    has_body
}

fn same_path(path: &Path, target: &Path) -> bool {
    if path == target {
        return true;
    }
    match (path.canonicalize(), target.canonicalize()) {
        (Ok(path), Ok(target)) => path == target,
        _ => false,
    }
}

fn is_header_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

fn cursor_file_offset(cursor: CXCursor) -> Option<u32> {
    // SAFETY: Parser traversal keeps the cursor's translation unit alive.
    let location = unsafe { clang_getCursorLocation(cursor) };
    // SAFETY: The location came from libclang, and a null location is valid for comparison.
    if unsafe { clang_equalLocations(location, clang_getNullLocation()) } != 0 {
        return None;
    }

    let mut file = ptr::null_mut();
    let mut offset = 0;
    // SAFETY: The location is live, file and offset are writable, and null pointers
    // omit the unneeded line and column outputs.
    unsafe {
        clang_getExpansionLocation(
            location,
            &mut file,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut offset,
        );
    }
    if file.is_null() { None } else { Some(offset) }
}
