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
        code.chars().take_while(|&v| v == until).count() - 1
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

                if val == Some(' ') {
                    advance_by(find_pos_until_is_not_char(i, ' ', code), &mut chars);
                    continue;
                }

                let val = val.unwrap();

                let current_block = *block_indexes.last().unwrap();

                if val == '=' {
                    if matches!(last_action, PerfomedAction::DefinedVariable) {
                        last_action = PerfomedAction::FoundOperator('=');
                    } else {
                        panic!("Syntax error: Operator '=' is used to define initial values to variables.")
                    }
                    continue;
                }

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

                if val == '{' && string_count == 0 {
                    let block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(block);

                    block_indexes.push(block_key);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    last_action = PerfomedAction::OpenedBlock;

                    continue;
                }

                if val == '}' && string_count == 0 {
                    block_indexes.pop();
                    last_action = PerfomedAction::ClosedBlock;
                    continue;
                }

                if string_count == 0 && slice_with_size(i, i + 3, code) == Some("var") {
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
