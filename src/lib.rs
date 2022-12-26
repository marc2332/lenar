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
        BytesVal {
            value: Vec<u8>,
        },
        FunctionCall {
            fn_name: String,
            arguments: TokenKey,
        },
        VarRef {
            var_name: String,
        },
        PropertyRef {
            path: Vec<String>,
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
    fn slice_until_delimeter(code: &mut Chars) -> String {
        let until = [',', ';', ')'];
        code.take_while(|&v| !until.contains(&v))
            .collect::<String>()
    }

    #[inline(always)]
    fn find_pos_until_is_not_char(start: usize, until: char, code: &str) -> usize {
        let code = &code[start..];
        code.chars().take_while(|&v| v == until).count()
    }

    #[inline(always)]
    fn count_unexpected_between(start: usize, until: char, code: &str) -> usize {
        let code = &code[start..];
        code.chars()
            .take_while(|&v| v != until)
            .filter(|v| v.is_whitespace())
            .count()
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
        ReferencedVariable,
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
                        let string_val = Token::BytesVal {
                            value: code[i - string_count + 1..i]
                                .chars()
                                .collect::<String>()
                                .as_bytes()
                                .to_vec(),
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
                    // is a function call
                    if count_unexpected_between(i, '(', code) == 0 {
                        let item_name = slice_until('(', &mut chars);
                        let item_name = format!("{}{}", val, item_name);

                        let value_block = Token::Block { tokens: Vec::new() };
                        let block_key = tokens_map.insert(value_block);

                        let fn_def = Token::FunctionCall {
                            fn_name: item_name,
                            arguments: block_key,
                        };
                        let fn_key = tokens_map.insert(fn_def);

                        let current_block = tokens_map.get_mut(current_block).unwrap();
                        current_block.add_token(fn_key);

                        block_indexes.push(block_key);

                        last_action = PerfomedAction::CalledFunction;

                        continue;
                    } else if count_unexpected_between(i, '.', code) == 0 {
                        let attrs_path = slice_until_delimeter(&mut chars);
                        let attrs_path = format!("{}{}", val, attrs_path);
                        let path = attrs_path
                            .split('.')
                            .map(|v| v.to_string())
                            .collect::<Vec<String>>();

                        let var_ref = Token::PropertyRef { path };
                        let var_ref_key = tokens_map.insert(var_ref);

                        let current_block = tokens_map.get_mut(current_block).unwrap();
                        current_block.add_token(var_ref_key);

                        last_action = PerfomedAction::ReferencedVariable;

                        continue;
                    } else {
                        let item_name = slice_until_delimeter(&mut chars);
                        let item_name = format!("{}{}", val, item_name);

                        let var_ref = Token::VarRef {
                            var_name: item_name,
                        };
                        let var_ref_key = tokens_map.insert(var_ref);

                        let current_block = tokens_map.get_mut(current_block).unwrap();
                        current_block.add_token(var_ref_key);

                        last_action = PerfomedAction::ReferencedVariable;

                        continue;
                    }
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
    pub use core::slice::Iter;
    use std::fmt::Debug;
    use std::{
        collections::HashMap,
        io::{stdout, Write},
        rc::Rc,
    };

    use crate::tokenizer::{Token, Tokenizer};

    /// A interpreter given a Tokenizer
    pub struct VM {
        tokenizer: Tokenizer,
    }

    impl VM {
        pub fn new(tokenizer: Tokenizer) -> Self {
            Self { tokenizer }
        }

        /// Run the VM
        pub fn run(&self) {
            let mut context = Context::default();

            context.setup_globals();

            let global_token = self.tokenizer.get_global();
            let global_block = self.tokenizer.get_token(global_token);

            let tok = global_block.unwrap();

            compute_expr(tok, &self.tokenizer, &mut context, &[]);
        }
    }

    /// The primitive types used in the VM
    /// TODO Would be great if I could avoid using heap-allocated types such as String or Vec and
    /// instead use the equivalent Rust primitives, just like I do with `VMType::Bytes`
    #[derive(Debug, Clone)]
    pub enum VMType<'a> {
        List(Vec<VMType<'a>>),
        String(String),
        Bytes(&'a [u8]),
        Void,
        Instance(Rc<dyn VMInstance<'a>>),
    }

    pub trait VMInstance<'a>: Debug {
        fn get_props(&self, path: &mut Iter<String>) -> VMType<'a> {
            let prop = path.next();
            if let Some(prop) = prop {
                self.get_prop(prop)
            } else {
                VMType::Void
            }
        }
        fn get_prop(&self, prop: &str) -> VMType<'a>;
    }

    pub trait VMFunction {
        fn call(&mut self, _args: &[VMType]);

        // TODO could be interesting to add some metadata methods, such as name.
    }

    /// A thread context
    ///
    /// TODO
    /// - Implement bottom->top scope finding recursion, e.g, value resolvers as `call_function` or
    ///   `get_variable` need to find the called function's scope ID from the caller scope ID
    #[derive(Default)]
    pub struct Context<'a> {
        variables: HashMap<String, VMType<'a>>,
        functions: HashMap<String, Box<dyn VMFunction>>,
        scopes: HashMap<usize, Context<'a>>,
    }

    impl<'a> Context<'a> {
        /// Some builtins varibles and values defined in the global scope, such as `println()`
        pub fn setup_globals(&mut self) {
            #[derive(Debug)]
            struct LenarGlobal;

            impl<'a> VMInstance<'a> for LenarGlobal {
                fn get_prop(&self, prop: &str) -> VMType<'a> {
                    match prop {
                        "version" => VMType::Bytes("1.0.0".as_bytes()),
                        _ => VMType::Void,
                    }
                }
            }

            // `print()`
            struct PrintFunc;

            impl VMFunction for PrintFunc {
                fn call(&mut self, args: &[VMType]) {
                    for val in args {
                        if let VMType::Bytes(bts) = val {
                            stdout().write(bts).ok();
                        }
                    }
                    stdout().flush().ok();
                }
            }

            // println()
            struct PrintLnFunc;

            impl VMFunction for PrintLnFunc {
                fn call(&mut self, args: &[VMType]) {
                    for val in args {
                        if let VMType::Bytes(bts) = val {
                            stdout().write(bts).ok();
                        }
                    }
                    stdout().write("\n".as_bytes()).ok();
                    stdout().flush().ok();
                }
            }

            self.functions
                .insert("print".to_string(), Box::new(PrintFunc));
            self.functions
                .insert("println".to_string(), Box::new(PrintLnFunc));
            self.variables
                .insert("Lenar".to_string(), VMType::Instance(Rc::new(LenarGlobal)));
        }

        pub fn get_scope(&mut self, path: &mut Iter<usize>) -> &mut Context<'a> {
            let scope = path.next();

            if let Some(scope) = scope {
                self.scopes.get_mut(scope).unwrap().get_scope(path)
            } else {
                self
            }
        }

        /// Call a function given a name, a scope ID and arguments
        pub fn call_function(
            &mut self,
            name: impl AsRef<str>,
            scope_id: &[usize],
            args: &[VMType],
        ) -> VMType {
            let scope = self.get_scope(&mut scope_id.iter());

            let func = scope.functions.get_mut(name.as_ref());
            if let Some(func) = func {
                func.call(args)
            } else {
                panic!("Function '{}' is not defined in this scope.", name.as_ref());
            }

            VMType::Void
        }

        /// Define a variable with a given name and a value in the specified scope ID
        pub fn define_variable(
            &mut self,
            name: impl AsRef<str>,
            scope_id: &[usize],
            value: VMType<'a>,
        ) {
            let scope = self.get_scope(&mut scope_id.iter());
            scope.variables.insert(name.as_ref().to_string(), value);
        }

        /// Resolve a variable value given it's name and the caller scope ID
        pub fn get_variable(&mut self, name: impl AsRef<str>, scope_id: &[usize]) -> VMType<'a> {
            let scope = self.get_scope(&mut scope_id.iter());
            scope
                .variables
                .get(name.as_ref())
                .cloned()
                .unwrap_or(VMType::Void)
        }

        pub fn get_variable_by_path(
            &mut self,
            path: &'a [String],
            scope_id: &[usize],
        ) -> VMType<'a> {
            let mut path = path.iter();
            let scope = self.get_scope(&mut scope_id.iter());

            let var_holder = path.next().unwrap();

            if let Some(VMType::Instance(instance)) = scope.variables.get_mut(var_holder) {
                let instance = Rc::get_mut(instance).unwrap();
                instance.get_props(&mut path)
            } else {
                VMType::Void
            }
        }

        /// Define a variable with a given name and a value in the specified scope ID
        pub fn create_scope(&mut self, scope_path: &[usize], scope_id: usize) {
            let scope = self.get_scope(&mut scope_path.iter());
            let mut new_scope = Context::default();

            new_scope.setup_globals();
            scope.scopes.insert(scope_id, new_scope);
        }
    }

    /// Resolve an expression to a value
    fn compute_expr<'a>(
        token: &'a Token,
        tokens_map: &'a Tokenizer,
        context: &mut Context<'a>,
        scope_path: &[usize],
    ) -> VMType<'a> {
        match token {
            Token::Block { tokens } => {
                let mut next_scope_id = scope_path.last().copied().unwrap_or(0);

                for (i, tok) in tokens.iter().enumerate() {
                    let is_last = i == tokens.len() - 1;
                    let tok = tokens_map.get_token(*tok).unwrap();
                    let res = if matches!(tok, Token::Block { .. }) {
                        next_scope_id += 1;
                        context.create_scope(scope_path, next_scope_id);
                        let scope_path = &[scope_path, &[next_scope_id]].concat();
                        compute_expr(tok, tokens_map, context, scope_path)
                    } else {
                        compute_expr(tok, tokens_map, context, scope_path)
                    };

                    if is_last {
                        return res;
                    }
                }

                VMType::Void
            }
            Token::VarDef {
                var_name,
                block_value,
            } => {
                let value = tokens_map.get_token(*block_value).unwrap();
                let res = compute_expr(value, tokens_map, context, scope_path);

                context.define_variable(var_name, scope_path, res);

                VMType::Void
            }
            Token::FunctionCall { arguments, fn_name } => {
                let value = tokens_map.get_token(*arguments).unwrap();
                let mut args = Vec::new();
                if let Token::Block { tokens } = value {
                    for tok in tokens {
                        let tok = tokens_map.get_token(*tok).unwrap();
                        let res = compute_expr(tok, tokens_map, context, scope_path);

                        args.push(res);
                    }
                }

                context.call_function(fn_name, scope_path, &args);

                VMType::Void
            }
            Token::StringVal { value } => VMType::String(value.to_string()),
            Token::BytesVal { value } => VMType::Bytes(value),
            Token::VarRef { var_name } => context.get_variable(var_name, scope_path),
            Token::PropertyRef { path } => context.get_variable_by_path(path, scope_path),
        }
    }
}
