use lenar::*;

fn main() {
    use runtime::*;

    let code = r#"
        thread(
            fn(arg0) {
                println(arg0);
                let list = newList("0" "0" "0" "0" "0");

                iter(list fn(n i){
                    println(i);
                    sleep("1000");
                });
            } 
            "Pass a value to the thread!"
        );
        
        sleep("5000");

        println("Finished!");
    "#;

    Runtime::run(&code);
}
