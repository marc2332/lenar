use lenar::runtime::*;

static CODE: &str = r#"

let name = "Hello World!!";

let iterator = fn(v) {
    print(v);
};

iter(name iterator);

"#;

fn main() {
    Runtime::run(CODE);
}
