use lenar::*;

fn main() {
    use tokenizer::*;
    use vm::*;

    let code = r#"
       print({
        let idk = "dasda";

        "nice"
       })
    "#;

    let tokenizer = Tokenizer::new(&code);

    let vm = VM::new(tokenizer);

    vm.run();
}
