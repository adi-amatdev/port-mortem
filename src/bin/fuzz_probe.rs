//! Machine-readable, Python-free probe for the external differential oracle.
//!
//! Input is two newline-separated UTF-8 records: grammar then input text.
//! Output is a stable line format rather than JSON so no serialization crate
//! is required.  This binary ships no Python linkage and uses no unsafe code.

use harness_factory_rs::exceptions::ParsimoniousError;
use harness_factory_rs::nodes::Node;
use harness_factory_rs::Grammar;
use std::io::{self, Read};

fn shape(node: &Node) -> String {
    let children = node.children.iter().map(shape).collect::<Vec<_>>().join(",");
    format!("{}:{}[{}]", node.start, node.end, children)
}

fn error_record(error: ParsimoniousError) -> String {
    match error {
        ParsimoniousError::Parse(error) => format!("ERR|Parse|{}", error.pos),
        ParsimoniousError::IncompleteParse(error) => format!("ERR|IncompleteParse|{}", error.0.pos),
        ParsimoniousError::LeftRecursion(error) => format!("ERR|LeftRecursion|{}", error.0.pos),
        ParsimoniousError::BadGrammar(_) => "ERR|BadGrammar|-1".to_owned(),
        ParsimoniousError::UndefinedLabel(_) => "ERR|UndefinedLabel|-1".to_owned(),
        ParsimoniousError::Visitation(_) => "ERR|VisitationError|-1".to_owned(),
    }
}

fn main() {
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() {
        println!("ERR|ProbeInput|-1");
        return;
    }
    let Some((grammar_source, input)) = raw.split_once('\n') else {
        println!("ERR|ProbeInput|-1");
        return;
    };
    // Python's Windows text pipe may send CRLF. The protocol delimiter is not
    // parser input, so normalize that transport newline before parsing.
    let input = input.strip_suffix('\n').unwrap_or(input);
    let input = input.strip_suffix('\r').unwrap_or(input);
    let grammar = match Grammar::new(grammar_source) {
        Ok(grammar) => grammar,
        Err(error) => {
            println!("{}", error_record(error));
            return;
        }
    };
    match grammar.parse(input, 0) {
        Ok(node) => println!("OK|{}", shape(&node)),
        Err(error) => println!("{}", error_record(error)),
    }
}
