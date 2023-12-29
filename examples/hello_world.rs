use std::sync::Arc;

use lenar::{parser::Parser, runtime::*};

static CODE: &str = r#"
    println("Running -> " Lenar.version);

    let speak = fn(msg) [] {
        println(msg);
        "Hey!"
    };

    println(speak("Hello?"));

    if(isEqual(1 1)) {
        println(speak("Hello again!"));
    }
"#;

fn main() {
    let parser = Arc::new(Parser::new(CODE));
    Runtime::evaluate(&parser).unwrap();
}
