use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let file = openFile("examples/fs.rs");
        
        iter(file fn(byte){
            print(byte);
        });

        let list = newList("51" "19" "8" "14");

        iter(list fn(number index){
            println(index "-" number)
        });
    "#;

    let tokenizer = Tokenizer::new(&code);

    Runtime::evaluate(&tokenizer);
}
