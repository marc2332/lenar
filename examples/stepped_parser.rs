use std::sync::Arc;

use lenar::*;

fn main() {
    use parser::*;
    use runtime::*;

    let code = r#"
        let hey = fn(v) {
            println("hey " v);
        };
    "#;

    let mut parser = Parser::new(&code);

    let code = r#"
        hey("marc");
    "#;

    parser.parse(code);

    Runtime::evaluate(&Arc::new(parser)).unwrap();
}
