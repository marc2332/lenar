pub mod tokenizer {
    use std::{iter::Peekable, str::Chars, sync::Arc};

    pub use slab::Slab;

    pub type TokenKey = usize;

    /// `Tokenizer` transforms an input, e.g a string, into a a Tokens map
    #[derive(Debug, Clone)]
    pub struct Tokenizer {
        tokens: Slab<Token>,
        global_block: TokenKey,
    }

    #[derive(Debug, Clone)]
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
        IfDef {
            condition_block: TokenKey,
            block_value: TokenKey,
        },
        NumberVal {
            value: usize,
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
        let until = [',', ';', ')', '}', ' ', '\n'];
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
        FoundNumber,
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

            let mut tokenizer = Self {
                tokens: tokens_map,
                global_block,
            };

            tokenizer.parse(code);

            tokenizer
        }

        pub fn wrap(self) -> Arc<Self> {
            Arc::new(self)
        }

        pub fn parse(&mut self, code: &str) {
            let tokens_map = &mut self.tokens;
            let global_block = self.global_block;

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
                        let item_name = format!("{val}{item_name}");

                        if item_name == "if" {
                            let expr_block = Token::Block { tokens: Vec::new() };
                            let expr_block_key = tokens_map.insert(expr_block);

                            let value_block = Token::Block { tokens: Vec::new() };
                            let block_key = tokens_map.insert(value_block);

                            let if_def = Token::IfDef {
                                block_value: block_key,
                                condition_block: expr_block_key,
                            };
                            let if_key = tokens_map.insert(if_def);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(if_key);

                            block_indexes.push((block_key, BlockType::FuncValue));
                            block_indexes.push((expr_block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        } else if item_name == "fn" {
                            let args_block = Token::Block { tokens: Vec::new() };
                            let args_block_key = tokens_map.insert(args_block);

                            let value_block = Token::Block { tokens: Vec::new() };
                            let block_key = tokens_map.insert(value_block);

                            let fn_def = Token::FnDef {
                                block_value: block_key,
                                arguments_block: args_block_key,
                            };
                            let fn_key = tokens_map.insert(fn_def);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(fn_key);

                            block_indexes.push((block_key, BlockType::FuncValue));
                            block_indexes.push((args_block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        } else {
                            let value_block = Token::Block { tokens: Vec::new() };
                            let block_key = tokens_map.insert(value_block);

                            let fn_call_def = Token::FunctionCall {
                                fn_name: item_name,
                                arguments: block_key,
                            };
                            let fn_call_key = tokens_map.insert(fn_call_def);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(fn_call_key);

                            block_indexes.push((block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        }

                        continue;
                    } else if count_unexpected_between(i, '.', code) == 0 {
                        let attrs_path = slice_until_delimeter(&mut chars);
                        let attrs_path = format!("{val}{attrs_path}");
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
                    } else if val.is_ascii_digit() {
                        let item_val = slice_until_delimeter(&mut chars);
                        let item_val = format!("{val}{item_val}");

                        if let Ok(value) = item_val.parse::<usize>() {
                            let number_val = Token::NumberVal { value };

                            let number_val_key = tokens_map.insert(number_val);

                            let current_block = tokens_map.get_mut(current_block).unwrap();
                            current_block.add_token(number_val_key);

                            last_action = PerfomedAction::FoundNumber;
                        }

                        continue;
                    } else {
                        let item_name = slice_until_delimeter(&mut chars);
                        let item_name = format!("{val}{item_name}");

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
    use std::fmt::{Debug, Display};
    use std::fs::File;
    use std::io::Read;
    use std::str::from_utf8;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;
    use std::{
        collections::HashMap,
        io::{stdout, Write},
        rc::Rc,
    };

    use slab::Slab;

    use crate::tokenizer::{Token, Tokenizer};

    /// A interpreter given a Tokenizer
    pub struct Runtime;

    impl Runtime {
        pub fn run_with_scope<'a>(
            context: &mut Scope<'a>,
            tokenizer: &'a Arc<Tokenizer>,
        ) -> LenarValue<'a> {
            let global_block = tokenizer.get_token(tokenizer.get_global()).unwrap();
            evaluate_expression(global_block, tokenizer, context, &[])
        }

        /// Evaluate the runtime code and return the exit value
        pub fn evaluate(tokenizer: &Arc<Tokenizer>) -> LenarValue {
            let mut context = Scope::default();
            context.setup_globals();

            Self::run_with_scope(&mut context, tokenizer)
        }

        pub fn run(code: &str) {
            let tokenizer = Arc::new(Tokenizer::new(code));
            Self::evaluate(&tokenizer);
        }
    }

    /// Runtime values
    #[derive(Debug, Clone)]
    pub enum LenarValue<'a> {
        Usize(usize),
        List(Vec<LenarValue<'a>>),
        Str(&'a str),
        Bytes(&'a [u8]),
        OwnedBytes(Vec<u8>),
        Void,
        Bool(bool),
        Instance(Rc<RefCell<dyn RuntimeInstance<'a>>>),
        Function(Rc<dyn RuntimeFunction>),
        Enum(LenarEnum<'a>),
        Ref(Rc<RefCell<LenarValue<'a>>>),
    }

    #[derive(Debug, Clone, Default)]
    pub struct LenarEnum<'a>(HashMap<&'a str, LenarValue<'a>>);

    impl<'a> LenarEnum<'a> {
        pub fn new_with_variant(variant_name: &'a str, variant_value: LenarValue<'a>) -> Self {
            let mut en = LenarEnum::default();
            en.0.insert(variant_name, variant_value);
            en
        }

        pub fn peek_variant(&self, variant_name: &str) -> Option<&LenarValue<'a>> {
            self.0.get(variant_name)
        }

        pub fn get_variant(mut self, variant_name: &str) -> Option<LenarValue<'a>> {
            self.0.remove(variant_name)
        }
    }

    impl Display for LenarEnum<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(
                &self
                    .0
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        }
    }

    impl Display for LenarValue<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LenarValue::Usize(u) => f.write_str(&format!("{u}")),
                LenarValue::List(l) => f
                    .debug_map()
                    .value(&l.iter().map(|v| format!("{v}")))
                    .finish(),
                LenarValue::Str(s) => f.write_str(s),
                LenarValue::Bytes(b) => f.write_str(from_utf8(b).unwrap()),
                LenarValue::OwnedBytes(b) => f.write_str(from_utf8(b).unwrap()),
                LenarValue::Void => f.write_str("Void"),
                LenarValue::Bool(b) => f.write_str(&format!("{b}")),
                LenarValue::Instance(i) => f.write_str(i.borrow().get_name()),
                LenarValue::Function(func) => f.write_str(func.get_name()),
                LenarValue::Enum(en) => f.write_str(&en.to_string()),
                LenarValue::Ref(r) => f.write_str(&r.borrow().to_string()),
            }
        }
    }

    impl PartialEq for LenarValue<'_> {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Usize(l0), Self::Usize(r0)) => l0 == r0,
                (Self::List(l0), Self::List(r0)) => l0 == r0,
                (Self::Str(l0), Self::Str(r0)) => l0 == r0,
                (Self::Bytes(l0), Self::Bytes(r0)) => l0 == r0,
                (Self::OwnedBytes(l0), Self::OwnedBytes(r0)) => l0 == r0,
                (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
                (Self::Instance(_), Self::Instance(_)) => false,
                (Self::Function(_), Self::Function(_)) => false,
                (Self::Void, Self::Void) => true,
                _ => false,
            }
        }
    }

    impl<'a> LenarValue<'a> {
        pub fn is_void(&self) -> bool {
            matches!(self, Self::Void)
        }

        pub fn as_bytes(&self) -> Option<&[u8]> {
            match self {
                Self::OwnedBytes(v) => Some(v),
                Self::Bytes(v) => Some(v),
                _ => None,
            }
        }

        pub fn as_integer_mut(&mut self) -> Option<&mut usize> {
            match self {
                Self::Usize(v) => Some(v),
                _ => None,
            }
        }

        pub fn as_integer(&self) -> Option<&usize> {
            match self {
                Self::Usize(v) => Some(v),
                _ => None,
            }
        }
    }

    pub trait RuntimeInstance<'a>: Debug {
        fn get_props(&self, path: &mut Iter<String>) -> LenarValue<'a> {
            let prop = path.next();
            if let Some(prop) = prop {
                self.get_prop(prop)
            } else {
                LenarValue::Void
            }
        }

        fn get_prop(&self, prop: &str) -> LenarValue<'a>;

        fn get_name<'s>(&self) -> &'s str;
    }

    pub trait RuntimeFunction: Debug {
        fn call<'s>(
            &mut self,
            _args: Vec<LenarValue<'s>>,
            tokens_map: &'s Arc<Tokenizer>,
        ) -> LenarValue<'s>;

        fn get_name<'s>(&self) -> &'s str;
    }

    /// Runtime Scope that includes variables and nested Scopes.
    #[derive(Default)]
    pub struct Scope<'a> {
        locks: Arc<Mutex<Slab<JoinHandle<()>>>>,
        variables: HashMap<String, LenarValue<'a>>,
        scopes: HashMap<usize, Scope<'a>>,
    }

    impl<'a> Scope<'a> {
        /// Add an instance to the global scope
        pub fn add_global_instance(&mut self, val: impl RuntimeInstance<'a> + 'static) {
            self.variables.insert(
                val.get_name().to_owned(),
                LenarValue::Instance(Rc::new(RefCell::new(val))),
            );
        }

        /// Add a function to the global scope
        pub fn add_global_function(&mut self, val: impl RuntimeFunction + 'static) {
            self.variables.insert(
                val.get_name().to_owned(),
                LenarValue::Function(Rc::new(val)),
            );
        }

        /// Define global utilities
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
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    match args[0] {
                        LenarValue::Usize(rid) => {
                            let resources_files = self.resources_files.borrow_mut();
                            let mut file = resources_files.get(rid).unwrap();
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();
                            LenarValue::OwnedBytes(buf)
                        }
                        _ => LenarValue::Void,
                    }
                }

                fn get_name<'s>(&self) -> &'s str {
                    "toString"
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
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let file_path = args[0].as_bytes().unwrap();
                    let file_path = from_utf8(file_path).unwrap();
                    let file = File::open(file_path).unwrap();

                    let mut resources_files = self.resources_files.borrow_mut();
                    let rid = resources_files.insert(file);

                    LenarValue::Usize(rid)
                }

                fn get_name<'s>(&self) -> &'s str {
                    "openFile"
                }
            }

            #[derive(Debug)]
            struct LenarGlobal;

            impl<'a> RuntimeInstance<'a> for LenarGlobal {
                fn get_prop(&self, prop: &str) -> LenarValue<'a> {
                    match prop {
                        "version" => LenarValue::Bytes("1.0.0".as_bytes()),
                        _ => LenarValue::Void,
                    }
                }

                fn get_name<'s>(&self) -> &'s str {
                    "Lenar"
                }
            }

            // `print()`
            #[derive(Debug)]
            struct PrintFunc;

            impl PrintFunc {
                pub fn write(value: &LenarValue) {
                    match value {
                        LenarValue::OwnedBytes(bts) => {
                            stdout().write(bts).ok();
                        }
                        LenarValue::Bytes(bts) => {
                            stdout().write(bts).ok();
                        }
                        LenarValue::Function(func) => {
                            stdout().write(func.get_name().as_bytes()).ok();
                        }
                        LenarValue::Instance(instance) => {
                            stdout().write(instance.borrow().get_name().as_bytes()).ok();
                        }
                        LenarValue::Bool(b) => {
                            stdout().write(b.to_string().as_bytes()).ok();
                        }
                        LenarValue::Usize(n) => {
                            stdout().write(n.to_string().as_bytes()).ok();
                        }
                        LenarValue::Str(s) => {
                            stdout().write(s.as_bytes()).ok();
                        }
                        LenarValue::List(l) => {
                            l.iter().for_each(Self::write);
                        }
                        LenarValue::Void => {
                            stdout().write("Void".as_bytes()).ok();
                        }
                        LenarValue::Enum(en) => {
                            stdout().write(en.to_string().as_bytes()).ok();
                        }
                        LenarValue::Ref(r) => {
                            stdout().write(r.borrow().to_string().as_bytes()).ok();
                        }
                    }
                }
            }

            impl RuntimeFunction for PrintFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    for val in args {
                        Self::write(&val);
                    }
                    stdout().flush().ok();
                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "print"
                }
            }

            // println()
            #[derive(Debug)]
            struct PrintLnFunc;

            impl RuntimeFunction for PrintLnFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    for val in args {
                        PrintFunc::write(&val);
                    }
                    stdout().write("\n".as_bytes()).ok();
                    stdout().flush().ok();
                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "println"
                }
            }

            // isEqual()
            #[derive(Debug)]
            struct IsEqual;

            impl RuntimeFunction for IsEqual {
                fn call<'s>(
                    &mut self,
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let args = args.get(0).zip(args.get(1));
                    let res = if let Some((a, b)) = args {
                        a.eq(b)
                    } else {
                        false
                    };
                    LenarValue::Bool(res)
                }

                fn get_name<'s>(&self) -> &'s str {
                    "isEqual"
                }
            }

            // NewList()
            #[derive(Debug)]
            struct NewListFunc;

            impl RuntimeFunction for NewListFunc {
                fn call<'s>(
                    &mut self,
                    args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    LenarValue::List(args)
                }

                fn get_name<'s>(&self) -> &'s str {
                    "newList"
                }
            }

            // iter()
            #[derive(Debug)]
            struct IterFunc {
                resources_files: Rc<RefCell<Slab<File>>>,
            }

            impl IterFunc {
                pub fn new(resources_files: Rc<RefCell<Slab<File>>>) -> Self {
                    Self { resources_files }
                }
            }

            impl RuntimeFunction for IterFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let iterator = args.remove(0);
                    let fun = args.remove(0);

                    if let LenarValue::Function(mut fun) = fun {
                        let fun = Rc::get_mut(&mut fun).unwrap();
                        match iterator {
                            LenarValue::Usize(rid) => {
                                let resources_files = self.resources_files.borrow_mut();
                                let file = resources_files.get(rid).unwrap();
                                let bytes = file.bytes();

                                for byte in bytes {
                                    if let Ok(byte) = byte {
                                        fun.call(vec![LenarValue::Bytes(&[byte])], _tokens_map);
                                    } else {
                                        break;
                                    }
                                }
                            }
                            LenarValue::Bytes(bytes) => {
                                for byte in bytes {
                                    fun.call(vec![LenarValue::Bytes(&[*byte])], _tokens_map);
                                }
                            }
                            LenarValue::OwnedBytes(bytes) => {
                                for byte in bytes {
                                    fun.call(vec![LenarValue::Bytes(&[byte])], _tokens_map);
                                }
                            }
                            LenarValue::List(items) => {
                                for (i, item) in items.into_iter().enumerate() {
                                    fun.call(vec![item, LenarValue::Usize(i)], _tokens_map);
                                }
                            }
                            _ => {}
                        }
                    }

                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "iter"
                }
            }

            // thread()
            #[derive(Debug)]
            struct ThreadFunc;

            impl RuntimeFunction for ThreadFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let fun = args.remove(0);

                    if let LenarValue::Function(mut fun) = fun {
                        let fun = Rc::get_mut(&mut fun).unwrap();
                        fun.call(args, tokens_map);
                    }

                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "thread"
                }
            }

            // sleep()
            #[derive(Debug)]
            struct SleepFunc;

            impl RuntimeFunction for SleepFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    if let LenarValue::Usize(time) = v {
                        thread::sleep(Duration::from_millis(time as u64));
                    }
                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "sleep"
                }
            }

            // wait()
            #[derive(Debug)]
            struct WaitFunc(Arc<Mutex<Slab<JoinHandle<()>>>>);

            impl WaitFunc {
                pub fn new(locks: Arc<Mutex<Slab<JoinHandle<()>>>>) -> Self {
                    Self(locks)
                }
            }

            impl RuntimeFunction for WaitFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    if let LenarValue::Usize(rid) = v {
                        let handle = self.0.lock().unwrap().remove(rid);
                        handle.join().unwrap();
                    }
                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "wait"
                }
            }

            // Ok()
            #[derive(Debug)]
            struct OkFunc;

            impl RuntimeFunction for OkFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    LenarValue::Enum(LenarEnum::new_with_variant("Ok", v))
                }

                fn get_name<'s>(&self) -> &'s str {
                    "Ok"
                }
            }

            // Err()
            #[derive(Debug)]
            struct ErrFunc;

            impl RuntimeFunction for ErrFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    LenarValue::Enum(LenarEnum::new_with_variant("Err", v))
                }

                fn get_name<'s>(&self) -> &'s str {
                    "Err"
                }
            }

            // isOk()
            #[derive(Debug)]
            struct IsOkFunc;

            impl RuntimeFunction for IsOkFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    match v {
                        LenarValue::Enum(variants) => {
                            let ok_variant = variants.peek_variant("Ok");
                            LenarValue::Bool(ok_variant.is_some())
                        }
                        _ => LenarValue::Bool(false),
                    }
                }

                fn get_name<'s>(&self) -> &'s str {
                    "isOk"
                }
            }

            // unwrap()
            #[derive(Debug)]
            struct UnwrapFunc;

            impl RuntimeFunction for UnwrapFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    match v {
                        LenarValue::Enum(variants) => {
                            let variant = variants.get_variant("Ok");
                            variant.unwrap_or_else(|| panic!("Unwrapped a <Err> value."))
                        }
                        _ => LenarValue::Void,
                    }
                }

                fn get_name<'s>(&self) -> &'s str {
                    "unwrap"
                }
            }

            // unwrapErr()
            #[derive(Debug)]
            struct UnwrapErrFunc;

            impl RuntimeFunction for UnwrapErrFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    match v {
                        LenarValue::Enum(variants) => {
                            let variant = variants.get_variant("Err");
                            variant.unwrap_or_else(|| panic!("Unwrapped a <Ok> value."))
                        }
                        _ => LenarValue::Void,
                    }
                }

                fn get_name<'s>(&self) -> &'s str {
                    "unwrapErr"
                }
            }

            // ref()
            #[derive(Debug)]
            struct RefFunc;

            impl RuntimeFunction for RefFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let v = args.remove(0);
                    LenarValue::Ref(Rc::new(RefCell::new(v)))
                }

                fn get_name<'s>(&self) -> &'s str {
                    "ref"
                }
            }

            // add()
            #[derive(Debug)]
            struct AddFunc;

            impl RuntimeFunction for AddFunc {
                fn call<'s>(
                    &mut self,
                    mut args: Vec<LenarValue<'s>>,
                    _tokens_map: &'s Arc<Tokenizer>,
                ) -> LenarValue<'s> {
                    let value = args.remove(0);
                    let increment = args.remove(0);
                    if let LenarValue::Ref(value) = value {
                        let mut value = value.borrow_mut();
                        let value = value.as_integer_mut();
                        let increment = increment.as_integer();
                        if let Some((value, increment)) = value.zip(increment) {
                            *value += increment;
                        }
                    }
                    LenarValue::Void
                }

                fn get_name<'s>(&self) -> &'s str {
                    "add"
                }
            }

            self.variables
                .insert("add".to_string(), LenarValue::Function(Rc::new(AddFunc)));
            self.variables
                .insert("ref".to_string(), LenarValue::Function(Rc::new(RefFunc)));
            self.variables.insert(
                "unwrapErr".to_string(),
                LenarValue::Function(Rc::new(UnwrapErrFunc)),
            );
            self.variables.insert(
                "unwrap".to_string(),
                LenarValue::Function(Rc::new(UnwrapFunc)),
            );
            self.variables
                .insert("Err".to_string(), LenarValue::Function(Rc::new(ErrFunc)));
            self.variables
                .insert("isOk".to_string(), LenarValue::Function(Rc::new(IsOkFunc)));
            self.variables
                .insert("Ok".to_string(), LenarValue::Function(Rc::new(OkFunc)));
            self.variables.insert(
                "wait".to_string(),
                LenarValue::Function(Rc::new(WaitFunc::new(self.locks.clone()))),
            );
            self.variables.insert(
                "sleep".to_string(),
                LenarValue::Function(Rc::new(SleepFunc)),
            );
            self.variables.insert(
                "thread".to_string(),
                LenarValue::Function(Rc::new(ThreadFunc)),
            );
            self.variables.insert(
                "newList".to_string(),
                LenarValue::Function(Rc::new(NewListFunc)),
            );
            self.variables.insert(
                "iter".to_string(),
                LenarValue::Function(Rc::new(IterFunc::new(resources_files.clone()))),
            );
            self.variables.insert(
                "toString".to_string(),
                LenarValue::Function(Rc::new(ToStringFunc::new(resources_files.clone()))),
            );
            self.variables.insert(
                "openFile".to_string(),
                LenarValue::Function(Rc::new(OpenFileFunc::new(resources_files))),
            );
            self.variables.insert(
                "print".to_string(),
                LenarValue::Function(Rc::new(PrintFunc)),
            );
            self.variables.insert(
                "println".to_string(),
                LenarValue::Function(Rc::new(PrintLnFunc)),
            );
            self.variables.insert(
                "Lenar".to_string(),
                LenarValue::Instance(Rc::new(RefCell::new(LenarGlobal))),
            );
            self.variables.insert(
                "isEqual".to_string(),
                LenarValue::Function(Rc::new(IsEqual)),
            );
        }

        pub fn get_scope(&mut self, path: &mut Iter<usize>) -> &mut Scope<'a> {
            let scope = path.next();

            if let Some(scope) = scope {
                self.scopes.get_mut(scope).unwrap().get_scope(path)
            } else {
                self
            }
        }

        pub fn get_function(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
        ) -> Option<&mut Rc<dyn RuntimeFunction>> {
            let scope = path.next();
            if let Some(scope) = scope {
                let result = self
                    .scopes
                    .get_mut(scope)
                    .unwrap()
                    .get_function(name.as_ref(), path);
                if result.is_some() {
                    return result;
                }
            }

            let func = self.variables.get_mut(name.as_ref());
            if let Some(LenarValue::Function(func)) = func {
                Some(func)
            } else {
                None
            }
        }

        /// Call a function given a name, a scope ID and arguments
        pub fn call_function(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
            args: Vec<LenarValue<'a>>,
            tokens_map: &'a Arc<Tokenizer>,
        ) -> LenarValue<'a> {
            let func = self.get_function(name, path);

            if let Some(func) = func {
                let func = Rc::get_mut(func).unwrap();
                func.call(args, tokens_map)
            } else {
                LenarValue::Void
            }
        }

        /// Define a variable with a given name and a value in the specified scope ID
        pub fn define_variable(
            &mut self,
            name: impl AsRef<str>,
            scope_id: &[usize],
            value: LenarValue<'a>,
        ) {
            let scope = self.get_scope(&mut scope_id.iter());
            scope.variables.insert(name.as_ref().to_string(), value);
        }

        /// Resolve a variable value given it's name and the caller scope ID
        pub fn get_variable(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
        ) -> LenarValue<'a> {
            let scope = path.next();
            if let Some(scope) = scope {
                let result = self
                    .scopes
                    .get_mut(scope)
                    .unwrap()
                    .get_variable(name.as_ref(), path);
                if !result.is_void() {
                    return result;
                }
            }

            // Currently referencing a variable clones it's value,
            // Once I add proper value-movements I will do this by calling
            // `variables.remove(name.as_ref())` and without the `clone()`
            // This way the variable's owned value will get removed from the scope folder
            // and returned to the variable referencer

            let var = self.variables.get(name.as_ref());
            if let Some(var) = var {
                var.clone()
            } else {
                LenarValue::Void
            }
        }

        pub fn get_variable_by_path(
            &mut self,
            var_path: &'a [String],
            path: &mut Iter<usize>,
        ) -> LenarValue<'a> {
            let scope = path.next();
            if let Some(scope) = scope {
                let result = self
                    .scopes
                    .get_mut(scope)
                    .unwrap()
                    .get_variable_by_path(var_path, path);
                if !result.is_void() {
                    return result;
                }
            }

            let mut var_path = var_path.iter();

            let var_holder = var_path.next().unwrap();
            if let Some(LenarValue::Instance(instance)) = self.variables.get(var_holder) {
                let instance = instance.borrow_mut();
                instance.get_props(&mut var_path)
            } else {
                LenarValue::Void
            }
        }

        /// Create a new scope given an ID in the specified scope by a path
        pub fn create_scope(&mut self, scope_path: &[usize], scope_id: usize) {
            let scope = self.get_scope(&mut scope_path.iter());
            let new_scope = Scope::default();

            scope.scopes.insert(scope_id, new_scope);
        }

        /// Drop a scope given an ID and a scope path
        pub fn drop_scope(&mut self, scope_path: &[usize], scope_id: usize) {
            let scope = self.get_scope(&mut scope_path.iter());
            scope.scopes.remove(&scope_id);
        }
    }

    /// Evaluate an expression to a value
    fn evaluate_expression<'a>(
        token: &'a Token,
        tokens_map: &'a Arc<Tokenizer>,
        scope: &mut Scope<'a>,
        scope_path: &[usize],
    ) -> LenarValue<'a> {
        match token {
            Token::Block { tokens } => {
                let mut next_scope_id = scope_path.last().copied().unwrap_or(0);

                for (i, tok) in tokens.iter().enumerate() {
                    let is_last = i == tokens.len() - 1;
                    let tok = tokens_map.get_token(*tok).unwrap();
                    let res = if matches!(tok, Token::Block { .. }) {
                        next_scope_id += 1;
                        // Create block scope
                        scope.create_scope(scope_path, next_scope_id);

                        // Run the block expression in the new scope
                        let scope_path = &[scope_path, &[next_scope_id]].concat();
                        let return_val = evaluate_expression(tok, tokens_map, scope, scope_path);

                        // Remove the scope
                        scope.drop_scope(scope_path, next_scope_id);
                        return_val
                    } else {
                        // Run the expression in the inherited scope
                        evaluate_expression(tok, tokens_map, scope, scope_path)
                    };

                    // Return the returned value from the expression as result of this block
                    if is_last {
                        return res;
                    }
                }

                LenarValue::Void
            }
            Token::VarDef {
                var_name,
                block_value,
            } => {
                let value = tokens_map.get_token(*block_value).unwrap();
                let res = evaluate_expression(value, tokens_map, scope, scope_path);
                scope.define_variable(var_name, scope_path, res);

                LenarValue::Void
            }
            Token::FunctionCall { arguments, fn_name } => {
                if fn_name == "thread" {
                    let tokens_map = tokens_map.clone();
                    let arguments = *arguments;
                    let fn_name = fn_name.clone();

                    let handle = thread::spawn(move || {
                        let mut context = Scope::default();

                        context.setup_globals();
                        let value = tokens_map.get_token(arguments).unwrap();
                        let mut args = Vec::new();
                        if let Token::Block { tokens } = value {
                            for tok in tokens {
                                let tok = tokens_map.get_token(*tok).unwrap();
                                let res = evaluate_expression(tok, &tokens_map, &mut context, &[]);

                                args.push(res);
                            }
                        }

                        context.call_function(fn_name, &mut [].iter(), args, &tokens_map);
                    });
                    let id = scope.locks.lock().unwrap().insert(handle);
                    LenarValue::Usize(id)
                } else {
                    let value = tokens_map.get_token(*arguments).unwrap();
                    let mut args = Vec::new();
                    if let Token::Block { tokens } = value {
                        for tok in tokens {
                            let tok = tokens_map.get_token(*tok).unwrap();
                            let res = evaluate_expression(tok, tokens_map, scope, scope_path);

                            args.push(res);
                        }
                    }

                    scope.call_function(fn_name, &mut scope_path.iter(), args, tokens_map)
                }
            }
            Token::StringVal { value } => LenarValue::Str(value),
            Token::BytesVal { value } => LenarValue::Bytes(value),
            Token::VarRef { var_name } => scope.get_variable(var_name, &mut scope_path.iter()),
            Token::PropertyRef { path } => scope.get_variable_by_path(path, &mut scope_path.iter()),
            Token::FnDef {
                arguments_block,
                block_value,
            } => {
                // Anonymous function create at runtime
                #[derive(Debug)]
                struct Function {
                    arguments_block: usize,
                    block_value: usize,
                }

                impl RuntimeFunction for Function {
                    fn call<'s>(
                        &mut self,
                        mut args: Vec<LenarValue<'s>>,
                        tokens_map: &'s Arc<Tokenizer>,
                    ) -> LenarValue<'s> {
                        // Anonymous functions do not inherit any scope,
                        // instead, they only have their own global scope,
                        // This means, you cannot reference variables from outside
                        // this scope, you can pass them as arguments though.
                        let mut context = Scope::default();

                        context.setup_globals();

                        // Define each argument as a variable in the function scope
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

                        evaluate_expression(block_token, tokens_map, &mut context, &[])
                    }

                    fn get_name<'s>(&self) -> &'s str {
                        "Anonymous"
                    }
                }
                LenarValue::Function(Rc::new(Function {
                    arguments_block: *arguments_block,
                    block_value: *block_value,
                }))
            }
            Token::IfDef {
                condition_block: expr,
                block_value,
            } => {
                let expr_token = tokens_map.get_token(*expr).unwrap();
                let expr_res = evaluate_expression(expr_token, tokens_map, scope, scope_path);

                // If the condition expression returns a `true` it
                // will evaluate the actual block
                if LenarValue::Bool(true) == expr_res {
                    let expr_body_token = tokens_map.get_token(*block_value).unwrap();
                    evaluate_expression(expr_body_token, tokens_map, scope, scope_path)
                } else {
                    LenarValue::Void
                }
            }
            Token::NumberVal { value } => LenarValue::Usize(*value),
        }
    }
}
