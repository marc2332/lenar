
mod tokenizer {
    use slotmap::{SlotMap, DefaultKey};

    /// `Tokenizer` transforms an input, e.g a string, into a a Tokens map
    #[derive(Debug)]
    pub struct Tokenizer {
        tokens: SlotMap<DefaultKey, Token>
    }

    impl Tokenizer {
        pub fn from_str(code: &str) -> Self {

            let mut tokens = SlotMap::new();

            let (mut block_indexes,) = (Vec::new(),);       
            
            for val in code.chars() {
                if val == '{' {
                    let block = Token::Block {
                        tokens: Vec::new()
                    };
                    let block_key = tokens.insert(block);
                    block_indexes.push(block_key);
                }

                if val == '}' {
                    block_indexes.pop();
                }

                let current_block = block_indexes.last();
            
                if let Some(current_block) = current_block {
                    
                }
            }

            Self {
                tokens
            }
        }

        pub fn get_map(&self) -> &SlotMap<DefaultKey, Token> {
            &self.tokens
        }
    }

    #[derive(Debug)]
    enum Token {
        Block {
            tokens: Vec<Token>   
        }        
    }
}

fn main() {
    use tokenizer::Tokenizer;

    let code = r#"main {
        { { { } } }
    }"#;

    let tokens = Tokenizer::from_str(code);

    println!("{:?}", tokens);
}
