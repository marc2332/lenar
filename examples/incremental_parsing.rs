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

    tokenizer.parse(
        r#"
        hey("marc");
    "#,
    );

    Runtime::evaluate(&tokenizer);
}
