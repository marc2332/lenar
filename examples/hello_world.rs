use std::sync::Arc;

use lenar::{runtime::*, parser::Parser};

static CODE: &str = r#"

let a = fn() [] {
    println("A");
};

let b = fn() [a] {
    println("B");
    a();
};

b();

"#;

fn main() {
    let parser = Arc::new(Parser::new(CODE));
    Runtime::evaluate(&parser).unwrap();
}