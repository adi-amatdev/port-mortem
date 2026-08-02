#![forbid(unsafe_code)]
//! Port of the source library.
//!
//! Generated scaffold. Modules are added by the harness as translations land;
//! the crate root itself is harness-owned so the unsafe guard cannot be
//! edited away by a coder agent.

pub mod exceptions;
pub mod expressions;
pub mod grammar;
pub mod nodes;
pub mod utils;

// Package-level facade matching `parsimonious.__init__`.
pub use exceptions::{BadGrammar, IncompleteParseError, ParseError, UndefinedLabel, VisitationError};
pub use grammar::{Grammar, TokenGrammar};
pub use nodes::{rule, NodeVisitor, Rule};

#[cfg(test)]
mod public_api_tests {
    use super::*;
    use core::convert::Infallible;

    struct Visitor;

    impl NodeVisitor for Visitor {
        type Output = usize;
        type Error = Infallible;

        fn visit(&self, node: &nodes::Node) -> Result<Self::Output, Self::Error> {
            Ok(node.children.len())
        }
    }

    #[test]
    fn package_facade_exports_are_usable_from_the_crate_root() {
        let grammar: TokenGrammar = match Grammar::new("root = 'a'") {
            Ok(grammar) => grammar,
            Err(error) => panic!("facade grammar must compile: {error}"),
        };
        let _ = grammar;
        assert_eq!(rule("root = 'a'").definition, "root = 'a'");
        let node = nodes::Node::new("root", "a", 0, 1, vec![]);
        assert_eq!(Visitor.visit(&node), Ok(0));
    }
}
