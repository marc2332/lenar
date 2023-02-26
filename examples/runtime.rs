use std::sync::Arc;

use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"

        println(Ok("test"));
        println(Err("error!"));

        println(isOk(Ok(5)));
        println(isOk(Err("Something went wrong")));

        println(unwrap(Ok(5)));
        println(unwrapErr(Err("Something went wrong")));
        
    "#;

    let tokenizer = Arc::new(Tokenizer::new(&code));

    Runtime::evaluate(&tokenizer);
}
