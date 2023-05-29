use std::sync::Arc;

use ansi_term::{Color, Style};
use lenar::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

fn main() {
    use parser::*;
    use runtime::*;

    #[derive(Debug)]
    struct ClearFunc;

    impl RuntimeFunction for ClearFunc {
        fn call<'s>(
            &mut self,
            _args: Vec<LenarValue<'s>>,
            _objects_map: &'s Arc<Parser>,
        ) -> LenarResult<LenarValue<'s>> {
            print!("\x1B[2J\x1B[1;1H");
            Ok(LenarValue::Void)
        }

        fn get_name<'s>(&self) -> &'s str {
            "clear"
        }
    }

    let mut code = "".to_string();

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic(">".to_string()),
        DefaultPromptSegment::Empty,
    );

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                code.push_str(&buffer);

                let parser = Parser::new(&code).wrap();

                let mut scope = Scope::default();
                scope.setup_globals();
                scope.add_global_function(ClearFunc);

                let res = Runtime::run_with_scope(&mut scope, &parser);

                if let Ok(res) = res {
                    println!(
                        "{}",
                        Style::new()
                            .fg(Color::RGB(190, 190, 190))
                            .paint(res.to_string())
                    );
                } else if let Err(res) = res {
                    println!(
                        "Error: {}",
                        Style::new().fg(Color::Red).paint(format!("{res:?}"))
                    );
                }
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            }
            _ => {}
        }
    }
}
