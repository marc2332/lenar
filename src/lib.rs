pub mod tokenizer {
    use std::str::Chars;

    pub use slab::Slab;

    pub type TokenKey = usize;

    /// `Tokenizer` transforms an input, e.g a string, into a a Tokens map
    #[derive(Debug)]
    pub struct Tokenizer {
        tokens: Slab<Token>,
        global_block: TokenKey,
    }

    #[derive(Debug)]
    pub enum Token {
        Block {
            tokens: Vec<TokenKey>,
        },
        VarDef {
            block_value: TokenKey,
            var_name: String,
        },
        StringVal {
            value: String,
        },
        FunctionCall {
            fn_name: String,
            arguments: TokenKey,
        },
    }

    impl Token {
        #[inline(always)]
        pub fn add_token(&mut self, token: TokenKey) {
            if let Token::Block { tokens } = self {
                tokens.push(token);
            }
        }
    }

    #[inline(always)]
    fn slice_with_size(start: usize, end: usize, code: &str) -> Option<&str> {
        if code.len() < end {
            None
        } else {
            Some(&code[start..end])
        }
    }

    #[inline(always)]
    fn slice_until(until: char, code: &mut Chars) -> String {
        code.take_while(|&v| v != until).collect::<String>()
    }

    #[inline(always)]
    fn find_pos_until_is_not_char(start: usize, until: char, code: &str) -> usize {
        let code = &code[start..];
        code.chars().take_while(|&v| v == until).count()
    }

    enum PerfomedAction {
        EnteredGlobalScope,
        DefinedVariable,
        OpenedBlock,
        ClosedBlock,
        ClosedStatement,
        OpenedString,
        ClosedString,
        FoundOperator(char),
        CalledFunction,
    }

    impl Tokenizer {
        pub fn new(code: &str) -> Self {
            let mut tokens_map = Slab::new();

            let global_block_token = Token::Block { tokens: Vec::new() };
            let global_block = tokens_map.insert(global_block_token);

            let mut block_indexes = vec![global_block];
            let mut string_count = 0;
            let mut last_action = PerfomedAction::EnteredGlobalScope;

            let len = code.len();
            let mut chars = code.chars();

            fn advance_by(how_much: usize, chars: &mut Chars) {
                for _ in 0..how_much {
                    chars.next();
                }
            }

            loop {
                let i = len - chars.size_hint().1.unwrap();

                let val = chars.next();

                if val.is_none() {
                    break;
                }

                // Skip spaces and line breaks
                if string_count == 0 && (val == Some(' ') || val == Some('\n')) {
                    advance_by(find_pos_until_is_not_char(i + 1, ' ', code), &mut chars);
                    continue;
                }

                let val = val.unwrap();

                let current_block = *block_indexes.last().unwrap();

                // TODO closing parenthesis should only close the last `arguments` block not an actual code block
                if val == ')' && string_count == 0 {
                    block_indexes.pop();
                    continue;
                }

                // Check operator syntax
                if val == '=' {
                    if matches!(last_action, PerfomedAction::DefinedVariable) {
                        last_action = PerfomedAction::FoundOperator('=');
                    } else {
                        panic!("Syntax error: Operator '=' is used to define initial values to variables.")
                    }
                    continue;
                }

                // End a statement
                if val == ';' {
                    if string_count == 0 {
                        block_indexes.pop();
                        last_action = PerfomedAction::ClosedStatement;
                    }
                    continue;
                }

                if val == '"' {
                    // String closed
                    if string_count > 0 {
                        let string_val = Token::StringVal {
                            value: code[i - string_count + 1..i].chars().collect::<String>(),
                        };

                        let string_key = tokens_map.insert(string_val);

                        let block_value = tokens_map.get_mut(current_block).unwrap();
                        if let Token::Block { tokens } = block_value {
                            tokens.push(string_key);
                        }
                        last_action = PerfomedAction::ClosedString;
                        string_count = 0
                    } else {
                        last_action = PerfomedAction::OpenedString;
                        string_count += 1;
                    }
                    continue;
                }

                // Start a block
                if val == '{' && string_count == 0 {
                    let block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(block);

                    block_indexes.push(block_key);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    last_action = PerfomedAction::OpenedBlock;

                    continue;
                }

                // Closing a block
                if val == '}' && string_count == 0 {
                    block_indexes.pop();
                    last_action = PerfomedAction::ClosedBlock;
                    continue;
                }

                // Variable declarations
                if string_count == 0 && slice_with_size(i, i + 3, code) == Some("let") {
                    advance_by(3, &mut chars);
                    let var_name = slice_until(' ', &mut chars);
                    let value_block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(value_block);

                    let var_def = Token::VarDef {
                        block_value: block_key,
                        var_name,
                    };
                    let var_key = tokens_map.insert(var_def);

                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(var_key);

                    block_indexes.push(block_key);

                    last_action = PerfomedAction::DefinedVariable;

                    continue;
                }

                if string_count > 0 {
                    string_count += 1;
                    continue;
                }

                if string_count == 0 {
                    let fn_name = slice_until('(', &mut chars);
                    let fn_name = format!("{}{}", val, fn_name);

                    let value_block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(value_block);

                    let fn_def = Token::FunctionCall {
                        fn_name,
                        arguments: block_key,
                    };
                    let fn_key = tokens_map.insert(fn_def);

                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(fn_key);

                    block_indexes.push(block_key);

                    last_action = PerfomedAction::CalledFunction;

                    continue;
                }
            }

            Self {
                tokens: tokens_map,
                global_block,
            }
        }

        /// Retrieve the global block token
        pub fn get_global(&self) -> TokenKey {
            self.global_block
        }

        /// Retrieve a Token given a `key`
        #[inline(always)]
        pub fn get_token(&self, key: TokenKey) -> Option<&Token> {
            self.tokens.get(key)
        }
    }
}

pub mod vm {
    use std::{
        collections::HashMap,
        io::{stdout, Write},
    };

    use crate::tokenizer::{Token, Tokenizer};

    pub struct VM {
        tokenizer: Tokenizer,
    }

    impl VM {
        pub fn new(tokenizer: Tokenizer) -> Self {
            Self { tokenizer }
        }

        pub fn run(&self) {
            let mut context = Context::new();

            context.setup_globals();

            let global_token = self.tokenizer.get_global();
            let global_block = self.tokenizer.get_token(global_token);

            let tok = global_block.unwrap();

            compute_expr(tok, &self.tokenizer, &mut context);
        }
    }

    #[derive(Debug)]
    pub enum VMType {
        List(Vec<VMType>),
        String(String),
        Void,
    }

    pub trait VMFunction {
        fn call(&mut self, _args: &Vec<VMType>) {
            panic!("This is not a function.")
        }
    }

    pub struct Context {
        functions: HashMap<usize, HashMap<String, Box<dyn VMFunction>>>,
    }

    impl Context {
        pub fn new() -> Self {
            Self {
                functions: HashMap::default(),
            }
        }

        pub fn setup_globals(&mut self) {
            self.functions.insert(0, HashMap::default());

            let global_scope = self.functions.get_mut(&0).unwrap();

            struct PrintFunc;

            impl VMFunction for PrintFunc {
                fn call(&mut self, args: &Vec<VMType>) {
                    args.iter().for_each(|v| {
                        if let VMType::String(string) = &v {
                            stdout().write(string.as_bytes()).unwrap();
                        }
                    });
                    stdout().flush().unwrap();
                }
            }

            global_scope.insert("print".to_string(), Box::new(PrintFunc));
        }

        pub fn call_function(
            &mut self,
            name: impl AsRef<str>,
            scope_id: Option<usize>,
            args: &Vec<VMType>,
        ) -> VMType {
            let scope_id = scope_id.unwrap_or(0);

            let scope = self.functions.get_mut(&scope_id);

            if let Some(scope) = scope {
                let func = scope.get_mut(name.as_ref());
                if let Some(func) = func {
                    func.call(args)
                }
            }

            VMType::Void
        }
    }

    fn compute_expr(token: &Token, tokens_map: &Tokenizer, context: &mut Context) -> VMType {
        match token {
            Token::Block { tokens } => {
                for (i, tok) in tokens.iter().enumerate() {
                    let is_last = i == tokens.len() - 1;
                    let tok = tokens_map.get_token(*tok).unwrap();
                    let res = compute_expr(tok, tokens_map, context);
                    if is_last {
                        return res;
                    }
                }

                VMType::Void
            }
            Token::VarDef { .. } => {
                //let value = tokens_map.get_token(*block_value).unwrap();
                //let res = compute_expr(value,tokens_map, context);

                VMType::Void
            }
            Token::FunctionCall { arguments, fn_name } => {
                let value = tokens_map.get_token(*arguments).unwrap();
                let mut args = Vec::new();
                if let Token::Block { tokens } = value {
                    for tok in tokens {
                        let tok = tokens_map.get_token(*tok).unwrap();
                        let res = compute_expr(tok, tokens_map, context);
                        args.push(res);
                    }
                }

                context.call_function(&fn_name, None, &args);

                VMType::Void
            }
            Token::StringVal { value } => VMType::String(value.to_string()),
        }
    }
}
