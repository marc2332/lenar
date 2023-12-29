use ansi_term::{Color, Style};
use lenar::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

fn main() {
    use parser::*;
    use runtime::*;

    #[derive(Debug)]
    struct ClearFunc;

    impl RuntimeFunction for ClearFunc {
        fn call(
            &mut self,
            _args: Vec<LenarValue>,
            _objects_map: &Parser,
        ) -> LenarResult<LenarValue> {
            print!("\x1B[2J\x1B[1;1H");
            Ok(LenarValue::Void)
        }

        fn get_name(&self) -> &str {
            "clear"
        }
    }
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic(">".to_string()),
        DefaultPromptSegment::Empty,
    );

    let mut parser = Parser::new("");

    let mut scope = Scope::default();
    scope.setup_globals();
    scope.add_global_function(ClearFunc);

    let mut execution = Runtime::run_with_scope(&mut scope, &parser);

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                parser.parse(&buffer);

                execution =
                    Runtime::resume_execution(&mut scope, &parser, execution.scope_position);

                if let Ok(res) = execution.result {
                    println!(
                        "{}",
                        Style::new()
                            .fg(Color::RGB(190, 190, 190))
                            .paint(res.to_string())
                    );
                } else if let Err(err) = execution.result {
                    println!(
                        "Error: {}",
                        Style::new().fg(Color::Red).paint(format!("{err:?}"))
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
