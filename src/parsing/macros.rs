use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MacroConstantKind {
    Integer,
    Float,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMacroValue {
    pub kind: MacroConstantKind,
    pub value: String,
}

pub fn parse_macro_value(tokens: &[String]) -> Option<ParsedMacroValue> {
    let tokens = strip_wrapping_parentheses(tokens);
    if tokens.is_empty() {
        return None;
    }

    let integer_value = tokens.join("");
    if let Some(value) = normalize_macro_integer_literal(&integer_value) {
        return Some(ParsedMacroValue {
            kind: MacroConstantKind::Integer,
            value,
        });
    }
    if let Some(value) = normalize_macro_float_literal(&integer_value) {
        return Some(ParsedMacroValue {
            kind: MacroConstantKind::Float,
            value,
        });
    }

    let strings = tokens
        .iter()
        .map(|token| normalize_string_literal(token))
        .collect::<Option<Vec<_>>>()?;
    Some(ParsedMacroValue {
        kind: MacroConstantKind::String,
        value: strings.join(" + "),
    })
}

fn strip_wrapping_parentheses(tokens: &[String]) -> &[String] {
    let mut current = tokens;
    while is_wrapped_in_parentheses(current) {
        current = &current[1..current.len() - 1];
    }
    current
}

fn is_wrapped_in_parentheses(tokens: &[String]) -> bool {
    if tokens.len() < 3
        || tokens.first().map(String::as_str) != Some("(")
        || tokens.last().map(String::as_str) != Some(")")
    {
        return false;
    }

    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.as_str() {
            "(" => depth += 1,
            ")" => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return false,
                };
                if depth == 0 && index != tokens.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn normalize_macro_integer_literal(value: &str) -> Option<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return None;
    }

    if is_supported_integer_literal(normalized) {
        Some(normalized.to_string())
    } else {
        None
    }
}

fn is_supported_integer_literal(value: &str) -> bool {
    let value = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    if value.is_empty() {
        return false;
    }

    let digits_end = value
        .char_indices()
        .rfind(|(_, ch)| !matches!(ch, 'u' | 'U' | 'l' | 'L'))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let (digits, suffix) = value.split_at(digits_end);
    if digits.is_empty()
        || suffix
            .chars()
            .any(|ch| !matches!(ch, 'u' | 'U' | 'l' | 'L'))
    {
        return false;
    }

    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
    } else {
        digits.chars().all(|ch| ch.is_ascii_digit())
    }
}

fn normalize_macro_float_literal(value: &str) -> Option<String> {
    let normalized = value.trim();
    let (sign, rest) = split_number_sign(normalized);
    if rest.is_empty() {
        return None;
    }

    let body = strip_float_suffix(rest);
    if body.starts_with("0x") || body.starts_with("0X") {
        validate_hex_float(body)?;
        return Some(format!("{sign}{body}"));
    }

    let normalized_body = normalize_decimal_float(body)?;
    Some(format!("{sign}{normalized_body}"))
}

fn split_number_sign(value: &str) -> (&str, &str) {
    value
        .strip_prefix('+')
        .map(|rest| ("+", rest))
        .or_else(|| value.strip_prefix('-').map(|rest| ("-", rest)))
        .unwrap_or(("", value))
}

fn strip_float_suffix(value: &str) -> &str {
    value.strip_suffix(['f', 'F', 'l', 'L']).unwrap_or(value)
}

fn normalize_decimal_float(value: &str) -> Option<String> {
    let (mantissa, exponent) = split_decimal_exponent(value)?;
    let has_exponent = exponent.is_some();
    let mantissa = normalize_decimal_mantissa(mantissa, has_exponent)?;
    if let Some(exponent) = exponent {
        validate_exponent_digits(&exponent[1..])?;
        Some(format!("{mantissa}{exponent}"))
    } else {
        Some(mantissa)
    }
}

fn split_decimal_exponent(value: &str) -> Option<(&str, Option<&str>)> {
    let exponent_index = value.find(['e', 'E']);
    match exponent_index {
        Some(index) => {
            let (mantissa, exponent) = value.split_at(index);
            Some((mantissa, Some(exponent)))
        }
        None => Some((value, None)),
    }
}

fn normalize_decimal_mantissa(value: &str, has_exponent: bool) -> Option<String> {
    if let Some((left, right)) = value.split_once('.') {
        if left.is_empty() && right.is_empty() {
            return None;
        }
        if !left.chars().all(|ch| ch.is_ascii_digit())
            || !right.chars().all(|ch| ch.is_ascii_digit())
        {
            return None;
        }
        let left = if left.is_empty() { "0" } else { left };
        let right = if right.is_empty() { "0" } else { right };
        return Some(format!("{left}.{right}"));
    }

    if has_exponent && !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()) {
        Some(value.to_string())
    } else {
        None
    }
}

fn validate_exponent_digits(value: &str) -> Option<()> {
    let digits = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    (!digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())).then_some(())
}

fn validate_hex_float(value: &str) -> Option<()> {
    let rest = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))?;
    let exponent_index = rest.find(['p', 'P'])?;
    let (mantissa, exponent) = rest.split_at(exponent_index);
    validate_hex_mantissa(mantissa)?;
    validate_exponent_digits(&exponent[1..])
}

fn validate_hex_mantissa(value: &str) -> Option<()> {
    let mut hex_digits = 0usize;
    let mut dots = 0usize;
    for ch in value.chars() {
        if ch == '.' {
            dots += 1;
            if dots > 1 {
                return None;
            }
            continue;
        }
        if ch.is_ascii_hexdigit() {
            hex_digits += 1;
        } else {
            return None;
        }
    }
    (hex_digits > 0).then_some(())
}

fn normalize_string_literal(token: &str) -> Option<String> {
    if !token.starts_with('"') || !token.ends_with('"') || token.len() < 2 {
        return None;
    }

    let decoded = decode_c_string_literal_body(&token[1..token.len() - 1])?;
    Some(quote_go_byte_string(&decoded))
}

fn decode_c_string_literal_body(value: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\n' || ch == '\r' {
            return None;
        }
        if ch != '\\' {
            push_utf8(&mut out, ch);
            continue;
        }

        let escaped = chars.next()?;
        match escaped {
            '\'' => out.push(b'\''),
            '"' => out.push(b'"'),
            '?' => out.push(b'?'),
            '\\' => out.push(b'\\'),
            'a' => out.push(0x07),
            'b' => out.push(0x08),
            'f' => out.push(0x0c),
            'n' => out.push(b'\n'),
            'r' => out.push(b'\r'),
            't' => out.push(b'\t'),
            'v' => out.push(0x0b),
            '\n' => {}
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            '0'..='7' => out.push(decode_octal_escape(escaped, &mut chars)?),
            'x' => out.push(decode_hex_escape(&mut chars)?),
            'u' => push_utf8(&mut out, decode_universal_character_name(&mut chars, 4)?),
            'U' => push_utf8(&mut out, decode_universal_character_name(&mut chars, 8)?),
            _ => return None,
        }
    }

    Some(out)
}

fn decode_octal_escape(
    first: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Option<u8> {
    let mut value = first.to_digit(8)?;
    for _ in 0..2 {
        let Some(next) = chars.peek().copied() else {
            break;
        };
        let Some(digit) = next.to_digit(8) else {
            break;
        };
        chars.next();
        value = value * 8 + digit;
    }
    u8::try_from(value).ok()
}

fn decode_hex_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<u8> {
    let mut value = 0u32;
    let mut seen = false;
    while let Some(next) = chars.peek().copied() {
        let Some(digit) = next.to_digit(16) else {
            break;
        };
        chars.next();
        value = value.checked_mul(16)?.checked_add(digit)?;
        seen = true;
    }
    seen.then_some(())?;
    u8::try_from(value).ok()
}

fn decode_universal_character_name(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    digits: usize,
) -> Option<char> {
    let mut value = 0u32;
    for _ in 0..digits {
        let digit = chars.next()?.to_digit(16)?;
        value = value.checked_mul(16)?.checked_add(digit)?;
    }
    char::from_u32(value)
}

fn push_utf8(out: &mut Vec<u8>, ch: char) {
    let mut buf = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}

fn quote_go_byte_string(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    for byte in bytes {
        match *byte {
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(*byte as char),
            _ => out.push_str(&format!("\\x{byte:02x}")),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::{MacroConstantKind, parse_macro_value};

    fn tokens(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_integer_literals() {
        let value = parse_macro_value(&tokens(&["(", "0x01U", ")"])).unwrap();

        assert_eq!(value.kind, MacroConstantKind::Integer);
        assert_eq!(value.value, "0x01U");
    }

    #[test]
    fn parses_float_literals() {
        let cases = [
            ("1.5", "1.5"),
            ("1.", "1.0"),
            (".5", "0.5"),
            ("1e-3", "1e-3"),
            ("+2.0f", "+2.0"),
            ("-3.0L", "-3.0"),
            ("0x1.8p+1F", "0x1.8p+1"),
        ];

        for (input, expected) in cases {
            let value = parse_macro_value(&tokens(&[input])).unwrap();

            assert_eq!(value.kind, MacroConstantKind::Float);
            assert_eq!(value.value, expected);
        }
    }

    #[test]
    fn parses_string_literals() {
        let value = parse_macro_value(&tokens(&["(", "\"hello\\n\"", ")"])).unwrap();

        assert_eq!(value.kind, MacroConstantKind::String);
        assert_eq!(value.value, "\"hello\\n\"");
    }

    #[test]
    fn parses_c_string_escapes_as_go_byte_strings() {
        let value = parse_macro_value(&tokens(&["\"a\\?\\x7f\""])).unwrap();

        assert_eq!(value.kind, MacroConstantKind::String);
        assert_eq!(value.value, "\"a?\\x7f\"");
    }

    #[test]
    fn parses_universal_character_names() {
        let value = parse_macro_value(&tokens(&["\"caf\\u00e9 \\U0001f680\""])).unwrap();

        assert_eq!(value.kind, MacroConstantKind::String);
        assert_eq!(value.value, "\"caf\\xc3\\xa9 \\xf0\\x9f\\x9a\\x80\"");
    }

    #[test]
    fn parses_adjacent_string_literals() {
        let value = parse_macro_value(&tokens(&["\"hello\"", "\" world\""])).unwrap();

        assert_eq!(value.kind, MacroConstantKind::String);
        assert_eq!(value.value, "\"hello\" + \" world\"");
    }

    #[test]
    fn rejects_unsupported_macro_values() {
        assert!(parse_macro_value(&tokens(&["MAKE_FLAG", "(", "1", ")"])).is_none());
        assert!(parse_macro_value(&tokens(&["u8\"hello\""])).is_none());
        assert!(parse_macro_value(&tokens(&["R\"(hello)\""])).is_none());
        assert!(parse_macro_value(&tokens(&["'x'"])).is_none());
        assert!(parse_macro_value(&tokens(&["1f"])).is_none());
        assert!(parse_macro_value(&tokens(&["1e"])).is_none());
        assert!(parse_macro_value(&tokens(&["0x1.2"])).is_none());
        assert!(parse_macro_value(&tokens(&["\"a\"", "+", "\"b\""])).is_none());
    }
}
