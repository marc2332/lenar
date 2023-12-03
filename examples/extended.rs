use std::sync::Arc;

use lenar::*;

fn main() {
    use parser::*;
    use runtime::*;

    let code = r#"
        coolFunc(coolInstance.hey);
    "#;

    let parser = Parser::new(&code).wrap();

    let mut scope = Scope::default();
    scope.setup_globals();

    #[derive(Debug)]
    struct CoolInstance;

    impl RuntimeInstance for CoolInstance {
        fn get_prop(&self, prop: &str) -> LenarValue {
            if prop == "hey" {
                LenarValue::Str("hey".to_string())
            } else {
                LenarValue::Void
            }
        }

        fn get_name(&self) -> &str {
            "coolInstance"
        }
    }

    #[derive(Debug)]
    struct CoolFunc;

    impl RuntimeFunction for CoolFunc {
        fn call(
            &mut self,
            mut args: Vec<LenarValue>,
            _objects_map: &Arc<Parser>,
        ) -> LenarResult<LenarValue> {
            let val = args.remove(0);
            let val = val.to_string();
            println!("{val}");
            Ok(LenarValue::Void)
        }

        fn get_name(&self) -> &str {
            "coolFunc"
        }
    }

    scope.add_global_function(CoolFunc);
    scope.add_global_instance(CoolInstance);

    Runtime::run_with_scope(&mut scope, &parser).unwrap();
}
