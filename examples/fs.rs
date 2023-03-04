use std::sync::Arc;

use lenar::*;

fn main() {
    use parser::*;
    use runtime::*;

    let code = r#"
        let file = openFile("examples/fs.rs");
        
        iter(file fn(byte){
            print(byte);
        });

        let list = newList(15 19 8 14);

        iter(list fn(number index){
            println(index "-" number)
        });
    "#;

    let parser = Arc::new(Parser::new(&code));

    Runtime::evaluate(&parser).unwrap();
}
