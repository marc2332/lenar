mod tokenizer {
    pub use slotmap::{DefaultKey, SlotMap};

    /// `Tokenizer` transforms an input, e.g a string, into a a Tokens map
    #[derive(Debug)]
    pub struct Tokenizer {
        tokens: SlotMap<DefaultKey, Token>,
        global_block: DefaultKey,
    }

    pub enum TokenizerAction {
        GroupBlock { block: DefaultKey },
        AssignBlock { block: DefaultKey, to: DefaultKey },
    }

    #[derive(Debug)]
    pub enum Token {
        Block { tokens: Vec<DefaultKey> },
        VarDef { block_value: DefaultKey },
        StringVal { value: String },
    }

    impl Token {
        pub fn add_token(&mut self, token: DefaultKey) {
            if let Token::Block { tokens } = self {
                tokens.push(token);
            }
        }
    }

    impl Tokenizer {
        pub fn from_str(code: &str) -> Self {
            let mut tokens_map = SlotMap::new();

            let global_block_token = Token::Block { tokens: Vec::new() };
            let global_block = tokens_map.insert(global_block_token);
            let mut block_indexes = vec![global_block];
            let mut actions = vec![TokenizerAction::GroupBlock {
                block: global_block,
            }];

            let mut i = 0;
            let mut string_count = 0;

            loop {
                let val = code.chars().nth(i);

                if val.is_none() {
                    break;
                }

                let val = val.unwrap();

                let current_block = *block_indexes.last().unwrap();

                if val == '"' {
                    // String closed
                    if string_count > 0 {
                        let last_action = actions.last_mut();
                        if let Some(TokenizerAction::AssignBlock { block, .. }) = last_action {
                            let string_val = Token::StringVal {
                                value: code
                                    .chars()
                                    .enumerate()
                                    .filter(|&(c, _)| c >= string_count && c <= i)
                                    .map(|(_, e)| e)
                                    .collect::<String>(),
                            };
                            let string_key = tokens_map.insert(string_val);

                            let block_value = tokens_map.get_mut(*block).unwrap();
                            if let Token::Block { tokens } = block_value {
                                tokens.push(string_key);
                            }
                        }
                        string_count = 0
                    } else {
                        string_count += 1;
                    }
                }

                if val == '{' && string_count == 0 {
                    let block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(block);

                    let last_action = actions.last();
                    if !matches!(last_action, Some(TokenizerAction::AssignBlock { .. })) {
                        actions.push(TokenizerAction::GroupBlock { block: block_key })
                    }
                    block_indexes.push(block_key);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    i += 1;
                    continue;
                }

                if val == '}' && string_count == 0 {
                    let closing_block = actions.last().unwrap();
                    match closing_block {
                        TokenizerAction::GroupBlock { .. } => {
                            block_indexes.pop();
                            actions.pop();
                        }
                        TokenizerAction::AssignBlock { .. } => {
                            block_indexes.pop();
                            actions.pop();
                        }
                    }
                    i += 1;
                    continue;
                }

                let slice_with_size = |start: usize, end: usize, code: &str| -> String {
                    code.chars()
                        .enumerate()
                        .filter(|&(c, _)| c >= start && c <= end)
                        .map(|(_, e)| e)
                        .collect::<String>()
                };

                let slice_until = |start: usize, until: char, code: &str| -> String {
                    let mut in_string = false;
                    let mut block_indexes = 0;
                    code.chars()
                        .enumerate()
                        .filter(|&(c, _)| c >= start)
                        .take_while(|&(_, v)| {
                            if v == '{' && !in_string {
                                block_indexes += 1;
                            } else if v == '}' && !in_string {
                                block_indexes -= 1;
                            } else if v == '"' {
                                in_string = !in_string;
                            }
                            !(v == until && block_indexes == 0 && !in_string)
                        })
                        .map(|(_, e)| e)
                        .collect::<String>()
                };

                if slice_with_size(i, i + 2, code) == "var" {
                    let var_name = slice_until(i + 3, '=', code);

                    let value_block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens_map.insert(value_block);

                    let var_def = Token::VarDef {
                        block_value: block_key,
                    };
                    let var_key = tokens_map.insert(var_def);
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(var_key);

                    actions.push(TokenizerAction::AssignBlock {
                        block: block_key,
                        to: var_key,
                    });

                    i += 3 + var_name.len();
                }

                i += 1;
            }

            Self {
                tokens: tokens_map,
                global_block,
            }
        }

        /// Retrieve the global block token
        pub fn get_global(&self) -> DefaultKey {
            self.global_block
        }

        /// Retrieve a Token given a `key`
        #[inline(always)]
        pub fn get_token(&self, key: DefaultKey) -> Option<&Token> {
            self.tokens.get(key)
        }
    }
}

fn main() {
    use tokenizer::*;

    let code = r#"
        var test = { "test" "test"};
        { { } }
        { }
        { { { var hola = 1; } } }
    "#;

    let tokens_map = Tokenizer::from_str(code);

    let global_token = tokens_map.get_global();
    let global_block = tokens_map.get_token(global_token);

    let tok = global_block.unwrap();

    fn iter_block(block: &Token, tokens_map: &Tokenizer) {
        if let Token::Block { tokens } = block {
            println!("-> Inside of block");
            for tok_id in tokens {
                let tok = tokens_map.get_token(*tok_id).unwrap();
                match tok {
                    Token::Block { .. } => {
                        iter_block(tok, tokens_map);
                    }
                    Token::VarDef { block_value } => {
                        let value = tokens_map.get_token(*block_value).unwrap();
                        if let Token::Block { tokens } = value {
                            println!(
                                "== Var definition has a block with {}# statements",
                                tokens.len()
                            );
                        }
                    }
                    _ => {}
                }
            }
            println!("<- Leaving block");
        }
    }

    iter_block(tok, &tokens_map);

    println!("{:?}", tokens_map);
}
