//! Safe, standalone parser benchmark probe.
//!
//! The external benchmark runner invokes this binary with the same grammar,
//! input, warmup, and iteration count used for the Python original. No Python
//! runtime is linked into this crate.

use harness_factory_rs::Grammar;
use std::env;
use std::time::{Duration, Instant};

fn arg(name: &str) -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(key) = args.next() {
        if key == name {
            return args.next();
        }
    }
    None
}

fn parse_count(value: Option<String>, name: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {name}"))
}

fn main() {
    if env::args().any(|arg| arg == "--startup") {
        println!("STARTUP|ok");
        return;
    }
    let grammar_source = match arg("--grammar") {
        Some(value) => value,
        None => {
            eprintln!("missing --grammar");
            std::process::exit(2);
        }
    };
    let input = match arg("--input") {
        Some(value) => value,
        None => {
            eprintln!("missing --input");
            std::process::exit(2);
        }
    };
    let warmup = match parse_count(arg("--warmup"), "--warmup") {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let iterations = match parse_count(arg("--iterations"), "--iterations") {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let grammar = match Grammar::new(&grammar_source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("grammar error: {error}");
            std::process::exit(2);
        }
    };
    for _ in 0..warmup {
        let _ = grammar.parse(&input, 0);
    }
    if let Some(seconds) = arg("--memory-seconds") {
        let seconds = seconds.parse::<f64>().unwrap_or(1.0);
        let deadline = Instant::now() + Duration::from_secs_f64(seconds);
        while Instant::now() < deadline {
            let _ = grammar.parse(&input, 0);
        }
        println!("MEMORY|ok");
        return;
    }
    let started = Instant::now();
    let mut successes = 0usize;
    let mut errors = 0usize;
    for _ in 0..iterations {
        if grammar.parse(&input, 0).is_ok() {
            successes += 1;
        } else {
            errors += 1;
        }
    }
    println!("RESULT|{}|{}|{}", started.elapsed().as_nanos(), successes, errors);
}
