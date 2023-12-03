use std::sync::Arc;

use lenar::{parser::Parser, runtime::*};

static CODE: &str = r#"

    let printIter = fn(v) [] {
        iter(v fn(v) [] { print(v) })
    };

    printIter(list(1 2 3))

"#;

fn main() {
    let parser = Arc::new(Parser::new(CODE));
    Runtime::evaluate(&parser).unwrap();
}
