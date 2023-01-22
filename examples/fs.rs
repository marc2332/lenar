use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let file = openFile("examples/fs.rs");
        let v = toString(file);
        println(v);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    runtime.evaluate();
}
