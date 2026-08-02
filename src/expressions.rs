//! PEG expression combinators and a parse-local packrat evaluator.

use crate::exceptions::{
    ExpressionDescription, IncompleteParseError, LeftRecursionError, ParseError, ParseInput,
    ParsimoniousError,
};
use crate::grammar::Grammar;
use crate::nodes::Node;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// An immutable PEG expression. Immutable expression graphs and parse-local
/// caches make a [`Grammar`] safe to use concurrently without locking.
#[derive(Clone)]
pub struct Expression {
    pub(crate) id: usize,
    pub name: String,
    pub(crate) kind: ExpressionKind,
    pub(crate) custom: Option<Arc<dyn CustomRule>>,
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Expression")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name && self.kind == other.kind
    }
}

impl Eq for Expression {}

/// A statically typed custom parser rule.
///
/// Python accepts arbitrary callables and inspects their signature at runtime.
/// Rust makes that contract explicit: custom rules receive the input, position,
/// and containing grammar, and either produce a node, decline the match, or
/// return a parser error.
pub trait CustomRule: Send + Sync {
    /// Attempt a match starting at `pos`.
    fn match_rule(
        &self,
        text: &str,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Option<Node>, ParsimoniousError>;
}

impl<F> CustomRule for F
where
    F: Fn(&str, usize, &Grammar) -> Result<Option<Node>, ParsimoniousError> + Send + Sync,
{
    fn match_rule(
        &self,
        text: &str,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Option<Node>, ParsimoniousError> {
        self(text, pos, grammar)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExpressionKind {
    Literal(String),
    Regex(String),
    Sequence(Vec<Arc<Expression>>),
    OneOf(Vec<Arc<Expression>>),
    Lookahead {
        member: Arc<Expression>,
        negative: bool,
    },
    Quantifier {
        member: Arc<Expression>,
        min: usize,
        max: Option<usize>,
    },
    Custom(String),
    Ref(String),
}

static NEXT_EXPRESSION_ID: AtomicUsize = AtomicUsize::new(1_000_000);

fn generated_id() -> usize {
    NEXT_EXPRESSION_ID.fetch_add(1, Ordering::Relaxed)
}

impl Expression {
    pub(crate) fn new(id: usize, name: impl Into<String>, kind: ExpressionKind) -> Arc<Self> {
        Arc::new(Self {
            id,
            name: name.into(),
            kind,
            custom: None,
        })
    }
    /// Match all input from `pos`.
    pub fn parse(
        self: &Arc<Self>,
        text: &str,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Node, ParsimoniousError> {
        let node = self.match_text(text, pos, grammar)?;
        let len = text.chars().count();
        if node.end < len {
            return Err(IncompleteParseError::new(
                ParseInput::Text(text.to_owned()),
                node.end as isize,
                self.description(),
            )
            .into());
        }
        Ok(node)
    }
    /// Match a prefix of input from `pos`.
    pub fn match_text(
        self: &Arc<Self>,
        text: &str,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Node, ParsimoniousError> {
        let mut state = ParseState::new(text);
        match state.match_expr(self, pos, grammar)? {
            Some(node) => Ok(node),
            None => Err(ParsimoniousError::Parse(state.error)),
        }
    }
    pub(crate) fn description(&self) -> ExpressionDescription {
        ExpressionDescription::new(
            (!self.name.is_empty()).then(|| self.name.clone()),
            self.as_rule(),
        )
    }
    /// A grammar-style rendering of the expression.
    pub fn as_rule(&self) -> String {
        let rhs = self.rhs();
        if self.name.is_empty() {
            rhs
        } else {
            format!("{} = {}", self.name, rhs)
        }
    }
    fn rhs(&self) -> String {
        match &self.kind {
            ExpressionKind::Literal(value) => format!("{:?}", value),
            ExpressionKind::Regex(value) => format!("~{:?}", value),
            ExpressionKind::Custom(label) => label.clone(),
            ExpressionKind::Ref(name) => name.clone(),
            ExpressionKind::Sequence(xs) => format!(
                "({})",
                xs.iter()
                    .map(|x| if x.name.is_empty() {
                        x.rhs()
                    } else {
                        x.name.clone()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            ExpressionKind::OneOf(xs) => format!(
                "({})",
                xs.iter()
                    .map(|x| if x.name.is_empty() {
                        x.rhs()
                    } else {
                        x.name.clone()
                    })
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
            ExpressionKind::Lookahead { member, negative } => {
                format!("{}{}", if *negative { "!" } else { "&" }, member.rhs())
            }
            ExpressionKind::Quantifier { member, min, max } => {
                let q = match (*min, *max) {
                    (0, Some(1)) => "?".to_owned(),
                    (0, None) => "*".to_owned(),
                    (1, None) => "+".to_owned(),
                    (n, None) => format!("{{{n},}}"),
                    (0, Some(n)) => format!("{{,{n}}}"),
                    (n, Some(m)) => format!("{{{n},{m}}}"),
                };
                format!("{}{}", member.rhs(), q)
            }
        }
    }
}

enum Memo {
    InProgress,
    Done(Option<Node>),
}
struct ParseState<'a> {
    text: &'a str,
    chars: Vec<char>,
    cache: HashMap<(usize, usize), Memo>,
    error: ParseError,
}
impl<'a> ParseState<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            chars: text.chars().collect(),
            cache: HashMap::new(),
            error: ParseError::for_text(text),
        }
    }
    fn match_expr(
        &mut self,
        expr: &Arc<Expression>,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Option<Node>, ParsimoniousError> {
        let key = (expr.id, pos);
        if let Some(value) = self.cache.get(&key) {
            return match value {
                Memo::Done(node) => Ok(node.clone()),
                Memo::InProgress => Err(LeftRecursionError::new(
                    ParseInput::Text(self.text.to_owned()),
                    expr.description(),
                )
                .into()),
            };
        }
        self.cache.insert(key, Memo::InProgress);
        let node = self.uncached(expr, pos, grammar)?;
        self.cache.insert(key, Memo::Done(node.clone()));
        if node.is_none()
            && (pos as isize >= self.error.pos)
            && (!expr.name.is_empty()
                || self
                    .error
                    .expr
                    .as_ref()
                    .and_then(|x| x.name.as_ref())
                    .is_none())
        {
            self.error.record_failure(pos as isize, expr.description());
        }
        Ok(node)
    }
    fn uncached(
        &mut self,
        expr: &Arc<Expression>,
        pos: usize,
        grammar: &Grammar,
    ) -> Result<Option<Node>, ParsimoniousError> {
        match &expr.kind {
            ExpressionKind::Literal(value) => {
                let needle: Vec<char> = value.chars().collect();
                if self.chars.get(pos..pos + needle.len()) == Some(needle.as_slice()) {
                    Ok(Some(Node::new(
                        expr.name.clone(),
                        self.text,
                        pos,
                        pos + needle.len(),
                        vec![],
                    )))
                } else {
                    Ok(None)
                }
            }
            ExpressionKind::Regex(pattern) => {
                let end = regex_prefix(pattern, &self.chars, pos);
                Ok(end.map(|end| Node::new(expr.name.clone(), self.text, pos, end, vec![])))
            }
            ExpressionKind::Custom(_) => match &expr.custom {
                Some(matcher) => matcher.match_rule(self.text, pos, grammar),
                None => Ok(None),
            },
            ExpressionKind::Ref(name) => match grammar.rule(name) {
                Some(target) => self.match_expr(target, pos, grammar),
                None => Ok(None),
            },
            ExpressionKind::Sequence(members) => {
                let mut at = pos;
                let mut nodes = Vec::with_capacity(members.len());
                for member in members {
                    let Some(node) = self.match_expr(member, at, grammar)? else {
                        return Ok(None);
                    };
                    at = node.end;
                    nodes.push(node);
                }
                Ok(Some(Node::new(
                    expr.name.clone(),
                    self.text,
                    pos,
                    at,
                    nodes,
                )))
            }
            ExpressionKind::OneOf(members) => {
                for member in members {
                    if let Some(node) = self.match_expr(member, pos, grammar)? {
                        return Ok(Some(Node::new(
                            expr.name.clone(),
                            self.text,
                            pos,
                            node.end,
                            vec![node],
                        )));
                    }
                }
                Ok(None)
            }
            ExpressionKind::Lookahead { member, negative } => {
                let found = self.match_expr(member, pos, grammar)?.is_some();
                if found != *negative {
                    Ok(Some(Node::new(
                        expr.name.clone(),
                        self.text,
                        pos,
                        pos,
                        vec![],
                    )))
                } else {
                    Ok(None)
                }
            }
            ExpressionKind::Quantifier { member, min, max } => {
                let mut at = pos;
                let mut nodes = Vec::new();
                while at < self.chars.len() && max.is_none_or(|limit| nodes.len() < limit) {
                    let Some(node) = self.match_expr(member, at, grammar)? else {
                        break;
                    };
                    let empty = node.end == at;
                    nodes.push(node);
                    if empty && nodes.len() >= *min {
                        break;
                    }
                    at = nodes.last().map_or(at, |n| n.end);
                }
                if nodes.len() >= *min {
                    Ok(Some(Node::new(
                        expr.name.clone(),
                        self.text,
                        pos,
                        at,
                        nodes,
                    )))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

// Deliberately small regex engine for the grammar language's anchored patterns.
// It accepts literals, '.', \d/\s/\w and their uppercase negations, character
// classes, and the usual single-atom quantifiers; unsupported syntax simply
// does not match instead of panicking.
fn regex_prefix(pattern: &str, chars: &[char], pos: usize) -> Option<usize> {
    let p: Vec<char> = pattern.chars().collect();
    let mut pi = 0;
    let mut at = pos;
    while pi < p.len() {
        let (pred, consumed) = atom(&p[pi..])?;
        pi += consumed;
        let (min, max, qlen) = quantifier(&p[pi..]);
        pi += qlen;
        let mut count = 0;
        while max.is_none_or(|m| count < m) && at < chars.len() && pred(chars[at]) {
            at += 1;
            count += 1;
        }
        if count < min {
            return None;
        }
    }
    Some(at)
}
fn atom(p: &[char]) -> Option<(Box<dyn Fn(char) -> bool>, usize)> {
    let first = *p.first()?;
    match first {
        '.' => Some((Box::new(|_| true), 1)),
        '\\' => {
            let next = *p.get(1)?;
            let f: Box<dyn Fn(char) -> bool> = match next {
                'd' => Box::new(|c| c.is_ascii_digit()),
                'D' => Box::new(|c| !c.is_ascii_digit()),
                's' => Box::new(|c| c.is_whitespace()),
                'S' => Box::new(|c| !c.is_whitespace()),
                'w' => Box::new(|c| c.is_ascii_alphanumeric() || c == '_'),
                'W' => Box::new(|c| !(c.is_ascii_alphanumeric() || c == '_')),
                other => Box::new(move |c| c == other),
            };
            Some((f, 2))
        }
        '[' => {
            let end = p.iter().position(|c| *c == ']')?;
            let body = &p[1..end];
            let negated = body.first() == Some(&'^');
            let body = if negated { &body[1..] } else { body };
            let mut ranges = Vec::new();
            let mut i = 0;
            while i < body.len() {
                if i + 2 < body.len() && body[i + 1] == '-' {
                    ranges.push((body[i], body[i + 2]));
                    i += 3;
                } else {
                    ranges.push((body[i], body[i]));
                    i += 1;
                }
            }
            Some((
                Box::new(move |c| ranges.iter().any(|(a, b)| *a <= c && c <= *b) != negated),
                end + 1,
            ))
        }
        '(' | '|' | '^' | '$' => None,
        other => Some((Box::new(move |c| c == other), 1)),
    }
}
fn quantifier(p: &[char]) -> (usize, Option<usize>, usize) {
    match p.first() {
        Some('*') => (0, None, 1),
        Some('+') => (1, None, 1),
        Some('?') => (0, Some(1), 1),
        Some('{') => {
            let end = p.iter().position(|c| *c == '}');
            let Some(end) = end else {
                return (1, Some(1), 0);
            };
            let body: String = p[1..end].iter().collect();
            let parts: Vec<_> = body.split(',').collect();
            let min = parts.first().and_then(|x| x.parse().ok()).unwrap_or(0);
            let max = if parts.len() == 1 {
                Some(min)
            } else {
                parts
                    .get(1)
                    .and_then(|x| (!x.is_empty()).then(|| x.parse().ok()).flatten())
            };
            (min, max, end + 1)
        }
        _ => (1, Some(1), 0),
    }
}

fn named(kind: ExpressionKind, name: impl Into<String>) -> Arc<Expression> {
    Expression::new(generated_id(), name, kind)
}

/// Construct a literal expression.
pub fn literal(value: impl Into<String>) -> Arc<Expression> {
    named(ExpressionKind::Literal(value.into()), "")
}

/// Construct negative lookahead. It succeeds without consuming input only
/// when `member` does not match at the current position.
pub fn not(member: Arc<Expression>) -> Arc<Expression> {
    named(
        ExpressionKind::Lookahead {
            member,
            negative: true,
        },
        "",
    )
}

/// Construct a named negative-lookahead expression.
pub fn not_named(member: Arc<Expression>, name: impl Into<String>) -> Arc<Expression> {
    named(
        ExpressionKind::Lookahead {
            member,
            negative: true,
        },
        name,
    )
}

/// Construct a zero-or-more repetition expression.
pub fn zero_or_more(member: Arc<Expression>) -> Arc<Expression> {
    zero_or_more_named(member, "")
}

/// Construct a named zero-or-more repetition expression.
pub fn zero_or_more_named(member: Arc<Expression>, name: impl Into<String>) -> Arc<Expression> {
    named(
        ExpressionKind::Quantifier {
            member,
            min: 0,
            max: None,
        },
        name,
    )
}

/// Construct a one-or-more repetition expression.
pub fn one_or_more(member: Arc<Expression>) -> Arc<Expression> {
    one_or_more_min_named(member, 1, "")
}

/// Construct a one-or-more expression with the Python API's configurable
/// minimum count.
pub fn one_or_more_min(member: Arc<Expression>, min: usize) -> Arc<Expression> {
    one_or_more_min_named(member, min, "")
}

/// Construct a named one-or-more expression with a configurable minimum.
pub fn one_or_more_min_named(
    member: Arc<Expression>,
    min: usize,
    name: impl Into<String>,
) -> Arc<Expression> {
    named(
        ExpressionKind::Quantifier {
            member,
            min,
            max: None,
        },
        name,
    )
}

/// Construct an optional expression.
pub fn optional(member: Arc<Expression>) -> Arc<Expression> {
    optional_named(member, "")
}

/// Construct a named optional expression.
pub fn optional_named(member: Arc<Expression>, name: impl Into<String>) -> Arc<Expression> {
    named(
        ExpressionKind::Quantifier {
            member,
            min: 0,
            max: Some(1),
        },
        name,
    )
}

/// Rust's compile-time equivalent of Python's runtime `is_callable` check.
/// Non-rule values are rejected by the type system rather than returning false
/// later while a grammar is being assembled.
pub fn is_callable<R: CustomRule + ?Sized>(_: &R) -> bool {
    true
}

/// Convert a typed custom rule into an expression that participates in the
/// normal packrat cache and error propagation path.
pub fn expression<R>(rule: R, rule_name: impl Into<String>) -> Arc<Expression>
where
    R: CustomRule + 'static,
{
    let name = rule_name.into();
    Arc::new(Expression {
        id: generated_id(),
        kind: ExpressionKind::Custom("{custom rule}".to_string()),
        name,
        custom: Some(Arc::new(rule)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grammar() -> Grammar {
        match Grammar::new("fallback = 'x'") {
            Ok(grammar) => grammar,
            Err(error) => panic!("test grammar must compile: {error}"),
        }
    }

    #[test]
    fn negative_lookahead_does_not_consume_and_rejects_a_match() {
        let grammar = grammar();
        let expression = not(literal("a"));
        let node = expression.match_text("", 0, &grammar);
        assert_eq!(node.map(|node| (node.start, node.end)), Ok((0, 0)));
        assert!(expression.match_text("a", 0, &grammar).is_err());
    }

    #[test]
    fn repetitions_and_optional_preserve_tree_shape_and_offsets() {
        let grammar = grammar();
        let many = zero_or_more_named(literal("a"), "many");
        let repeated = many.match_text("aaa", 0, &grammar);
        assert_eq!(
            repeated.map(|node| (node.expr_name, node.start, node.end, node.children.len())),
            Ok(("many".to_string(), 0, 3, 3))
        );

        let maybe = optional_named(literal("a"), "maybe");
        let absent = maybe.match_text("b", 0, &grammar);
        assert_eq!(
            absent.map(|node| (node.expr_name, node.start, node.end, node.children.len())),
            Ok(("maybe".to_string(), 0, 0, 0))
        );

        let at_least_two = one_or_more_min(literal("a"), 2);
        assert!(at_least_two.match_text("a", 0, &grammar).is_err());
    }

    #[test]
    fn custom_rules_are_typed_callable_expressions() {
        let grammar = grammar();
        let matcher = |text: &str, pos: usize, _: &Grammar| {
            if text.chars().nth(pos) == Some('!') {
                Ok(Some(Node::new("bang", text, pos, pos + 1, vec![])))
            } else {
                Ok(None)
            }
        };
        assert!(is_callable(&matcher));
        let expression = expression(matcher, "bang");
        assert_eq!(
            expression
                .match_text("!", 0, &grammar)
                .map(|node| node.text()),
            Ok("!".to_string())
        );
    }
}
