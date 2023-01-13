pub mod tokenizer {
    use std::{iter::Peekable, str::Chars};

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
        FnDef {
            arguments_block: TokenKey,
            block_value: TokenKey,
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
    fn slice_until(until: char, chars: &mut Peekable<Chars>) -> String {
        chars.take_while(|&v| v != until).collect::<String>()
    }

    #[inline(always)]
    fn slice_until_delimeter(chars: &mut Peekable<Chars>) -> String {
        let until = [',', ';', ')', '}'];
        let mut s = String::new();
        while let Some(c) = chars.next_if(|v| !until.contains(v)) {
            s.push_str(&c.to_string());
        }
        s
    }

    #[inline(always)]
    fn count_unexpected_between(start: usize, until: char, code: &str) -> usize {
        let code = &code[start..];
        code.chars()
            .take_while(|&v| v != until)
            .filter(|v| v.is_whitespace() || v == &';' || v == &')')
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

    #[derive(Clone, Copy, PartialEq)]
    enum BlockType {
        Generic,
        FuncCall,
        Value,
        FuncValue,
    }

    impl Tokenizer {
        pub fn new(code: &str) -> Self {
            let mut tokens_map = Slab::new();

            let global_block_token = Token::Block { tokens: Vec::new() };
            let global_block = tokens_map.insert(global_block_token);

            let mut block_indexes = vec![(global_block, BlockType::Generic)];
            let mut string_count = 0;
            let mut last_action = PerfomedAction::EnteredGlobalScope;

            let len = code.len();
            let mut chars = code.chars().peekable();

            fn advance_by(how_much: usize, chars: &mut Peekable<Chars>) {
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
                    continue;
                }

                let val = val.unwrap();

                let (current_block, current_block_type) = *block_indexes.last().unwrap();

                // TODO closing parenthesis should only close the last `arguments` block not an actual code block
                if val == ')' && string_count == 0 {
                    block_indexes.pop();
                    last_action = PerfomedAction::ClosedStatement;
                    continue;
                }

                // Check operator syntax
                if val == '=' && string_count == 0 {
                    if matches!(last_action, PerfomedAction::DefinedVariable) {
                        last_action = PerfomedAction::FoundOperator('=');
                    } else {
                        panic!("Syntax error: Operator '=' is used to define initial values to variables.")
                    }
                    continue;
                }

                // End a statement
                if val == ';' && string_count == 0 {
                    if BlockType::Value == current_block_type {
                        block_indexes.pop();
                    }

                    last_action = PerfomedAction::ClosedStatement;
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

                    block_indexes.push((block_key, BlockType::Generic));
                    let current_block = tokens_map.get_mut(current_block).unwrap();
                    current_block.add_token(block_key);

                    last_action = PerfomedAction::OpenedBlock;

                    continue;
                }

                // Closing a block
                if val == '}' && string_count == 0 {
                    block_indexes.pop();
                    if let Some((_, BlockType::FuncValue)) = block_indexes.last() {
                        block_indexes.pop();
                    }
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

                    block_indexes.push((block_key, BlockType::Value));

                    last_action = PerfomedAction::DefinedVariable;

                    continue;
                }

                if string_count > 0 {
                    string_count += 1;
                    continue;
                }

                if string_count == 0 {
                    // Functions
                    if count_unexpected_between(i, '(', code) == 0 {
                        let item_name = slice_until('(', &mut chars);
                        let item_name = format!("{}{}", val, item_name);

                        if item_name == "fn" {
                            let args_block = Token::Block { tokens: Vec::new() };
                            let args_block_key = tokens_map.insert(args_block);

                            let value_block = Token::Block { tokens: Vec::new() };
                            let block_key = tokens_map.insert(value_block);

                            let var_def = Token::FnDef {
                                block_value: block_key,
                                arguments_block: args_block_key,
                            };
                            let var_key = tokens_map.insert(var_def);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(var_key);

                            block_indexes.push((block_key, BlockType::FuncValue));
                            block_indexes.push((args_block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        } else {
                            let value_block = Token::Block { tokens: Vec::new() };
                            let block_key = tokens_map.insert(value_block);

                            let fn_def = Token::FunctionCall {
                                fn_name: item_name,
                                arguments: block_key,
                            };
                            let fn_key = tokens_map.insert(fn_def);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(fn_key);

                            block_indexes.push((block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        }

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

pub mod runtime {
    pub use core::slice::Iter;
    use std::cell::RefCell;
    use std::fmt::Debug;
    use std::fs::File;
    use std::io::Read;
    use std::str::from_utf8;
    use std::{
        collections::HashMap,
        io::{stdout, Write},
        rc::Rc,
    };

    use slab::Slab;

    use crate::tokenizer::{Token, Tokenizer};

    /// A interpreter given a Tokenizer
    pub struct Runtime {
        tokenizer: Tokenizer,
    }

    impl Runtime {
        pub fn new(tokenizer: Tokenizer) -> Self {
            Self { tokenizer }
        }

        /// Run the Runtime
        pub fn run(&self) {
            let mut context = Context::default();

            context.setup_globals();

            let global_token = self.tokenizer.get_global();
            let global_block = self.tokenizer.get_token(global_token);

            let tok = global_block.unwrap();

            compute_expr(tok, &self.tokenizer, &mut context, &[]);
        }
    }

    /// The primitive types used in the Runtime
    /// TODO Would be great if I could avoid using heap-allocated types such as String or Vec and
    /// instead use the equivalent Rust primitives, just like I do with `RuntimeType::Bytes`
    #[derive(Debug, Clone)]
    pub enum RuntimeType<'a> {
        Usize(usize),
        List(Vec<RuntimeType<'a>>),
        String(String),
        Str(&'a str),
        Bytes(&'a [u8]),
        OwnedBytes(Vec<u8>),
        Void,
        Instance(Rc<dyn RuntimeInstance<'a>>),
        Function(Rc<dyn RuntimeFunction>),
    }

    impl<'a> RuntimeType<'a> {
        pub fn as_list(&self) -> Option<&Vec<RuntimeType<'a>>> {
            if let Self::List(v) = self {
                Some(v)
            } else {
                None
            }
        }

        pub fn as_string(&self) -> Option<&String> {
            if let Self::String(v) = self {
                Some(v)
            } else {
                None
            }
        }

        pub fn as_bytes(&self) -> Option<&[u8]> {
            match self {
                Self::OwnedBytes(v) => Some(v),
                Self::Bytes(v) => Some(v),
                _ => None,
            }
        }

        pub fn as_void(&self) -> Option<()> {
            if let Self::Void = self {
                Some(())
            } else {
                None
            }
        }

        pub fn as_instance(&self) -> Option<&Rc<dyn RuntimeInstance<'a>>> {
            if let Self::Instance(v) = self {
                Some(v)
            } else {
                None
            }
        }
    }

    pub trait RuntimeInstance<'a>: Debug {
        fn get_props(&self, path: &mut Iter<String>) -> RuntimeType<'a> {
            let prop = path.next();
            if let Some(prop) = prop {
                self.get_prop(prop)
            } else {
                RuntimeType::Void
            }
        }
        fn get_prop(&self, prop: &str) -> RuntimeType<'a>;
    }

    pub trait RuntimeFunction: Debug {
        fn call<'s>(
            &mut self,
            _args: Vec<RuntimeType<'s>>,
            tokens_map: &'s Tokenizer,
        ) -> RuntimeType<'s>;

        // TODO could be interesting to add some metadata methods, such as name.
    }

    /// A thread context
    ///
    /// TODO
    /// - Implement bottom->top scope finding recursion, e.g, value resolvers as `call_function` or
    ///   `get_variable` need to find the called function's scope ID from the caller scope ID
    #[derive(Default)]
    pub struct Context<'a> {
        variables: HashMap<String, RuntimeType<'a>>,
        scopes: HashMap<usize, Context<'a>>,
    }

    impl<'a> Context<'a> {
        /// Some builtins varibles and values defined in the global scope, such as `println()`
        pub fn setup_globals(&mut self) {
            let resources_files = Rc::new(RefCell::new(Slab::<File>::new()));

            #[derive(Debug)]
            struct ToStringFunc {
                resources_files: Rc<RefCell<Slab<File>>>,
            }

            impl ToStringFunc {
                pub fn new(resources_files: Rc<RefCell<Slab<File>>>) -> Self {
                    Self { resources_files }
                }
            }

            impl RuntimeFunction for ToStringFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<RuntimeType<'s>>,
                    _tokens_map: &'s Tokenizer,
                ) -> RuntimeType<'s> {
                    match args[0] {
                        RuntimeType::Usize(rid) => {
                            let resources_files = self.resources_files.borrow_mut();
                            let mut file = resources_files.get(rid).unwrap();
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();
                            RuntimeType::OwnedBytes(buf)
                        }
                        _ => RuntimeType::Void,
                    }
                }
            }

            #[derive(Debug)]
            struct OpenFileFunc {
                resources_files: Rc<RefCell<Slab<File>>>,
            }

            impl OpenFileFunc {
                pub fn new(resources_files: Rc<RefCell<Slab<File>>>) -> Self {
                    Self { resources_files }
                }
            }

            impl RuntimeFunction for OpenFileFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<RuntimeType<'s>>,
                    _tokens_map: &'s Tokenizer,
                ) -> RuntimeType<'s> {
                    let file_path = args[0].as_bytes().unwrap();
                    let file_path = from_utf8(file_path).unwrap();
                    let file = File::open(file_path).unwrap();

                    let mut resources_files = self.resources_files.borrow_mut();
                    let rid = resources_files.insert(file);

                    RuntimeType::Usize(rid)
                }
            }

            #[derive(Debug)]
            struct LenarGlobal;

            impl<'a> RuntimeInstance<'a> for LenarGlobal {
                fn get_prop(&self, prop: &str) -> RuntimeType<'a> {
                    match prop {
                        "version" => RuntimeType::Bytes("1.0.0".as_bytes()),
                        _ => RuntimeType::Void,
                    }
                }
            }

            // `print()`
            #[derive(Debug)]
            struct PrintFunc;

            impl RuntimeFunction for PrintFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<RuntimeType<'s>>,
                    _tokens_map: &'s Tokenizer,
                ) -> RuntimeType<'s> {
                    for val in args {
                        if let Some(bts) = val.as_bytes() {
                            stdout().write(bts).ok();
                        }
                    }
                    stdout().flush().ok();
                    RuntimeType::Void
                }
            }

            // println()
            #[derive(Debug)]
            struct PrintLnFunc;

            impl RuntimeFunction for PrintLnFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<RuntimeType<'s>>,
                    _tokens_map: &'s Tokenizer,
                ) -> RuntimeType<'s> {
                    for val in args {
                        if let Some(bts) = val.as_bytes() {
                            stdout().write(bts).ok();
                        }
                    }
                    stdout().write("\n".as_bytes()).ok();
                    stdout().flush().ok();
                    RuntimeType::Void
                }
            }

            self.variables.insert(
                "toString".to_string(),
                RuntimeType::Function(Rc::new(ToStringFunc::new(resources_files.clone()))),
            );
            self.variables.insert(
                "openFile".to_string(),
                RuntimeType::Function(Rc::new(OpenFileFunc::new(resources_files))),
            );
            self.variables.insert(
                "print".to_string(),
                RuntimeType::Function(Rc::new(PrintFunc)),
            );
            self.variables.insert(
                "println".to_string(),
                RuntimeType::Function(Rc::new(PrintLnFunc)),
            );
            self.variables.insert(
                "Lenar".to_string(),
                RuntimeType::Instance(Rc::new(LenarGlobal)),
            );
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
            args: Vec<RuntimeType<'a>>,
            tokens_map: &'a Tokenizer,
        ) -> RuntimeType<'a> {
            let scope = self.get_scope(&mut scope_id.iter());
            let func = scope.variables.get_mut(name.as_ref());

            if let Some(RuntimeType::Function(func)) = func {
                let func = Rc::get_mut(func).unwrap();
                func.call(args, tokens_map)
            } else {
                panic!("Function '{}' is not defined in this scope.", name.as_ref());
            }
        }

        /// Define a variable with a given name and a value in the specified scope ID
        pub fn define_variable(
            &mut self,
            name: impl AsRef<str>,
            scope_id: &[usize],
            value: RuntimeType<'a>,
        ) {
            let scope = self.get_scope(&mut scope_id.iter());
            scope.variables.insert(name.as_ref().to_string(), value);
        }

        /// Resolve a variable value given it's name and the caller scope ID
        pub fn get_variable(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
        ) -> RuntimeType<'a> {
            let var = self.variables.get(name.as_ref());

            if let Some(var) = var {
                var.clone()
            } else {
                let scope = path.next();
                if let Some(scope) = scope {
                    self.scopes.get_mut(scope).unwrap().get_variable(name, path)
                } else {
                    RuntimeType::Void
                }
            }
        }

        pub fn get_variable_by_path(
            &mut self,
            path: &'a [String],
            scope_id: &[usize],
        ) -> RuntimeType<'a> {
            let mut path = path.iter();
            let scope = self.get_scope(&mut scope_id.iter());

            let var_holder = path.next().unwrap();

            if let Some(RuntimeType::Instance(instance)) = scope.variables.get_mut(var_holder) {
                let instance = Rc::get_mut(instance).unwrap();
                instance.get_props(&mut path)
            } else {
                RuntimeType::Void
            }
        }

        /// Create a new scope given an ID in the specified scope by a path
        pub fn create_scope(&mut self, scope_path: &[usize], scope_id: usize) {
            let scope = self.get_scope(&mut scope_path.iter());
            let mut new_scope = Context::default();

            new_scope.setup_globals();
            scope.scopes.insert(scope_id, new_scope);
        }

        /// Drop a scope given an ID and a scope path
        pub fn drop_scope(&mut self, scope_path: &[usize], scope_id: usize) {
            let scope = self.get_scope(&mut scope_path.iter());
            scope.scopes.remove(&scope_id);
        }
    }

    /// Resolve an expression to a value
    fn compute_expr<'a>(
        token: &'a Token,
        tokens_map: &'a Tokenizer,
        context: &mut Context<'a>,
        scope_path: &[usize],
    ) -> RuntimeType<'a> {
        match token {
            Token::Block { tokens } => {
                let mut next_scope_id = scope_path.last().copied().unwrap_or(0);

                for (i, tok) in tokens.iter().enumerate() {
                    let is_last = i == tokens.len() - 1;
                    let tok = tokens_map.get_token(*tok).unwrap();
                    let res = if matches!(tok, Token::Block { .. }) {
                        next_scope_id += 1;
                        // Create block scope
                        context.create_scope(scope_path, next_scope_id);

                        // Run the block expression in the new scope
                        let scope_path = &[scope_path, &[next_scope_id]].concat();
                        let return_val = compute_expr(tok, tokens_map, context, scope_path);

                        // Remove the scope
                        context.drop_scope(scope_path, next_scope_id);
                        return_val
                    } else {
                        // Run the expression in the inherited scope
                        compute_expr(tok, tokens_map, context, scope_path)
                    };

                    // Return the returned value from the expressin as result of this block
                    if is_last {
                        return res;
                    }
                }

                RuntimeType::Void
            }
            Token::VarDef {
                var_name,
                block_value,
            } => {
                let value = tokens_map.get_token(*block_value).unwrap();
                let res = compute_expr(value, tokens_map, context, scope_path);
                context.define_variable(var_name, scope_path, res);

                RuntimeType::Void
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

                context.call_function(fn_name, scope_path, args, tokens_map)
            }
            Token::StringVal { value } => RuntimeType::Str(value),
            Token::BytesVal { value } => RuntimeType::Bytes(value),
            Token::VarRef { var_name } => context.get_variable(var_name, &mut scope_path.iter()),
            Token::PropertyRef { path } => context.get_variable_by_path(path, scope_path),
            Token::FnDef {
                arguments_block,
                block_value,
            } => {
                // User-defined function
                #[derive(Debug)]
                struct Function {
                    arguments_block: usize,
                    block_value: usize,
                }

                impl RuntimeFunction for Function {
                    fn call<'s>(
                        &mut self,
                        mut args: Vec<RuntimeType<'s>>,
                        tokens_map: &'s Tokenizer,
                    ) -> RuntimeType<'s> {
                        let mut context = Context::default();

                        context.setup_globals();

                        let arguments_block = tokens_map.get_token(self.arguments_block).unwrap();
                        if let Token::Block { tokens } = arguments_block {
                            for token in tokens {
                                let arg_token = tokens_map.get_token(*token).unwrap();
                                if let Token::VarRef { var_name } = arg_token {
                                    let arg_value = args.remove(0);
                                    context.variables.insert(var_name.to_owned(), arg_value);
                                }
                            }
                        }

                        let block_token = tokens_map.get_token(self.block_value).unwrap();

                        compute_expr(block_token, tokens_map, &mut context, &[])
                    }
                }
                RuntimeType::Function(Rc::new(Function {
                    arguments_block: *arguments_block,
                    block_value: *block_value,
                }))
            }
        }
    }
}
