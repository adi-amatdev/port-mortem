//! Error values produced while constructing and running parsimonious grammars.
//!
//! This is a safe, owned representation of `parsimonious/exceptions.py`.
//! Error values carry all formatting data themselves, so they are `Send` and
//! `Sync` without shared mutable state.

use std::error::Error;
use std::fmt;

/// A description of the expression that reported a parsing failure.
#[derive(Clone, PartialEq, Eq)]
pub struct ExpressionDescription {
    /// The expression's grammar-rule name, when it has one.
    pub name: Option<String>,
    /// The expression's rule-style representation.
    pub display: String,
}

impl ExpressionDescription {
    /// Creates an expression description used in parse diagnostics.
    pub fn new(name: Option<String>, display: impl Into<String>) -> Self {
        Self {
            name,
            display: display.into(),
        }
    }

    fn parse_rule_name(&self) -> String {
        match &self.name {
            Some(name) => format!("'{name}'"),
            None => self.display.clone(),
        }
    }

    fn left_recursion_rule_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.display.clone())
    }
}

impl fmt::Display for ExpressionDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display)
    }
}

impl fmt::Debug for ExpressionDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Input preserved in a parsing error.
#[derive(Clone, PartialEq, Eq)]
pub enum ParseInput {
    /// Character input, indexed by Unicode scalar value to match Python `str`.
    Text(String),
    /// Token input, indexed by token position as in `TokenGrammar`.
    Tokens(Vec<String>),
}

impl ParseInput {
    fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.chars().count(),
            Self::Tokens(tokens) => tokens.len(),
        }
    }

    fn normalized_stop(&self, index: isize) -> usize {
        normalize_index(index, self.len())
    }

    fn window(&self, pos: isize) -> String {
        let end = pos.saturating_add(20);
        match self {
            Self::Text(text) => slice_chars(text, pos, end),
            Self::Tokens(tokens) => {
                let start = normalize_index(pos, tokens.len());
                let end = normalize_index(end, tokens.len());
                if start >= end {
                    "[]".to_string()
                } else {
                    let values = tokens[start..end]
                        .iter()
                        .map(|token| python_repr(token))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("[{values}]")
                }
            }
        }
    }

    fn line(&self, pos: isize) -> Option<usize> {
        match self {
            Self::Text(text) => Some(
                text.chars()
                    .take(self.normalized_stop(pos))
                    .filter(|character| *character == '\n')
                    .count()
                    + 1,
            ),
            Self::Tokens(_) => None,
        }
    }

    fn column(&self, pos: isize) -> isize {
        match self {
            Self::Text(text) => {
                let before = self.normalized_stop(pos);
                let preceding_newline = text
                    .chars()
                    .take(before)
                    .enumerate()
                    .filter_map(|(index, character)| (character == '\n').then_some(index))
                    .last();
                preceding_newline
                    .map_or_else(|| pos.saturating_add(1), |index| pos - index as isize)
            }
            Self::Tokens(_) => pos.saturating_add(1),
        }
    }
}

/// A parse failed to match at the recorded position.
#[derive(Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The original parser input.
    pub text: ParseInput,
    /// The farthest input position reached by a failed expression.
    pub pos: isize,
    /// The expression responsible for the reported failure.
    pub expr: Option<ExpressionDescription>,
}

impl ParseError {
    /// Creates the initially blank, per-parse failure record used by `match`.
    pub fn new(text: ParseInput) -> Self {
        Self {
            text,
            pos: -1,
            expr: None,
        }
    }

    /// Creates a failure record for character input.
    pub fn for_text(text: impl Into<String>) -> Self {
        Self::new(ParseInput::Text(text.into()))
    }

    /// Creates a failure record for token input.
    pub fn for_tokens(tokens: Vec<String>) -> Self {
        Self::new(ParseInput::Tokens(tokens))
    }

    /// Records the expression and position selected by the parser's failure policy.
    pub fn record_failure(&mut self, pos: isize, expr: ExpressionDescription) {
        self.pos = pos;
        self.expr = Some(expr);
    }

    /// Returns the one-based line number, or `None` for token input.
    pub fn line(&self) -> Option<usize> {
        self.text.line(self.pos)
    }

    /// Returns the one-based column number at which matching stopped.
    pub fn column(&self) -> isize {
        self.text.column(self.pos)
    }

    fn rule_name(&self) -> String {
        self.expr
            .as_ref()
            .map(ExpressionDescription::parse_rule_name)
            .unwrap_or_else(|| "<unknown expression>".to_string())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rule {} didn't match at '{}' (line {}, column {}).",
            self.rule_name(),
            self.text.window(self.pos),
            display_line(self.line()),
            self.column()
        )
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ParseError {}

/// The parser encountered left recursion, which packrat parsing cannot handle.
#[derive(Clone, PartialEq, Eq)]
pub struct LeftRecursionError(pub ParseError);

impl LeftRecursionError {
    /// Creates a left-recursion error at the expression currently being matched.
    pub fn new(text: ParseInput, expr: ExpressionDescription) -> Self {
        Self(ParseError {
            text,
            pos: -1,
            expr: Some(expr),
        })
    }

    /// Returns the underlying parse failure metadata.
    pub fn parse_error(&self) -> &ParseError {
        &self.0
    }
}

impl fmt::Display for LeftRecursionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rule_name = self
            .0
            .expr
            .as_ref()
            .map(ExpressionDescription::left_recursion_rule_name)
            .unwrap_or_else(|| "<unknown expression>".to_string());
        write!(
            f,
            "Left recursion in rule {} at {} (line {}, column {}).\n\n\
             Parsimonious is a packrat parser, so it can't handle left recursion.\n\
             See https://en.wikipedia.org/wiki/Parsing_expression_grammar#Indirect_left_recursion\n\
             for how to rewrite your grammar into a rule that does not use left-recursion.",
            python_repr(&rule_name),
            python_repr(&self.0.text.window(self.0.pos)),
            display_line(self.0.line()),
            self.0.column()
        )
    }
}

impl fmt::Debug for LeftRecursionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for LeftRecursionError {}

/// A parse matched, but did not consume all input.
#[derive(Clone, PartialEq, Eq)]
pub struct IncompleteParseError(pub ParseError);

impl IncompleteParseError {
    /// Creates an incomplete-parse error at the first unmatched position.
    pub fn new(text: ParseInput, pos: isize, expr: ExpressionDescription) -> Self {
        Self(ParseError {
            text,
            pos,
            expr: Some(expr),
        })
    }

    /// Returns the underlying parse failure metadata.
    pub fn parse_error(&self) -> &ParseError {
        &self.0
    }
}

impl fmt::Display for IncompleteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rule_name = self
            .0
            .expr
            .as_ref()
            .and_then(|expr| expr.name.as_deref())
            .unwrap_or("<unknown expression>");
        write!(
            f,
            "Rule '{rule_name}' matched in its entirety, but it didn't consume all the text. \
             The non-matching portion of the text begins with '{}' (line {}, column {}).",
            self.0.text.window(self.0.pos),
            display_line(self.0.line()),
            self.0.column()
        )
    }
}

impl fmt::Debug for IncompleteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for IncompleteParseError {}

/// A visitor error augmented with its source class and parse-tree rendering.
#[derive(Clone, PartialEq, Eq)]
pub struct VisitationError {
    /// The source error's class name.
    pub original_class: String,
    /// The source error's display message.
    pub original_error: String,
    /// The pretty rendering of the node where visiting failed.
    pub parse_tree: String,
}

impl VisitationError {
    /// Wraps a visitor failure with the parse-tree context produced by the caller.
    pub fn new(
        original_class: impl Into<String>,
        original_error: impl Into<String>,
        parse_tree: impl Into<String>,
    ) -> Self {
        Self {
            original_class: original_class.into(),
            original_error: original_error.into(),
            parse_tree: parse_tree.into(),
        }
    }
}

impl fmt::Display for VisitationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}\n\nParse tree:\n{}",
            self.original_class, self.original_error, self.parse_tree
        )
    }
}

impl fmt::Debug for VisitationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for VisitationError {}

/// A grammar definition was invalid.
#[derive(Clone, PartialEq, Eq)]
pub struct BadGrammar {
    /// The grammar error explanation.
    pub message: String,
}

impl BadGrammar {
    /// Creates a grammar-definition error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BadGrammar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Debug for BadGrammar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for BadGrammar {}

/// A grammar rule label was referenced but never defined.
#[derive(Clone, PartialEq, Eq)]
pub struct UndefinedLabel {
    /// The missing grammar-rule label.
    pub label: String,
}

impl UndefinedLabel {
    /// Creates an undefined-label error.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl fmt::Display for UndefinedLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The label \"{}\" was never defined.", self.label)
    }
}

impl fmt::Debug for UndefinedLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for UndefinedLabel {}

/// Every error emitted by this crate's parser and grammar builder.
#[derive(Clone, PartialEq, Eq)]
pub enum ParsimoniousError {
    /// A parser expression did not match.
    Parse(ParseError),
    /// A packrat parse encountered left recursion.
    LeftRecursion(LeftRecursionError),
    /// A parse left unmatched input.
    IncompleteParse(IncompleteParseError),
    /// A visitor failed while walking a parse tree.
    Visitation(VisitationError),
    /// A grammar definition was invalid.
    BadGrammar(BadGrammar),
    /// A grammar referenced an unknown label.
    UndefinedLabel(UndefinedLabel),
}

impl fmt::Display for ParsimoniousError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::LeftRecursion(error) => error.fmt(f),
            Self::IncompleteParse(error) => error.fmt(f),
            Self::Visitation(error) => error.fmt(f),
            Self::BadGrammar(error) => error.fmt(f),
            Self::UndefinedLabel(error) => error.fmt(f),
        }
    }
}

impl fmt::Debug for ParsimoniousError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ParsimoniousError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::LeftRecursion(error) => Some(error),
            Self::IncompleteParse(error) => Some(error),
            Self::Visitation(error) => Some(error),
            Self::BadGrammar(error) => Some(error),
            Self::UndefinedLabel(error) => Some(error),
        }
    }
}

macro_rules! impl_from_error {
    ($type:ty, $variant:ident) => {
        impl From<$type> for ParsimoniousError {
            fn from(error: $type) -> Self {
                Self::$variant(error)
            }
        }
    };
}

impl_from_error!(ParseError, Parse);
impl_from_error!(LeftRecursionError, LeftRecursion);
impl_from_error!(IncompleteParseError, IncompleteParse);
impl_from_error!(VisitationError, Visitation);
impl_from_error!(BadGrammar, BadGrammar);
impl_from_error!(UndefinedLabel, UndefinedLabel);

fn normalize_index(index: isize, len: usize) -> usize {
    if index < 0 {
        len.saturating_sub(index.unsigned_abs())
    } else {
        usize::try_from(index)
            .ok()
            .map_or(len, |value| value.min(len))
    }
}

fn slice_chars(text: &str, start: isize, end: isize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let start = normalize_index(start, chars.len());
    let end = normalize_index(end, chars.len());
    if start >= end {
        String::new()
    } else {
        chars[start..end].iter().collect()
    }
}

fn python_repr(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('\'');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                use fmt::Write;
                let _ = write!(output, "\\x{:02x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('\'');
    output
}

fn display_line(line: Option<usize>) -> String {
    line.map_or_else(|| "None".to_string(), |line| line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named_rule() -> ExpressionDescription {
        ExpressionDescription::new(Some("close_parens".to_string()), "<Literal \")\")>")
    }

    #[test]
    fn parse_error_matches_source_formatting() {
        let mut error = ParseError::for_text("((fred!!");
        error.record_failure(6, named_rule());

        assert_eq!(error.line(), Some(1));
        assert_eq!(error.column(), 7);
        assert_eq!(
            error.to_string(),
            "Rule 'close_parens' didn't match at '!!' (line 1, column 7)."
        );
        assert_eq!(format!("{error:?}"), error.to_string());
    }

    #[test]
    fn parse_error_uses_unicode_scalar_positions() {
        let mut error = ParseError::for_text("a\n中文!");
        error.record_failure(4, named_rule());

        assert_eq!(error.line(), Some(2));
        assert_eq!(error.column(), 3);
        assert!(error.to_string().contains("'!'"));
    }

    #[test]
    fn token_input_has_no_line_number() {
        let mut error = ParseError::for_tokens(vec!["a".to_string(), "b".to_string()]);
        error.record_failure(1, named_rule());

        assert_eq!(error.line(), None);
        assert_eq!(error.column(), 2);
        assert!(error.to_string().contains("['b']"));
    }

    #[test]
    fn incomplete_parse_error_matches_source_formatting() {
        let error = IncompleteParseError::new(
            ParseInput::Text("chitty bangbang".to_string()),
            11,
            ExpressionDescription::new(Some("sequence".to_string()), "sequence"),
        );

        assert_eq!(
            error.to_string(),
            "Rule 'sequence' matched in its entirety, but it didn't consume all the text. The non-matching portion of the text begins with 'bang' (line 1, column 12)."
        );
    }

    #[test]
    fn left_recursion_error_explains_packrat_limitation() {
        let error = LeftRecursionError::new(ParseInput::Text("abc".to_string()), named_rule());

        assert!(error
            .to_string()
            .contains("Parsimonious is a packrat parser, so it can't handle left recursion."));
    }

    #[test]
    fn visitation_error_carries_context() {
        let error = VisitationError::new("ValueError", "bad visitor", "root\n  child");
        assert_eq!(
            error.to_string(),
            "ValueError: bad visitor\n\nParse tree:\nroot\n  child"
        );
    }

    #[test]
    fn undefined_label_matches_source_formatting() {
        let error = UndefinedLabel::new("missing");
        assert_eq!(
            error.to_string(),
            "The label \"missing\" was never defined."
        );
    }

    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ParsimoniousError>();
    }
}
