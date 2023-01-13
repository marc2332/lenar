use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        let func = fn(x) { 
            println(x); 
            "hello world"
        };

        let read = fn(file_path){
            let file = openFile(file_path);
            toString(file)
        };

        println(read("examples/fs.rs"));
         
        println(func("hola"));
        println(Lenar.version);
    "#;

    let tokenizer = Tokenizer::new(&code);

    let runtime = Runtime::new(tokenizer);

    runtime.run();
}
