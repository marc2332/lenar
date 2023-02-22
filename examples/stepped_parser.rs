use std::sync::Arc;

use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let hey = fn(v) {
            println("hey " v);
        };
    "#;

    let mut tokenizer = Tokenizer::new(&code);

    let code = r#"
        hey("marc");
    "#;

    tokenizer.parse(code);

    Runtime::evaluate(&Arc::new(tokenizer));
}
