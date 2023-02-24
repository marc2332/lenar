use std::sync::Arc;

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

    let tokenizer = Arc::new(Tokenizer::new(&code));

    let res = Runtime::evaluate(&tokenizer);

    assert_eq!(res, LenarValue::Bytes("test".as_bytes()));
}
