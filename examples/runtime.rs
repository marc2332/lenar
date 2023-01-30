use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        if(isEqual("test" "test")) {
            let something = fn(v) {
                println("test");
                v
            };
            println(something("hey"));
        };

        println(if(isEqual("test" "test")) { "wow" });

        println(isEqual("yes" "no"));
        
        "test"
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    let res = runtime.evaluate().to_string();

    assert_eq!(res, "test");
}
