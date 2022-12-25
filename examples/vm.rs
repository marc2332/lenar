use lenar::*;

fn main() {
    use tokenizer::*;
    use vm::*;

    let code = r#"
        let msg = "Hello World!";
        println(msg);
        println(Lenar.version);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let vm = VM::new(tokenizer);

    vm.run();
}
