use lenar::*;

fn main() {
    use tokenizer::*;
    use vm::*;

    let code = r#"
       println({
        println("hey")
        "nice"
       })
    "#;

    let tokenizer = Tokenizer::new(&code);

    let vm = VM::new(tokenizer);

    vm.run();
}
