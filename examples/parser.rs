use lenar::tokenizer;

fn main() {
    use tokenizer::*;

    let code = r#"
        var test = "test";
        { { } }
        { }
        { { { var hola = "wow"; } } }
        { }
    "#;

    let tokens_map = Tokenizer::new(&code);

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
                    Token::VarDef {
                        block_value,
                        var_name,
                    } => {
                        let value = tokens_map.get_token(*block_value).unwrap();
                        if let Token::Block { tokens } = value {
                            let mut var_val = None;
                            let val = tokens.last().unwrap();
                            let val = tokens_map.get_token(*val);
                            if let Some(val) = val {
                                if let Token::StringVal { value } = val {
                                    var_val = Some(value);
                                }
                            }
                            println!("== Variable <{}> has value of {:?}", var_name, var_val);
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
