//! Grammar-definition parser and immutable grammar container.

use crate::exceptions::{BadGrammar, ParsimoniousError, UndefinedLabel};
use crate::expressions::{Expression, ExpressionKind};
use crate::nodes::Node;
use std::collections::BTreeMap;
use std::sync::Arc;

/// An immutable collection of named PEG rules.
#[derive(Clone, Debug)]
pub struct Grammar {
    rules: BTreeMap<String, Arc<Expression>>,
    default_rule: Option<String>,
}

/// Rust's text-grammar facade for the Python `TokenGrammar` name.
///
/// The current port represents parser input as text, so token grammars share
/// the same immutable grammar definition type. Token-stream matching remains
/// a documented structural divergence until token matchers are ported.
pub type TokenGrammar = Grammar;
impl Grammar {
    /// Compile a Parsimonious grammar definition.
    pub fn new(source: &str) -> Result<Self, ParsimoniousError> {
        let mut parser = DefinitionParser::new(source)?;
        let raw = parser.rules()?;
        let mut rules = BTreeMap::new();
        for (name, ast) in raw {
            let expr = compile(ast, &mut parser.next_id);
            rules.insert(
                name.clone(),
                Arc::new(Expression {
                    id: parser.next_id(),
                    name,
                    kind: expr.kind,
                    custom: None,
                }),
            );
        }
        for expr in rules.values() {
            validate_refs(expr, &rules)?;
        }
        let default_rule = rules.keys().next().cloned();
        Ok(Self {
            rules,
            default_rule,
        })
    }
    /// Return a grammar with a different default rule.
    pub fn default(mut self, name: &str) -> Result<Self, ParsimoniousError> {
        if self.rules.contains_key(name) {
            self.default_rule = Some(name.to_owned());
            Ok(self)
        } else {
            Err(UndefinedLabel::new(name).into())
        }
    }
    /// Fetch a named rule.
    pub fn rule(&self, name: &str) -> Option<&Arc<Expression>> {
        self.rules.get(name)
    }
    /// Parse all text with the default rule.
    pub fn parse(&self, text: &str, pos: usize) -> Result<Node, ParsimoniousError> {
        let rule = self.default_expression()?;
        rule.parse(text, pos, self)
    }
    /// Match a prefix with the default rule.
    pub fn match_text(&self, text: &str, pos: usize) -> Result<Node, ParsimoniousError> {
        let rule = self.default_expression()?;
        rule.match_text(text, pos, self)
    }
    fn default_expression(&self) -> Result<&Arc<Expression>, ParsimoniousError> {
        self.default_rule
            .as_deref()
            .and_then(|name| self.rule(name))
            .ok_or_else(|| {
                BadGrammar::new("can't parse with a grammar that has no default rule").into()
            })
    }
}

#[derive(Clone)]
struct Ast {
    kind: ExpressionKind,
}
fn compile(ast: Ast, next: &mut usize) -> Ast {
    fn go(ast: Ast, next: &mut usize) -> Arc<Expression> {
        let kind = match ast.kind {
            ExpressionKind::Sequence(xs) => ExpressionKind::Sequence(
                xs.into_iter()
                    .map(|x| {
                        go(
                            Ast {
                                kind: x.kind.clone(),
                            },
                            next,
                        )
                    })
                    .collect(),
            ),
            ExpressionKind::OneOf(xs) => ExpressionKind::OneOf(
                xs.into_iter()
                    .map(|x| {
                        go(
                            Ast {
                                kind: x.kind.clone(),
                            },
                            next,
                        )
                    })
                    .collect(),
            ),
            ExpressionKind::Lookahead { member, negative } => ExpressionKind::Lookahead {
                member: go(
                    Ast {
                        kind: member.kind.clone(),
                    },
                    next,
                ),
                negative,
            },
            ExpressionKind::Quantifier { member, min, max } => ExpressionKind::Quantifier {
                member: go(
                    Ast {
                        kind: member.kind.clone(),
                    },
                    next,
                ),
                min,
                max,
            },
            other => other,
        };
        let id = *next;
        *next += 1;
        Arc::new(Expression {
            id,
            name: String::new(),
            kind,
            custom: None,
        })
    }
    let arc = go(ast, next);
    Ast {
        kind: arc.kind.clone(),
    }
}
fn validate_refs(
    expr: &Expression,
    rules: &BTreeMap<String, Arc<Expression>>,
) -> Result<(), ParsimoniousError> {
    match &expr.kind {
        ExpressionKind::Ref(name) => {
            if rules.contains_key(name) {
                Ok(())
            } else {
                Err(UndefinedLabel::new(name).into())
            }
        }
        ExpressionKind::Sequence(xs) | ExpressionKind::OneOf(xs) => {
            for member in xs {
                validate_refs(member, rules)?;
            }
            Ok(())
        }
        ExpressionKind::Lookahead { member, .. } | ExpressionKind::Quantifier { member, .. } => {
            validate_refs(member, rules)
        }
        ExpressionKind::Literal(_) | ExpressionKind::Regex(_) | ExpressionKind::Custom(_) => Ok(()),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Ident(String),
    Text(String),
    Regex(String),
    Sym(char),
}
struct DefinitionParser {
    tokens: Vec<Token>,
    at: usize,
    next_id: usize,
}
impl DefinitionParser {
    fn new(source: &str) -> Result<Self, ParsimoniousError> {
        Ok(Self {
            tokens: lex(source)?,
            at: 0,
            next_id: 1,
        })
    }
    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    fn rules(&mut self) -> Result<Vec<(String, Ast)>, ParsimoniousError> {
        let mut out = Vec::new();
        while self.at < self.tokens.len() {
            let name = self.ident()?;
            self.sym('=')?;
            let rhs = self.expression()?;
            out.push((name, rhs));
        }
        Ok(out)
    }
    fn expression(&mut self) -> Result<Ast, ParsimoniousError> {
        let mut xs = vec![self.sequence()?];
        while self.peek_sym('/') {
            self.at += 1;
            xs.push(self.sequence()?);
        }
        if xs.len() == 1 {
            Ok(xs.remove(0))
        } else {
            Ok(Ast {
                kind: ExpressionKind::OneOf(
                    xs.into_iter()
                        .map(|x| {
                            Arc::new(Expression {
                                id: 0,
                                name: String::new(),
                                kind: x.kind,
                                custom: None,
                            })
                        })
                        .collect(),
                ),
            })
        }
    }
    fn sequence(&mut self) -> Result<Ast, ParsimoniousError> {
        let mut xs = Vec::new();
        while self.starts_atom() {
            xs.push(self.term()?);
        }
        if xs.is_empty() {
            return Err(BadGrammar::new("expected expression").into());
        }
        if xs.len() == 1 {
            Ok(xs.remove(0))
        } else {
            Ok(Ast {
                kind: ExpressionKind::Sequence(
                    xs.into_iter()
                        .map(|x| {
                            Arc::new(Expression {
                                id: 0,
                                name: String::new(),
                                kind: x.kind,
                                custom: None,
                            })
                        })
                        .collect(),
                ),
            })
        }
    }
    fn term(&mut self) -> Result<Ast, ParsimoniousError> {
        let look = if self.peek_sym('!') || self.peek_sym('&') {
            let negative = self.peek_sym('!');
            self.at += 1;
            Some(negative)
        } else {
            None
        };
        let mut atom = self.atom()?;
        if let Some(negative) = look {
            atom = Ast {
                kind: ExpressionKind::Lookahead {
                    member: Arc::new(Expression {
                        id: 0,
                        name: String::new(),
                        kind: atom.kind,
                        custom: None,
                    }),
                    negative,
                },
            };
        }
        if let Some((min, max)) = self.take_quantifier()? {
            atom = Ast {
                kind: ExpressionKind::Quantifier {
                    member: Arc::new(Expression {
                        id: 0,
                        name: String::new(),
                        kind: atom.kind,
                        custom: None,
                    }),
                    min,
                    max,
                },
            };
        }
        Ok(atom)
    }
    fn atom(&mut self) -> Result<Ast, ParsimoniousError> {
        match self.tokens.get(self.at).cloned() {
            Some(Token::Text(value)) => {
                self.at += 1;
                Ok(Ast {
                    kind: ExpressionKind::Literal(value),
                })
            }
            Some(Token::Regex(value)) => {
                self.at += 1;
                Ok(Ast {
                    kind: ExpressionKind::Regex(value),
                })
            }
            Some(Token::Ident(name)) => {
                self.at += 1;
                Ok(Ast {
                    kind: ExpressionKind::Ref(name),
                })
            }
            Some(Token::Sym('(')) => {
                self.at += 1;
                let result = self.expression()?;
                self.sym(')')?;
                Ok(result)
            }
            _ => Err(BadGrammar::new("expected grammar atom").into()),
        }
    }
    fn take_quantifier(&mut self) -> Result<Option<(usize, Option<usize>)>, ParsimoniousError> {
        let Some(Token::Sym(symbol)) = self.tokens.get(self.at) else {
            return Ok(None);
        };
        match symbol {
            '*' => {
                self.at += 1;
                Ok(Some((0, None)))
            }
            '+' => {
                self.at += 1;
                Ok(Some((1, None)))
            }
            '?' => {
                self.at += 1;
                Ok(Some((0, Some(1))))
            }
            '{' => {
                self.at += 1;
                let min_text = self.take_number();
                let comma = self.peek_sym(',');
                if comma {
                    self.at += 1;
                }
                let max_text = self.take_number();
                self.sym('}')?;
                let min = min_text
                    .as_deref()
                    .map(str::parse)
                    .transpose()
                    .map_err(|_| BadGrammar::new("invalid quantifier"))?
                    .unwrap_or(0);
                let max = if comma {
                    max_text
                        .as_deref()
                        .map(str::parse)
                        .transpose()
                        .map_err(|_| BadGrammar::new("invalid quantifier"))?
                } else {
                    Some(min)
                };
                Ok(Some((min, max)))
            }
            _ => Ok(None),
        }
    }

    fn take_number(&mut self) -> Option<String> {
        match self.tokens.get(self.at) {
            Some(Token::Ident(value))
                if value.chars().all(|character| character.is_ascii_digit()) =>
            {
                self.at += 1;
                Some(value.clone())
            }
            _ => None,
        }
    }
    fn starts_atom(&self) -> bool {
        matches!(
            self.tokens.get(self.at),
            Some(
                Token::Text(_)
                    | Token::Regex(_)
                    | Token::Sym('(')
                    | Token::Sym('!')
                    | Token::Sym('&')
            )
        ) || matches!(self.tokens.get(self.at), Some(Token::Ident(_)) if !matches!(self.tokens.get(self.at+1), Some(Token::Sym('='))))
    }
    fn ident(&mut self) -> Result<String, ParsimoniousError> {
        match self.tokens.get(self.at).cloned() {
            Some(Token::Ident(value)) => {
                self.at += 1;
                Ok(value)
            }
            _ => Err(BadGrammar::new("expected rule name").into()),
        }
    }
    fn sym(&mut self, value: char) -> Result<(), ParsimoniousError> {
        if self.peek_sym(value) {
            self.at += 1;
            Ok(())
        } else {
            Err(BadGrammar::new(format!("expected '{value}'")).into())
        }
    }
    fn peek_sym(&self, value: char) -> bool {
        self.tokens.get(self.at) == Some(&Token::Sym(value))
    }
}
fn lex(source: &str) -> Result<Vec<Token>, ParsimoniousError> {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if chars[i] == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if "=()/!&*+?{},".contains(chars[i]) {
            {
                out.push(Token::Sym(chars[i]));
            }
            i += 1;
            continue;
        }
        if chars[i] == '~' {
            i += 1;
            while i < chars.len() && matches!(chars[i], 'r' | 'u' | 'b') {
                i += 1;
            }
            let value = quoted(&chars, &mut i)?;
            out.push(Token::Regex(value));
            continue;
        }
        if chars[i] == '\''
            || chars[i] == '\"'
            || ((chars[i] == 'r' || chars[i] == 'u' || chars[i] == 'b')
                && i + 1 < chars.len()
                && (chars[i + 1] == '\'' || chars[i + 1] == '\"'))
        {
            while i < chars.len() && chars[i] != '\'' && chars[i] != '\"' {
                i += 1;
            }
            out.push(Token::Text(quoted(&chars, &mut i)?));
            continue;
        }
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' || chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Ident(chars[start..i].iter().collect()));
            continue;
        }
        return Err(BadGrammar::new("invalid character in grammar").into());
    }
    Ok(out)
}
fn quoted(chars: &[char], at: &mut usize) -> Result<String, ParsimoniousError> {
    let quote = *chars
        .get(*at)
        .ok_or_else(|| BadGrammar::new("unterminated string literal"))?;
    *at += 1;
    let mut out = String::new();
    while *at < chars.len() && chars[*at] != quote {
        if chars[*at] == '\\' {
            *at += 1;
            let c = *chars
                .get(*at)
                .ok_or_else(|| BadGrammar::new("unterminated escape"))?;
            out.push(match c {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            *at += 1;
        } else {
            out.push(chars[*at]);
            *at += 1;
        }
    }
    if *at >= chars.len() {
        return Err(BadGrammar::new("unterminated string literal").into());
    }
    *at += 1;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exceptions::ParsimoniousError;

    #[test]
    fn grammar_parses_ordered_choice_and_repetition() {
        let grammar = Grammar::new("word = 'a' / 'ab'\nlist = word+")
            .and_then(|grammar| grammar.default("list"));
        let node = grammar.and_then(|grammar| grammar.parse("aaa", 0));
        assert_eq!(node.map(|node| node.text()), Ok("aaa".to_owned()));
    }

    #[test]
    fn grammar_preserves_unicode_scalar_offsets() {
        let grammar = Grammar::new("word = 'ä' 'b'");
        let node = grammar.and_then(|grammar| grammar.parse("äb", 0));
        assert_eq!(node.map(|node| (node.start, node.end)), Ok((0, 2)));
    }

    #[test]
    fn bounded_quantifiers_accept_their_declared_range() {
        let grammar = match Grammar::new("root = 'a'{2,3}") {
            Ok(grammar) => grammar,
            Err(error) => panic!("grammar compilation failed: {error}"),
        };
        assert_eq!(grammar.parse("aaa", 0).map(|node| node.end), Ok(3));
        assert!(grammar.match_text("a", 0).is_err());
    }

    #[test]
    fn undefined_reference_is_an_explicit_error() {
        assert!(matches!(
            Grammar::new("root = missing"),
            Err(ParsimoniousError::UndefinedLabel(_))
        ));
    }

    #[test]
    fn left_recursion_returns_error_instead_of_recursing_forever() {
        let grammar = Grammar::new("root = root 'a' / 'a'");
        assert!(matches!(
            grammar.and_then(|grammar| grammar.match_text("a", 0)),
            Err(ParsimoniousError::LeftRecursion(_))
        ));
    }
}
