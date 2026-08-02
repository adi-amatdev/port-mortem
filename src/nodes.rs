//! Parse-tree nodes produced by expressions.

use std::fmt;

/// A declarative grammar rule associated with a visitor implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    /// Parsimonious grammar syntax for the rule.
    pub definition: String,
}

/// Construct rule metadata for a Rust [`NodeVisitor`].
pub fn rule(definition: impl Into<String>) -> Rule {
    Rule {
        definition: definition.into(),
    }
}

/// Rust equivalent of Parsimonious's visitor contract.
///
/// Rust makes the output and error types explicit instead of dispatching
/// dynamically to Python `visit_*` methods.
pub trait NodeVisitor {
    /// Value produced by a successful visit.
    type Output;
    /// Error produced by a visitor implementation.
    type Error;

    /// Visit one parse-tree node.
    fn visit(&self, node: &Node) -> Result<Self::Output, Self::Error>;
}

/// A safe, owned parse-tree node. Positions use Unicode scalar offsets, as
/// Python strings do, rather than UTF-8 byte offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// Name of the expression which produced this node, if it is a rule.
    pub expr_name: String,
    /// Complete input supplied to the parser.
    pub full_text: String,
    /// Inclusive start offset in Unicode scalar values.
    pub start: usize,
    /// Exclusive end offset in Unicode scalar values.
    pub end: usize,
    /// Nodes produced by child expressions.
    pub children: Vec<Node>,
}

impl Node {
    pub(crate) fn new(
        expr_name: impl Into<String>,
        text: &str,
        start: usize,
        end: usize,
        children: Vec<Node>,
    ) -> Self {
        Self {
            expr_name: expr_name.into(),
            full_text: text.to_owned(),
            start,
            end,
            children,
        }
    }

    /// The input slice matched by this node.
    pub fn text(&self) -> String {
        self.full_text
            .chars()
            .skip(self.start)
            .take(self.end.saturating_sub(self.start))
            .collect()
    }

    /// Render this node and descendants in Parsimonious's human-readable form.
    pub fn prettily(&self) -> String {
        self.pretty_with_indent(0)
    }

    fn pretty_with_indent(&self, depth: usize) -> String {
        let mut out = String::new();
        out.push_str(&"    ".repeat(depth));
        let called = if self.expr_name.is_empty() {
            String::new()
        } else {
            format!(" called \"{}\"", self.expr_name)
        };
        out.push_str(&format!("<Node{called} matching \"{}\">", self.text()));
        for child in &self.children {
            out.push('\n');
            out.push_str(&child.pretty_with_indent(depth + 1));
        }
        out
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.prettily())
    }
}
