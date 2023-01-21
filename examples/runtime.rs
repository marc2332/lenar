use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        if(isEqual("test" "test")) {
            let something = fn(v) {
                println(Lenar.version);
                "hi"
            };
            println(something("hey"));
        };

        println(if(isEqual("test" "test")) { "wow" });
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    runtime.run();
}
