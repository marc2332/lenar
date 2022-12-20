mod tokenizer {
    pub use slotmap::{DefaultKey, SlotMap};

    /// `Tokenizer` transforms an input, e.g a string, into a a Tokens map
    #[derive(Debug)]
    pub struct Tokenizer {
        tokens: SlotMap<DefaultKey, Token>,
        global_block: DefaultKey,
    }

    impl Tokenizer {
        pub fn from_str(code: &str) -> Self {
            let mut tokens = SlotMap::new();

            let global_block_token = Token::Block { tokens: Vec::new() };
            let global_block = tokens.insert(global_block_token);
            let mut block_indexes = vec![global_block];

            let mut i = 0;

            loop {
                let val = code.chars().nth(i);

                if val.is_none() {
                    break;
                }

                let val = val.unwrap();

                let current_block = *block_indexes.last().unwrap();

                if val == '{' {
                    let block = Token::Block { tokens: Vec::new() };
                    let block_key = tokens.insert(block);
                    block_indexes.push(block_key);
                    let current_block = tokens.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    i += 1;
                    continue;
                }

                if val == '}' {
                    block_indexes.pop();
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
                            !(v == until && block_indexes == 0 && in_string == false)
                        })
                        .map(|(_, e)| e)
                        .collect::<String>()
                };

                if slice_with_size(i, i + 2, code) == "var" {
                    let var_name = slice_until(i + 3, '=', code);
                    println!("VAR NAME {}", var_name);
                    let var_val = slice_until(i + 3 + var_name.len(), ';', code);
                    // TODO: Create some kind of missing tasks backpack 
                    println!("VAR VAL {}", var_val);
                    let var_def = Token::VarDef;
                    let var_key = tokens.insert(var_def);
                    let current_block = tokens.get_mut(current_block).unwrap();
                    current_block.add_token(var_key);
                    i += 3 + var_name.len();
                }

                i += 1;
            }

            Self {
                tokens,
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

    #[derive(Debug)]
    pub enum Token {
        Block { tokens: Vec<DefaultKey> },
        VarDef,
    }

    impl Token {
        pub fn add_token(&mut self, token: DefaultKey) {
            if let Token::Block { tokens } = self {
                tokens.push(token);
            }
        }
    }
}

fn main() {
    use tokenizer::*;

    let code = r#"
        var test = {"ja{j{};ajajj"};
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
            println!("inside of block");
            for tok_id in tokens {
                let tok = tokens_map.get_token(*tok_id).unwrap();
                match tok {
                    Token::Block { .. } => {
                        iter_block(tok, tokens_map);
                    }
                    Token::VarDef => {
                        println!("var def");
                    }
                }
            }
            println!("leaving block");
        }
    }

    iter_block(tok, &tokens_map);

    println!("{:?}", tokens_map);
}
