mod tokenizer {
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
        Block { tokens: Vec<TokenKey> },
        VarDef { block_value: TokenKey },
        StringVal { value: String },
    }

    impl Token {
        pub fn add_token(&mut self, token: TokenKey) {
            if let Token::Block { tokens } = self {
                tokens.push(token);
            }
        }
    }

    impl Tokenizer {
        pub fn from_str(code: &str) -> Self {
            #[inline(always)]
            fn slice_with_size(start: usize, end: usize, code: &str) -> Option<&str> {
                if code.len() < end { None }
                else { Some(&code[start..end]) }
            }

            fn slice_until(start: usize, until: char, code: &str) -> String {
                let code = &code[start..];
                code.chars()
                    .take_while(|&v| {
                        v == until
                    })
                    .collect::<String>()
            }

            let mut tokens_map = Slab::new();

            let global_block_token = Token::Block { tokens: Vec::new() };
            let global_block = tokens_map.insert(global_block_token);
            let mut block_indexes = vec![global_block];

            let mut i = 0;
            let mut string_count = 0;

            loop {
                let val = code.chars().nth(i);

                if val.is_none() {
                    break;
                }

                let val = val.unwrap();

                let current_block = *block_indexes.last().unwrap();

                if val == ';' {
                    if string_count == 0 {
                        block_indexes.pop();
                    }
                    i += 1;
                    continue;
                }

                if val == '"' {
                    // String closed
                    if string_count > 0 {
                        let string_val = Token::StringVal {
                            value: code
                                .chars()
                                .enumerate()
                                .filter(|&(c, _)| c >= string_count && c <= i)
                                .map(|(_, e)| e)
                                .collect::<String>(),
                        };
                        let string_key = tokens_map.insert(string_val);

                        let block_value = tokens_map.get_mut(current_block).unwrap();
                        if let Token::Block { tokens } = block_value {
                            tokens.push(string_key);
                        }
                        string_count = 0
                    } else {
                        string_count += 1;
                    }
                    i += 1;
                    continue;
                }

                if val == '{' && string_count == 0 {
                    let block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(block);

                    block_indexes.push(block_key);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    i += 1;
                    continue;
                }

                if val == '}' && string_count == 0 {
                    block_indexes.pop();
                    i += 1;
                    continue;
                }

                if slice_with_size(i, i + 3, code) == Some("var") {
                    let var_name = slice_until(i + 4, '=', code);

                    let value_block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(value_block);

                    let var_def = Token::VarDef {
                        block_value: block_key,
                    };
                    let var_key = tokens_map.insert(var_def);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(var_key);

                    block_indexes.push(block_key);

                    i += 3 + var_name.len();
                    continue;
                }

                i += 1;
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

fn main() {
    use tokenizer::*;

    let code = r#"
        var test = { "test" };
        { { } }
        { }
        { { { var hola = "wow"; } } }
        { }
    "#;











    let tokens_map = Tokenizer::from_str(&code);

    let global_token = tokens_map.get_global();
    let global_block = tokens_map.get_token(global_token);

    let tok = global_block.unwrap();

    fn iter_block(block: &Token, tokens_map: &Tokenizer, global: bool) {
        if let Token::Block { tokens } = block {
            println!("-> Inside of block (global: {})", global);
            for tok_id in tokens {
                let tok = tokens_map.get_token(*tok_id).unwrap();
                match tok {
                    Token::Block { .. } => {
                        iter_block(tok, tokens_map, false);
                    }
                    Token::VarDef { block_value } => {
                        let value = tokens_map.get_token(*block_value).unwrap();
                        if let Token::Block { tokens } = value {
                            println!(
                                "== Variable definition has a block with {}# statements",
                                tokens.len()
                            );
                        }
                    }
                    _ => {}
                }
            }
            println!("<- Leaving block (global: {})", global);
        }
    }

    iter_block(tok, &tokens_map, true);
}
