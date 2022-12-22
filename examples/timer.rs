use std::time::Instant;

use lenar::tokenizer;

fn main() {
    use tokenizer::*;

    let now = Instant::now();

    let code = r#"
        var test = { "test" };
        { { } }
        { }
        { { { var hola = "wow"; } } }
        { }
    "#
    .repeat(1000);

    Tokenizer::new(&code);

    println!("{}s", now.elapsed().as_secs_f32());
}
