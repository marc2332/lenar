use lenar::*;

fn main() {
    use runtime::*;

    let code = r#"
        let handle = thread(
            fn(callback someOtherVal) [] {
                callback(someOtherVal);
                println("waiting 500ms");
                sleep(500);
            } 
            fn(v) [] { 
                println(v); 
                sleep(1000); 
            }
            "waiting 1000ms"
        );
        
        wait(handle);

        println("Finished!");
    "#;

    Runtime::run(&code);
}
