use std::{
    ffi::CStr,
    fs,
    path::{Path, PathBuf},
    ptr,
};

use clang_sys::*;

use crate::parsing::model::CppOperatorToken;

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

    let definition = unsafe { clang_getCursorDefinition(cursor) };
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
    unsafe {
        let translation_unit = clang_Cursor_getTranslationUnit(cursor);
        if translation_unit.is_null() {
            return false;
        }

        let mut tokens = ptr::null_mut();
        let mut token_count = 0;
        clang_tokenize(
            translation_unit,
            clang_getCursorExtent(cursor),
            &mut tokens,
            &mut token_count,
        );
        if tokens.is_null() || token_count == 0 {
            return false;
        }

        let slice = std::slice::from_raw_parts(tokens, token_count as usize);
        let has_body = slice.iter().any(|token| {
            cxstring_to_string(clang_getTokenSpelling(translation_unit, *token)) == "{"
        });
        clang_disposeTokens(translation_unit, tokens, token_count);
        has_body
    }
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

fn cursor_file_path(cursor: CXCursor) -> Option<PathBuf> {
    unsafe {
        let location = clang_getCursorLocation(cursor);
        if clang_equalLocations(location, clang_getNullLocation()) != 0 {
            return None;
        }

        let mut file = ptr::null_mut();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;
        clang_getExpansionLocation(location, &mut file, &mut line, &mut column, &mut offset);
        if file.is_null() {
            return None;
        }
        let raw = cxstring_to_string(clang_getFileName(file));
        if raw.is_empty() {
            None
        } else {
            Some(PathBuf::from(raw))
        }
    }
}

fn cursor_file_offset(cursor: CXCursor) -> Option<u32> {
    unsafe {
        let location = clang_getCursorLocation(cursor);
        if clang_equalLocations(location, clang_getNullLocation()) != 0 {
            return None;
        }

        let mut file = ptr::null_mut();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;
        clang_getExpansionLocation(location, &mut file, &mut line, &mut column, &mut offset);
        if file.is_null() { None } else { Some(offset) }
    }
}

unsafe fn cxstring_to_string(raw: CXString) -> String {
    let value = unsafe { clang_getCString(raw) };
    let owned = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    unsafe { clang_disposeString(raw) };
    owned
}
