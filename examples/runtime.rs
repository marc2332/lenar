use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let func = fn(x) { 
            println(x); 
            "hello world"
        };
 
        println(func("hola"));
        println(Lenar.version);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    runtime.run();
}
