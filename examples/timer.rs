use std::time::Instant;

use lenar::tokenizer;

fn main() {
    use tokenizer::*;

    let now = Instant::now();

    let code = r#"
        let test = { { "test" } };
        {
            woooow("ok");
        }
    "#
    .repeat(10000000);

    Tokenizer::new(&code);

    println!("{}s", now.elapsed().as_secs_f32());
}
