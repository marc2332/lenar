use lenar::*;

fn main() {
    use tokenizer::*;
    use vm::*;

    let code = r#"
        let test = "Hello World!";
        println(test);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let vm = VM::new(tokenizer);

    vm.run();
}
