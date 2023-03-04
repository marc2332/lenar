use std::time::Instant;

use lenar::parser;

fn main() {
    use parser::*;

    let now = Instant::now();

    let code = r#"
        let test = { { "test" } };
        {
            woooow("ok");
        }
    "#
    .repeat(10000000);

    Parser::new(&code);

    println!("{}s", now.elapsed().as_secs_f32());
}
