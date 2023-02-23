use std::sync::Arc;

use lenar::*;

fn main() {
    use runtime::*;
    use tokenizer::*;

    let code = r#"
        coolFunc(coolInstance.hey);
    "#;

    let tokenizer = Tokenizer::new(&code).wrap();

    let mut scope = Scope::default();
    scope.setup_globals();

    #[derive(Debug)]
    struct CoolInstance;

    impl<'a> RuntimeInstance<'a> for CoolInstance {
        fn get_prop(&self, prop: &str) -> LenarValue<'a> {
            if prop == "hey" {
                LenarValue::Bytes("hey".as_bytes())
            } else {
                LenarValue::Void
            }
        }

        fn get_name<'s>(&self) -> &'s str {
            "coolInstance"
        }
    }

    #[derive(Debug)]
    struct CoolFunc;

    impl RuntimeFunction for CoolFunc {
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
            "coolFunc"
        }
    }

    scope.add_global_function(CoolFunc);
    scope.add_global_instance(CoolInstance);

    Runtime::run_with_scope(&mut scope, &tokenizer);
}
