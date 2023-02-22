use std::sync::Arc;

use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        test("hey");
    "#;

    let tokenizer = Tokenizer::new(&code).wrap();

    let mut scope = Scope::default();
    scope.setup_globals();

    #[derive(Debug)]
    struct Test;

    impl RuntimeFunction for Test {
        fn call<'s>(
            &mut self,
            mut args: Vec<LenarValue<'s>>,
            _tokens_map: &'s Arc<Tokenizer>,
        ) -> LenarValue<'s> {
            let val = args.remove(0);
            let val = val.to_string();
            println!("{val}");
            LenarValue::Void
        }

        fn get_name<'s>(&self) -> &'s str {
            "test"
        }
    }

    scope.add_global_function(Test);

    Runtime::run_with_scope(&mut scope, &tokenizer);
}
