//! Rust integration adapter for the pinned Parsimonious original-suite themes.
//!
//! This target deliberately does not execute Python or import the frozen
//! `tests/original/` files.  It mirrors the highest-value behavioural cases in
//! Rust so `cargo test --test original` exercises the port through its public
//! API.  It is a smoke subset, not a claim of one-to-one coverage of every
//! Python test; unsupported dynamic-Python APIs remain outside this adapter.

use core::convert::Infallible;
use harness_factory_rs::exceptions::ParsimoniousError;
use harness_factory_rs::expressions::{
    literal, not, one_or_more_min, optional, zero_or_more,
};
use harness_factory_rs::{rule, Grammar, NodeVisitor};

fn grammar() -> Grammar {
    match Grammar::new("root = 'a'") {
        Ok(value) => value,
        Err(error) => panic!("fixture grammar must compile: {error}"),
    }
}

#[test]
fn original_grammar_construction_and_parse_success() {
    let parsed = match Grammar::new("root = 'a'+").and_then(|grammar| grammar.parse("aaa", 0)) {
        Ok(node) => node,
        Err(error) => panic!("original grammar success case failed: {error}"),
    };
    assert_eq!(parsed.text(), "aaa");
    assert_eq!((parsed.start, parsed.end), (0, 3));
}

#[test]
fn original_parse_failure_and_incomplete_error_types() {
    let parser = grammar();
    assert!(matches!(parser.parse("b", 0), Err(ParsimoniousError::Parse(_))));
    assert!(matches!(parser.parse("ab", 0), Err(ParsimoniousError::IncompleteParse(_))));
}

#[test]
fn original_not_lookahead_preserves_position() {
    let parser = grammar();
    let expr = not(literal("a"));
    let node = match expr.match_text("b", 0, &parser) {
        Ok(node) => node,
        Err(error) => panic!("negative lookahead should match: {error}"),
    };
    assert_eq!((node.start, node.end), (0, 0));
    assert!(matches!(expr.match_text("a", 0, &parser), Err(ParsimoniousError::Parse(_))));
}

#[test]
fn original_optional_and_repetition_behaviour() {
    let parser = grammar();
    let maybe = optional(literal("a"));
    let absent = match maybe.match_text("b", 0, &parser) {
        Ok(node) => node,
        Err(error) => panic!("optional should accept an absent member: {error}"),
    };
    assert_eq!((absent.start, absent.end, absent.children.len()), (0, 0, 0));

    let many = zero_or_more(literal("a"));
    let many_node = match many.match_text("aaa", 0, &parser) {
        Ok(node) => node,
        Err(error) => panic!("zero-or-more should match: {error}"),
    };
    assert_eq!((many_node.end, many_node.children.len()), (3, 3));

    let at_least_two = one_or_more_min(literal("a"), 2);
    assert!(at_least_two.match_text("a", 0, &parser).is_err());
    assert_eq!(
        at_least_two.match_text("aa", 0, &parser).map(|node| node.end),
        Ok(2)
    );
}

struct ChildCount;

impl NodeVisitor for ChildCount {
    type Output = usize;
    type Error = Infallible;

    fn visit(&self, node: &harness_factory_rs::nodes::Node) -> Result<Self::Output, Self::Error> {
        Ok(node.children.len())
    }
}

#[test]
fn original_node_visitor_and_rule_facade() {
    let metadata = rule("root = 'a'");
    assert_eq!(metadata.definition, "root = 'a'");
    let node = match grammar().parse("a", 0) {
        Ok(node) => node,
        Err(error) => panic!("visitor fixture parse failed: {error}"),
    };
    // Grammar::parse returns the named root-rule node itself; the referenced
    // literal is not wrapped as a child at this public-API boundary.
    assert_eq!(ChildCount.visit(&node), Ok(0));
}
