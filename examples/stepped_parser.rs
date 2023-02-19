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

    Runtime::evaluate(&tokenizer);

    let code = r#"
        hey("marc");
    "#;

    tokenizer.parse(code);

    Runtime::evaluate(&tokenizer);
}
