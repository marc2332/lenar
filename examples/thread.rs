use lenar::*;

fn main() {
    use runtime::*;

    let code = r#"
        let handle = thread(
            fn(callback someOtherVal) {
                callback(someOtherVal);
                sleep("500");
            } 
            fn(v) { println(v); sleep("1000") }
            "Some other val"
        );
        
        wait(handle);

        println("Finished!");
    "#;

    Runtime::run(&code);
}
