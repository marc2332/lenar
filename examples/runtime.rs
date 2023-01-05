use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let val = "last value";
        {
            let val = "first value!";
            println(val);
        }
        println(val);
        println(Lenar.version);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    runtime.run();
}