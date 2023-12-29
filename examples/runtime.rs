use lenar::*;

fn main() {
    use parser::*;
    use runtime::*;

    let code = r#"

        if(isEqual("test" "test")) {
            let something = fn(v) [] {
                println(Lenar.version);
                "hi"
            };
            println(something("hey"));
        };
        
        println(if(isEqual("test" "test")) { "wow" });

        let value = ref(0);

        let modify = fn(v) [] {
            add(v 5)
        };

        println(value);
        modify(value);
        println(value);
    "#;

    let parser = Parser::new(&code);

    Runtime::evaluate(&parser).unwrap();
}
