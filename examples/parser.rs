use lenar::tokenizer;

fn main() {
    use tokenizer::*;

    let code = r#"
        let test = hola("1" "wooow");
        {
            let test2 = yo("2" "hola");
            alright("
                ok
            ")
        }
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
                            let val = tokens.last().unwrap();
                            let val = tokens_map.get_token(*val);
                            if let Some(val) = val {
                                match val {
                                    Token::BytesVal { value } => {
                                        println!(
                                            "DEF: Variable <{}> has value of {:?}",
                                            var_name, value
                                        );
                                    }
                                    Token::FunctionCall { fn_name, arguments } => {
                                        let value = tokens_map.get_token(*arguments).unwrap();
                                        if let Token::Block { tokens } = value {
                                            let mut var_vals = Vec::new();
                                            for val in tokens {
                                                let val = tokens_map.get_token(*val);
                                                if let Some(val) = val {
                                                    if let Token::BytesVal { value } = val {
                                                        var_vals.push(
                                                            String::from_utf8(value.to_vec())
                                                                .unwrap(),
                                                        )
                                                    }
                                                }
                                            }
                                            println!("DEF: Variable <{}> with value of calling {:?} with arguments {:?}", var_name, fn_name, var_vals);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Token::FunctionCall { arguments, fn_name } => {
                        let value = tokens_map.get_token(*arguments).unwrap();
                        if let Token::Block { tokens } = value {
                            let mut var_vals = Vec::new();
                            for val in tokens {
                                let val = tokens_map.get_token(*val);
                                if let Some(val) = val {
                                    if let Token::BytesVal { value } = val {
                                        var_vals.push(String::from_utf8(value.to_vec()).unwrap())
                                    }
                                }
                            }

                            println!(
                                "CALL: Function call <{}> with arguments {:?}",
                                fn_name, var_vals
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
