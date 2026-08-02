//! General tools which don't depend on other parts of Parsimonious.
//!
//! Port of `parsimonious/utils.py`. The Python original's `StrAndRepr` mixin
//! (a class that makes `__repr__` delegate to `__str__`) has no direct Rust
//! counterpart — the same effect is achieved on [`Token`] below by
//! implementing [`fmt::Debug`] in terms of [`fmt::Display`], so a type-for-type
//! translation was skipped in favor of that idiom.

use std::error::Error;
use std::fmt;

/// The result of [`evaluate_string`]: Python's `ast.literal_eval` can return
/// either a `str` or a `bytes` object depending on the literal's prefix, and
/// both are meaningful grammar literals, so both are kept instead of
/// collapsing to one Rust type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLiteral {
    Str(String),
    Bytes(Vec<u8>),
}

/// Everything that can go wrong while evaluating a Python string-literal
/// token. Mirrors the `SyntaxError`/`ValueError` Python raises out of
/// `ast.literal_eval` on a malformed literal, without going through Python's
/// own parser to get there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalStringError {
    NotAQuotedString,
    UnknownPrefix(String),
    Unterminated,
    TrailingData,
    TruncatedEscape(char),
    InvalidHexDigit(char),
    InvalidCodePoint(u32),
    NonAsciiByte(char),
    UnsupportedNamedEscape,
}

impl fmt::Display for EvalStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalStringError::NotAQuotedString => write!(f, "not a quoted string literal"),
            EvalStringError::UnknownPrefix(p) => write!(f, "unrecognized string prefix {p:?}"),
            EvalStringError::Unterminated => write!(f, "unterminated string literal"),
            EvalStringError::TrailingData => write!(f, "unexpected data after the closing quote"),
            EvalStringError::TruncatedEscape(c) => write!(f, "truncated \\{c} escape"),
            EvalStringError::InvalidHexDigit(c) => write!(f, "\\{c} escape contains a non-hex digit"),
            EvalStringError::InvalidCodePoint(v) => write!(f, "{v:#x} is not a valid unicode code point"),
            EvalStringError::NonAsciiByte(c) => {
                write!(f, "bytes literal cannot contain the non-ASCII character {c:?}")
            }
            EvalStringError::UnsupportedNamedEscape => {
                // A `\N{...}` escape needs a full Unicode name table, which this
                // crate does not vendor; reported as an error instead of guessing.
                write!(f, "\\N{{...}} named escapes are not supported")
            }
        }
    }
}

impl Error for EvalStringError {}

/// Piggyback on Python's string-literal syntax so grammar literals can use
/// backslash escaping and niceties like `\n`, `\t`, etc.
///
/// This also supports:
/// 1. `b"strings"`, allowing grammars to parse bytestrings, in addition to `str`.
/// 2. `r"strings"` to simplify regexes.
///
/// `string` is the literal token text itself, quotes and prefix included
/// (e.g. `r#"r"foo\d+""#`), matching what `ast.literal_eval` was fed in the
/// Python original.
pub fn evaluate_string(string: &str) -> Result<StringLiteral, EvalStringError> {
    let (prefix, quoted) = split_prefix(string)?;
    let (is_raw, is_bytes) = match prefix.to_ascii_lowercase().as_str() {
        "" | "u" => (false, false),
        "r" => (true, false),
        "b" => (false, true),
        "rb" | "br" => (true, true),
        _ => return Err(EvalStringError::UnknownPrefix(prefix.to_string())),
    };
    let (_quote, inner) = strip_quotes(quoted)?;
    if is_raw {
        encode_raw(inner, is_bytes)
    } else if is_bytes {
        decode_bytes_escapes(inner).map(StringLiteral::Bytes)
    } else {
        decode_str_escapes(inner).map(StringLiteral::Str)
    }
}

fn split_prefix(s: &str) -> Result<(&str, &str), EvalStringError> {
    let quote_pos = s
        .find(|c| c == '"' || c == '\'')
        .ok_or(EvalStringError::NotAQuotedString)?;
    Ok((&s[..quote_pos], &s[quote_pos..]))
}

/// Finds the first unescaped closing quote matching the opening one and
/// returns the quote char plus the content between the quotes.
fn strip_quotes(body: &str) -> Result<(char, &str), EvalStringError> {
    let mut chars = body.char_indices();
    let (_, open) = chars.next().ok_or(EvalStringError::NotAQuotedString)?;
    if open != '"' && open != '\'' {
        return Err(EvalStringError::NotAQuotedString);
    }
    let mut escaped = false;
    for (i, c) in chars.by_ref() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == open {
            let inner = &body[open.len_utf8()..i];
            let rest = &body[i + c.len_utf8()..];
            return if rest.is_empty() {
                Ok((open, inner))
            } else {
                Err(EvalStringError::TrailingData)
            };
        }
    }
    Err(EvalStringError::Unterminated)
}

fn encode_raw(inner: &str, is_bytes: bool) -> Result<StringLiteral, EvalStringError> {
    if !is_bytes {
        return Ok(StringLiteral::Str(inner.to_string()));
    }
    let mut out = Vec::with_capacity(inner.len());
    for c in inner.chars() {
        if !c.is_ascii() {
            return Err(EvalStringError::NonAsciiByte(c));
        }
        out.push(c as u8);
    }
    Ok(StringLiteral::Bytes(out))
}

fn decode_str_escapes(inner: &str) -> Result<String, EvalStringError> {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }
        let esc = *chars.get(i + 1).ok_or(EvalStringError::Unterminated)?;
        match esc {
            '\n' => i += 2,
            '\\' => {
                out.push('\\');
                i += 2;
            }
            '\'' => {
                out.push('\'');
                i += 2;
            }
            '"' => {
                out.push('"');
                i += 2;
            }
            'a' => {
                out.push('\u{07}');
                i += 2;
            }
            'b' => {
                out.push('\u{08}');
                i += 2;
            }
            'f' => {
                out.push('\u{0C}');
                i += 2;
            }
            'n' => {
                out.push('\n');
                i += 2;
            }
            'r' => {
                out.push('\r');
                i += 2;
            }
            't' => {
                out.push('\t');
                i += 2;
            }
            'v' => {
                out.push('\u{0B}');
                i += 2;
            }
            '0'..='7' => {
                let (value, count) = read_octal(&chars, i + 1);
                out.push(codepoint_to_char(value)?);
                i += 1 + count;
            }
            'x' => {
                let value = read_hex(&chars, i + 2, 2, 'x')?;
                out.push(codepoint_to_char(value)?);
                i += 2 + 2;
            }
            'u' => {
                let value = read_hex(&chars, i + 2, 4, 'u')?;
                out.push(codepoint_to_char(value)?);
                i += 2 + 4;
            }
            'U' => {
                let value = read_hex(&chars, i + 2, 8, 'U')?;
                out.push(codepoint_to_char(value)?);
                i += 2 + 8;
            }
            'N' => return Err(EvalStringError::UnsupportedNamedEscape),
            other => {
                out.push('\\');
                out.push(other);
                i += 2;
            }
        }
    }
    Ok(out)
}

fn decode_bytes_escapes(inner: &str) -> Result<Vec<u8>, EvalStringError> {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            if !c.is_ascii() {
                return Err(EvalStringError::NonAsciiByte(c));
            }
            out.push(c as u8);
            i += 1;
            continue;
        }
        let esc = *chars.get(i + 1).ok_or(EvalStringError::Unterminated)?;
        match esc {
            '\n' => i += 2,
            '\\' => {
                out.push(b'\\');
                i += 2;
            }
            '\'' => {
                out.push(b'\'');
                i += 2;
            }
            '"' => {
                out.push(b'"');
                i += 2;
            }
            'a' => {
                out.push(0x07);
                i += 2;
            }
            'b' => {
                out.push(0x08);
                i += 2;
            }
            'f' => {
                out.push(0x0C);
                i += 2;
            }
            'n' => {
                out.push(b'\n');
                i += 2;
            }
            'r' => {
                out.push(b'\r');
                i += 2;
            }
            't' => {
                out.push(b'\t');
                i += 2;
            }
            'v' => {
                out.push(0x0B);
                i += 2;
            }
            // Bytes literals cap octal escapes to a single byte; Python
            // truncates the same way rather than erroring on overflow.
            '0'..='7' => {
                let (value, count) = read_octal(&chars, i + 1);
                out.push((value & 0xFF) as u8);
                i += 1 + count;
            }
            'x' => {
                let value = read_hex(&chars, i + 2, 2, 'x')?;
                out.push(value as u8);
                i += 2 + 2;
            }
            // `\u`, `\U`, `\N` are not special in a bytes literal.
            other => {
                out.push(b'\\');
                if !other.is_ascii() {
                    return Err(EvalStringError::NonAsciiByte(other));
                }
                out.push(other as u8);
                i += 2;
            }
        }
    }
    Ok(out)
}

fn read_octal(chars: &[char], start: usize) -> (u32, usize) {
    let mut value = 0u32;
    let mut count = 0usize;
    while count < 3 {
        match chars.get(start + count) {
            Some(c) if ('0'..='7').contains(c) => {
                value = value * 8 + (*c as u32 - '0' as u32);
                count += 1;
            }
            _ => break,
        }
    }
    (value, count)
}

fn read_hex(chars: &[char], start: usize, count: usize, esc: char) -> Result<u32, EvalStringError> {
    if start + count > chars.len() {
        return Err(EvalStringError::TruncatedEscape(esc));
    }
    let mut value = 0u32;
    for &c in &chars[start..start + count] {
        let digit = c.to_digit(16).ok_or(EvalStringError::InvalidHexDigit(esc))?;
        value = value * 16 + digit;
    }
    Ok(value)
}

fn codepoint_to_char(value: u32) -> Result<char, EvalStringError> {
    char::from_u32(value).ok_or(EvalStringError::InvalidCodePoint(value))
}

/// A token, for use with `TokenGrammar`s.
///
/// You will likely want to hold additional information alongside this, like
/// the characters that were lexed to create it. The only contract is that a
/// token has a `type`.
#[derive(Clone, PartialEq, Eq)]
pub struct Token {
    pub r#type: String,
}

impl Token {
    pub fn new(r#type: impl Into<String>) -> Self {
        Token { r#type: r#type.into() }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<Token \"{}\">", self.r#type)
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_string() {
        assert_eq!(
            evaluate_string(r#""foo""#),
            Ok(StringLiteral::Str("foo".to_string()))
        );
    }

    #[test]
    fn common_escapes() {
        assert_eq!(
            evaluate_string(r#""foo\n\t\\""#),
            Ok(StringLiteral::Str("foo\n\t\\".to_string()))
        );
    }

    #[test]
    fn unrecognized_escape_is_kept_literally() {
        assert_eq!(
            evaluate_string(r#""a\pb""#),
            Ok(StringLiteral::Str("a\\pb".to_string()))
        );
    }

    #[test]
    fn hex_and_unicode_escapes() {
        assert_eq!(
            evaluate_string(r#""\x41B\U00000043""#),
            Ok(StringLiteral::Str("ABC".to_string()))
        );
    }

    #[test]
    fn octal_escape() {
        assert_eq!(
            evaluate_string(r#""\101""#),
            Ok(StringLiteral::Str("A".to_string()))
        );
    }

    #[test]
    fn bytes_prefix() {
        assert_eq!(
            evaluate_string(r#"b"foo\x00""#),
            Ok(StringLiteral::Bytes(vec![b'f', b'o', b'o', 0]))
        );
    }

    #[test]
    fn bytes_reject_non_ascii() {
        assert_eq!(
            evaluate_string("b\"café\""),
            Err(EvalStringError::NonAsciiByte('é'))
        );
    }

    #[test]
    fn raw_string_keeps_backslashes() {
        assert_eq!(
            evaluate_string(r#"r"foo\n""#),
            Ok(StringLiteral::Str("foo\\n".to_string()))
        );
    }

    #[test]
    fn raw_bytes_prefix_either_order() {
        assert_eq!(evaluate_string(r#"rb"a\b""#), evaluate_string(r#"br"a\b""#));
    }

    #[test]
    fn u_prefix_cannot_combine() {
        assert_eq!(
            evaluate_string(r#"ur"foo""#),
            Err(EvalStringError::UnknownPrefix("ur".to_string()))
        );
    }

    #[test]
    fn unterminated_literal_errors() {
        assert_eq!(evaluate_string(r#""foo"#), Err(EvalStringError::Unterminated));
    }

    #[test]
    fn truncated_hex_escape_errors() {
        assert_eq!(
            evaluate_string(r#""\x4""#),
            Err(EvalStringError::TruncatedEscape('x'))
        );
    }

    #[test]
    fn token_display_and_repr_match() {
        let t = Token::new("a");
        assert_eq!(format!("{t}"), "<Token \"a\">");
        assert_eq!(format!("{t}"), format!("{t:?}"));
    }

    #[test]
    fn token_equality_is_by_type() {
        assert_eq!(Token::new("a"), Token::new("a"));
        assert_ne!(Token::new("a"), Token::new("b"));
    }
}
