pub mod parser {
    use std::{iter::Peekable, str::Chars, sync::Arc};

    pub use slab::Slab;

    pub type ParserObjectKey = usize;

    /// [`Parser`] transforms the given code into an AST.
    #[derive(Debug, Clone)]
    pub struct Parser {
        objects: Slab<ParserObject>,
        global_block: ParserObjectKey,
    }

    #[derive(Debug, Clone)]
    pub enum ParserObject {
        Block {
            objects: Vec<ParserObjectKey>,
        },
        VarDef {
            block_value: ParserObjectKey,
            var_name: String,
        },
        FnDef {
            arguments_block: ParserObjectKey,
            block_value: ParserObjectKey,
            capture_value: ParserObjectKey,
        },
        IfDef {
            condition_block: ParserObjectKey,
            block_value: ParserObjectKey,
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
            arguments: ParserObjectKey,
        },
        VarRef {
            var_name: String,
        },
        PropertyRef {
            path: Vec<String>,
        },
    }

    impl ParserObject {
        #[inline(always)]
        pub fn add_object(&mut self, object: ParserObjectKey) {
            if let ParserObject::Block { objects } = self {
                objects.push(object);
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
        let until = [',', ';', ')', '}', ' ', '\n', ']'];
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
        FuncCapture,
    }

    impl Parser {
        /// Create a [`Parser`] given some code
        pub fn new(code: &str) -> Self {
            let mut parser = Slab::new();

            let global_block_object = ParserObject::Block {
                objects: Vec::new(),
            };
            let global_block = parser.insert(global_block_object);

            let mut parser = Self {
                objects: parser,
                global_block,
            };

            parser.parse(code);

            parser
        }

        /// Wrap into an [Arc]
        pub fn wrap(self) -> Arc<Self> {
            Arc::new(self)
        }

        /// Parse additional code
        pub fn parse(&mut self, code: &str) {
            let parser = &mut self.objects;
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
                        let string_val = ParserObject::BytesVal {
                            value: code[i - string_count + 1..i]
                                .chars()
                                .collect::<String>()
                                .as_bytes()
                                .to_vec(),
                        };

                        let string_key = parser.insert(string_val);

                        let block_value = parser.get_mut(current_block).unwrap();
                        if let ParserObject::Block { objects } = block_value {
                            objects.push(string_key);
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
                    let block = ParserObject::Block {
                        objects: Vec::new(),
                    };
                    let block_key = parser.insert(block);

                    block_indexes.push((block_key, BlockType::Generic));
                    let current_block = parser.get_mut(current_block).unwrap();
                    current_block.add_object(block_key);

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

                if val == '[' && string_count == 0 {
                    continue;
                }

                // Closing a capture area block
                if val == ']' && string_count == 0 {
                    block_indexes.pop();
                    last_action = PerfomedAction::ClosedBlock;
                    continue;
                }

                // Variable declarations
                if string_count == 0 && slice_with_size(i, i + 3, code) == Some("let") {
                    advance_by(3, &mut chars);
                    let var_name = slice_until(' ', &mut chars);
                    let value_block = ParserObject::Block {
                        objects: Vec::new(),
                    };
                    let block_key = parser.insert(value_block);

                    let var_def = ParserObject::VarDef {
                        block_value: block_key,
                        var_name,
                    };
                    let var_key = parser.insert(var_def);

                    let current_block = parser.get_mut(current_block).unwrap();
                    current_block.add_object(var_key);

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
                            let expr_block = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let expr_block_key = parser.insert(expr_block);

                            let value_block = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let block_key = parser.insert(value_block);

                            let if_def = ParserObject::IfDef {
                                block_value: block_key,
                                condition_block: expr_block_key,
                            };
                            let if_key = parser.insert(if_def);

                            let current_block = parser.get_mut(current_block).unwrap();
                            current_block.add_object(if_key);

                            block_indexes.push((block_key, BlockType::FuncValue));
                            block_indexes.push((expr_block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        } else if item_name == "fn" {
                            // Function args
                            let args_block = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let args_block_key = parser.insert(args_block);

                            // Function body
                            let value_block = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let block_key = parser.insert(value_block);

                            // Function capture area
                            let value_capture = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let capture_key = parser.insert(value_capture);

                            let fn_def = ParserObject::FnDef {
                                block_value: block_key,
                                arguments_block: args_block_key,
                                capture_value: capture_key,
                            };
                            let fn_key = parser.insert(fn_def);

                            let current_block = parser.get_mut(current_block).unwrap();
                            current_block.add_object(fn_key);

                            block_indexes.push((block_key, BlockType::FuncValue));
                            block_indexes.push((capture_key, BlockType::FuncCapture));
                            block_indexes.push((args_block_key, BlockType::FuncCall));

                            last_action = PerfomedAction::CalledFunction;
                        } else {
                            let value_block = ParserObject::Block {
                                objects: Vec::new(),
                            };
                            let block_key = parser.insert(value_block);

                            let fn_call_def = ParserObject::FunctionCall {
                                fn_name: item_name,
                                arguments: block_key,
                            };
                            let fn_call_key = parser.insert(fn_call_def);

                            let current_block = parser.get_mut(current_block).unwrap();
                            current_block.add_object(fn_call_key);

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

                        let var_ref = ParserObject::PropertyRef { path };
                        let var_ref_key = parser.insert(var_ref);

                        let current_block = parser.get_mut(current_block).unwrap();
                        current_block.add_object(var_ref_key);

                        last_action = PerfomedAction::ReferencedVariable;

                        continue;
                    } else if val.is_ascii_digit() {
                        let item_val = slice_until_delimeter(&mut chars);
                        let item_val = format!("{val}{item_val}");

                        if let Ok(value) = item_val.parse::<usize>() {
                            let number_val = ParserObject::NumberVal { value };

                            let number_val_key = parser.insert(number_val);

                            let current_block = parser.get_mut(current_block).unwrap();
                            current_block.add_object(number_val_key);

                            last_action = PerfomedAction::FoundNumber;
                        }

                        continue;
                    } else {
                        let item_name = slice_until_delimeter(&mut chars);
                        let item_name = format!("{val}{item_name}");

                        let var_ref = ParserObject::VarRef {
                            var_name: item_name,
                        };
                        let var_ref_key = parser.insert(var_ref);

                        let current_block = parser.get_mut(current_block).unwrap();
                        current_block.add_object(var_ref_key);

                        last_action = PerfomedAction::ReferencedVariable;

                        continue;
                    }
                }
            }
        }

        /// Retrieve the global block object
        pub fn get_global(&self) -> ParserObjectKey {
            self.global_block
        }

        /// Retrieve a ParserObject given a `key`
        #[inline(always)]
        pub fn get_object(&self, key: ParserObjectKey) -> Option<&ParserObject> {
            self.objects.get(key)
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

    use crate::parser::{Parser, ParserObject};

    pub type LenarResult<T> = Result<T, LenarError>;

    /// A interpreter given a Parser
    pub struct Runtime;

    impl Runtime {
        pub fn run_with_scope(scope: &mut Scope, parser: &Arc<Parser>) -> LenarResult<LenarValue> {
            let global_block = parser.get_object(parser.get_global()).unwrap();
            evaluate_object(global_block, parser, scope, &[])
        }

        /// Evaluate the runtime code and return the exit value
        pub fn evaluate(parser: &Arc<Parser>) -> LenarResult<LenarValue> {
            let mut scope = Scope::default();
            scope.setup_globals();

            Self::run_with_scope(&mut scope, parser)
        }

        pub fn run(code: &str) {
            let parser = Arc::new(Parser::new(code));
            Self::evaluate(&parser).ok();
        }
    }

    /// Runtime values
    #[derive(Debug, Clone)]
    pub enum LenarValue {
        Usize(usize),
        List(Vec<LenarValue>),
        Str(String),
        Byte(u8),
        Bytes(Vec<u8>),
        OwnedBytes(Vec<u8>),
        Void,
        Bool(bool),
        Instance(Rc<RefCell<dyn RuntimeInstance>>),
        Function(Rc<RefCell<dyn RuntimeFunction>>),
        Enum(LenarEnum),
        Ref(Rc<RefCell<LenarValue>>),
    }

    /// Runtime values
    #[derive(Debug, Clone)]
    pub enum LenarError {
        VariableNotFound(String),
        WrongValue(String),
    }

    #[derive(Debug, Clone, Default)]
    pub struct LenarEnum(HashMap<String, LenarValue>);

    impl LenarEnum {
        pub fn new_with_variant(variant_name: String, variant_value: LenarValue) -> Self {
            let mut en = LenarEnum::default();
            en.0.insert(variant_name, variant_value);
            en
        }

        pub fn peek_variant(&self, variant_name: &str) -> Option<&LenarValue> {
            self.0.get(variant_name)
        }

        pub fn get_variant(mut self, variant_name: &str) -> Option<LenarValue> {
            self.0.remove(variant_name)
        }
    }

    impl Display for LenarEnum {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(
                &self
                    .0
                    .iter()
                    .map(|(k, v)| format!("{k}({v})"))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        }
    }

    impl Display for LenarValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LenarValue::Usize(u) => f.write_str(&format!("{u}")),
                LenarValue::List(l) => f
                    .debug_map()
                    .value(&l.iter().map(|v| format!("{v}")))
                    .finish(),
                LenarValue::Str(s) => f.write_str(s),
                LenarValue::Byte(b) => f.write_str(from_utf8(&[*b]).unwrap()),
                LenarValue::Bytes(b) => f.write_str(from_utf8(b).unwrap()),
                LenarValue::OwnedBytes(b) => f.write_str(from_utf8(b).unwrap()),
                LenarValue::Void => f.write_str("Void"),
                LenarValue::Bool(b) => f.write_str(&format!("{b}")),
                LenarValue::Instance(i) => f.write_str(i.borrow().get_name()),
                LenarValue::Function(func) => f.write_str(func.borrow().get_name()),
                LenarValue::Enum(en) => f.write_str(&en.to_string()),
                LenarValue::Ref(r) => f.write_str(&r.borrow().to_string()),
            }
        }
    }

    impl PartialEq for LenarValue {
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

    impl LenarValue {
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

        pub fn as_integer(&self) -> Option<usize> {
            match self {
                Self::Usize(v) => Some(*v),
                Self::Ref(v) => v.borrow().as_integer(),
                _ => None,
            }
        }

        pub fn as_func(&self) -> Option<Rc<RefCell<dyn RuntimeFunction>>> {
            match self {
                Self::Function(v) => Some(v.clone()),
                Self::Ref(v) => v.borrow().as_func(),
                _ => None,
            }
        }

        pub fn set_integer(&mut self, integer: usize) {
            match self {
                Self::Usize(v) => *v = integer,
                Self::Ref(r) => r.borrow_mut().set_integer(integer),
                _ => {}
            }
        }
    }

    /// Lenar special objects base
    pub trait RuntimeInstance: Debug {
        fn get_props(&self, path: &mut Iter<String>) -> LenarValue {
            let prop = path.next();
            if let Some(prop) = prop {
                self.get_prop(prop)
            } else {
                LenarValue::Void
            }
        }

        fn get_prop(&self, prop: &str) -> LenarValue;

        fn get_name(&self) -> &str;
    }

    /// Lenar function base trait
    pub trait RuntimeFunction: Debug {
        /// Call the runtime function implementation
        fn call(&mut self, _args: Vec<LenarValue>, parser: &Arc<Parser>)
            -> LenarResult<LenarValue>;

        /// Get the function name
        fn get_name(&self) -> &str;
    }

    /// Runtime Scope that includes variables and nested Scopes.
    #[derive(Default)]
    pub struct Scope {
        thread_locks: Arc<Mutex<Slab<JoinHandle<()>>>>,
        variables: HashMap<String, LenarValue>,
        scopes: HashMap<usize, Scope>,
    }

    impl Scope {
        /// Add a [`RuntimeInstance`] to the global scope
        pub fn add_global_instance(&mut self, val: impl RuntimeInstance + 'static) {
            self.variables.insert(
                val.get_name().to_owned(),
                LenarValue::Instance(Rc::new(RefCell::new(val))),
            );
        }

        /// Add a [`RuntimeFunction`] to the global scope
        pub fn add_global_function(&mut self, val: impl RuntimeFunction + 'static) {
            self.variables.insert(
                val.get_name().to_owned(),
                LenarValue::Function(Rc::new(RefCell::new(val))),
            );
        }

        /// Define the global variables
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
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    match args[0] {
                        LenarValue::Usize(rid) => {
                            let resources_files = self.resources_files.borrow_mut();
                            let mut file = resources_files.get(rid).unwrap();
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();
                            Ok(LenarValue::OwnedBytes(buf))
                        }
                        _ => Ok(LenarValue::Void),
                    }
                }

                fn get_name(&self) -> &str {
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
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let file_path = args[0].as_bytes().unwrap();
                    let file_path = from_utf8(file_path).unwrap();
                    let file = File::open(file_path).unwrap();

                    let mut resources_files = self.resources_files.borrow_mut();
                    let rid = resources_files.insert(file);

                    Ok(LenarValue::Usize(rid))
                }

                fn get_name(&self) -> &str {
                    "openFile"
                }
            }

            #[derive(Debug)]
            struct LenarGlobal;

            impl RuntimeInstance for LenarGlobal {
                fn get_prop(&self, prop: &str) -> LenarValue {
                    match prop {
                        "version" => LenarValue::Str("1.0.0".to_string()),
                        _ => LenarValue::Void,
                    }
                }

                fn get_name(&self) -> &str {
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
                        LenarValue::Byte(b) => {
                            stdout().write(&[*b]).ok();
                        }
                        LenarValue::Bytes(bts) => {
                            stdout().write(bts).ok();
                        }
                        LenarValue::Function(func) => {
                            stdout().write(func.borrow().get_name().as_bytes()).ok();
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
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    for val in args {
                        Self::write(&val);
                    }
                    stdout().flush().ok();
                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
                    "print"
                }
            }

            // println()
            #[derive(Debug)]
            struct PrintLnFunc;

            impl RuntimeFunction for PrintLnFunc {
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    for val in args {
                        PrintFunc::write(&val);
                    }
                    stdout().write("\n".as_bytes()).ok();
                    stdout().flush().ok();
                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
                    "println"
                }
            }

            // isEqual()
            #[derive(Debug)]
            struct IsEqual;

            impl RuntimeFunction for IsEqual {
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let args = args.get(0).zip(args.get(1));
                    let res = if let Some((a, b)) = args {
                        a.eq(b)
                    } else {
                        false
                    };
                    Ok(LenarValue::Bool(res))
                }

                fn get_name(&self) -> &str {
                    "isEqual"
                }
            }

            // NewList()
            #[derive(Debug)]
            struct NewListFunc;

            impl RuntimeFunction for NewListFunc {
                fn call(
                    &mut self,
                    args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    Ok(LenarValue::List(args))
                }

                fn get_name(&self) -> &str {
                    "list"
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
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let iterator = args.remove(0);
                    let fun = args.remove(0);

                    if let LenarValue::Function(fun) = fun {
                        let mut fun = fun.borrow_mut();
                        match iterator {
                            LenarValue::Usize(rid) => {
                                let resources_files = self.resources_files.borrow_mut();
                                let file = resources_files.get(rid).unwrap();
                                let bytes = file.bytes();

                                for byte in bytes {
                                    if let Ok(byte) = byte {
                                        fun.call(vec![LenarValue::Byte(byte)], _parser)?;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            LenarValue::Bytes(bytes) => {
                                for byte in bytes {
                                    fun.call(vec![LenarValue::Byte(byte)], _parser)?;
                                }
                            }
                            LenarValue::OwnedBytes(bytes) => {
                                for byte in bytes {
                                    fun.call(vec![LenarValue::Byte(byte)], _parser)?;
                                }
                            }
                            LenarValue::List(items) => {
                                for (i, item) in items.into_iter().enumerate() {
                                    fun.call(vec![item, LenarValue::Usize(i)], _parser)?;
                                }
                            }
                            _ => {}
                        }
                    }

                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
                    "iter"
                }
            }

            // thread()
            #[derive(Debug)]
            struct ThreadFunc;

            impl RuntimeFunction for ThreadFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let fun = args.remove(0);

                    if let LenarValue::Function(fun) = fun {
                        let mut fun = fun.borrow_mut();
                        fun.call(args, parser)?;
                    }

                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
                    "thread"
                }
            }

            // sleep()
            #[derive(Debug)]
            struct SleepFunc;

            impl RuntimeFunction for SleepFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    if let LenarValue::Usize(time) = v {
                        thread::sleep(Duration::from_millis(time as u64));
                    }
                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
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
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    if let LenarValue::Usize(rid) = v {
                        let handle = self.0.lock().unwrap().remove(rid);
                        handle.join().unwrap();
                    }
                    Ok(LenarValue::Void)
                }

                fn get_name(&self) -> &str {
                    "wait"
                }
            }

            // Ok()
            #[derive(Debug)]
            struct OkFunc;

            impl RuntimeFunction for OkFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    Ok(LenarValue::Enum(LenarEnum::new_with_variant(
                        "Ok".to_string(),
                        v,
                    )))
                }

                fn get_name(&self) -> &str {
                    "Ok"
                }
            }

            // Err()
            #[derive(Debug)]
            struct ErrFunc;

            impl RuntimeFunction for ErrFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    Ok(LenarValue::Enum(LenarEnum::new_with_variant(
                        "Err".to_string(),
                        v,
                    )))
                }

                fn get_name(&self) -> &str {
                    "Err"
                }
            }

            // isOk()
            #[derive(Debug)]
            struct IsOkFunc;

            impl RuntimeFunction for IsOkFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    match v {
                        LenarValue::Enum(variants) => {
                            let ok_variant = variants.peek_variant("Ok");
                            Ok(LenarValue::Bool(ok_variant.is_some()))
                        }
                        _ => Ok(LenarValue::Bool(false)),
                    }
                }

                fn get_name(&self) -> &str {
                    "isOk"
                }
            }

            // unwrap()
            #[derive(Debug)]
            struct UnwrapFunc;

            impl RuntimeFunction for UnwrapFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let value = args.remove(0);
                    match value {
                        LenarValue::Enum(variants) => {
                            let variant = variants.get_variant("Ok");
                            variant.ok_or_else(|| LenarError::WrongValue("Ok".to_owned()))
                        }
                        _ => Ok(LenarValue::Void),
                    }
                }

                fn get_name(&self) -> &str {
                    "unwrap"
                }
            }

            // unwrapErr()
            #[derive(Debug)]
            struct UnwrapErrFunc;

            impl RuntimeFunction for UnwrapErrFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let value = args.remove(0);
                    match value {
                        LenarValue::Enum(variants) => {
                            let variant = variants.get_variant("Err");
                            variant.ok_or_else(|| LenarError::WrongValue("Err".to_owned()))
                        }
                        _ => Ok(LenarValue::Void),
                    }
                }

                fn get_name(&self) -> &str {
                    "unwrapErr"
                }
            }

            // ref()
            #[derive(Debug)]
            struct RefFunc;

            impl RuntimeFunction for RefFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let v = args.remove(0);
                    Ok(LenarValue::Ref(Rc::new(RefCell::new(v))))
                }

                fn get_name(&self) -> &str {
                    "ref"
                }
            }

            // add()
            #[derive(Debug)]
            struct AddFunc;

            impl RuntimeFunction for AddFunc {
                fn call(
                    &mut self,
                    mut args: Vec<LenarValue>,
                    _parser: &Arc<Parser>,
                ) -> LenarResult<LenarValue> {
                    let value = args.remove(0);
                    let increment = args.remove(0);

                    let result = match value {
                        LenarValue::Ref(value) => {
                            let mut value = value.borrow_mut();
                            let increment = increment.as_integer();
                            if let Some(increment) = increment {
                                value.set_integer(increment);

                                if let Some(n) = value.as_integer() {
                                    LenarValue::Usize(n)
                                } else {
                                    LenarValue::Void
                                }
                            } else {
                                LenarValue::Void
                            }
                        }
                        _ => LenarValue::Void,
                    };
                    Ok(result)
                }

                fn get_name(&self) -> &str {
                    "add"
                }
            }

            self.variables.insert(
                "add".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(AddFunc))),
            );
            self.variables.insert(
                "ref".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(RefFunc))),
            );
            self.variables.insert(
                "unwrapErr".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(UnwrapErrFunc))),
            );
            self.variables.insert(
                "unwrap".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(UnwrapFunc))),
            );
            self.variables.insert(
                "Err".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(ErrFunc))),
            );
            self.variables.insert(
                "isOk".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(IsOkFunc))),
            );
            self.variables.insert(
                "Ok".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(OkFunc))),
            );
            self.variables.insert(
                "wait".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(WaitFunc::new(
                    self.thread_locks.clone(),
                )))),
            );
            self.variables.insert(
                "sleep".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(SleepFunc))),
            );
            self.variables.insert(
                "thread".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(ThreadFunc))),
            );
            self.variables.insert(
                "list".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(NewListFunc))),
            );
            self.variables.insert(
                "iter".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(IterFunc::new(
                    resources_files.clone(),
                )))),
            );
            self.variables.insert(
                "toString".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(ToStringFunc::new(
                    resources_files.clone(),
                )))),
            );
            self.variables.insert(
                "openFile".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(OpenFileFunc::new(resources_files)))),
            );
            self.variables.insert(
                "print".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(PrintFunc))),
            );
            self.variables.insert(
                "println".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(PrintLnFunc))),
            );
            self.variables.insert(
                "Lenar".to_string(),
                LenarValue::Instance(Rc::new(RefCell::new(LenarGlobal))),
            );
            self.variables.insert(
                "isEqual".to_string(),
                LenarValue::Function(Rc::new(RefCell::new(IsEqual))),
            );
        }

        /// Get a mutable handle scope to the desired [`Scope`]
        pub fn get_scope(&mut self, path: &mut Iter<usize>) -> &mut Scope {
            let scope = path.next();

            if let Some(scope) = scope {
                self.scopes.get_mut(scope).unwrap().get_scope(path)
            } else {
                self
            }
        }

        /// Get a mutable handle to the desired [`RuntimeFunction`]
        pub fn get_function(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
        ) -> Option<Rc<RefCell<dyn RuntimeFunction>>> {
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

            let variable = self.variables.get(name.as_ref())?;
            variable.as_func()
        }

        /// Call a function given a name, a scope ID and arguments
        pub fn call_function(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
            args: Vec<LenarValue>,
            parser: &Arc<Parser>,
        ) -> LenarResult<LenarValue> {
            let func_name = name.as_ref().to_string();
            let func = self.get_function(name, path);

            if let Some(func) = func {
                let mut func = func.borrow_mut();
                func.call(args, parser)
            } else {
                Err(LenarError::VariableNotFound(func_name))
            }
        }

        /// Define a variable with a given name and a value in the specified scope ID
        pub fn define_variable(
            &mut self,
            name: impl AsRef<str>,
            scope_path: &[usize],
            value: LenarValue,
        ) {
            let scope = self.get_scope(&mut scope_path.iter());
            scope.variables.insert(name.as_ref().to_string(), value);
        }

        /// Resolve a variable value given it's name and the caller scope ID
        pub fn get_variable(
            &mut self,
            name: impl AsRef<str>,
            path: &mut Iter<usize>,
        ) -> LenarResult<LenarValue> {
            let scope = path.next();

            if let Some(scope) = scope {
                let result = self
                    .scopes
                    .get_mut(scope)
                    .unwrap()
                    .get_variable(name.as_ref(), path);
                if let Ok(result) = result {
                    if !result.is_void() {
                        return Ok(result);
                    }
                }
            }

            // Currently referencing a variable clones it's value,
            // Once I add proper value-movements I will do this by calling
            // `variables.remove(name.as_ref())` and without the `clone()`
            // This way the variable's owned value will get removed from the scope folder
            // and returned to the variable referencer
            let var_name = name.as_ref().to_owned();
            let var = self.variables.get(name.as_ref());
            if let Some(var) = var {
                Ok(var.clone())
            } else {
                Err(LenarError::VariableNotFound(var_name))
            }
        }

        pub fn get_variable_by_path(
            &mut self,
            var_path: &[String],
            path: &mut Iter<usize>,
        ) -> LenarResult<LenarValue> {
            let scope = path.next();
            if let Some(scope) = scope {
                let result = self
                    .scopes
                    .get_mut(scope)
                    .unwrap()
                    .get_variable_by_path(var_path, path);
                if let Ok(result) = result {
                    if !result.is_void() {
                        return Ok(result);
                    }
                }
            }

            let mut var_path = var_path.iter();

            let var_holder = var_path.next().unwrap();
            if let Some(LenarValue::Instance(instance)) = self.variables.get(var_holder) {
                let instance = instance.borrow_mut();
                Ok(instance.get_props(&mut var_path))
            } else {
                Err(LenarError::VariableNotFound(var_holder.clone()))
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

    /// Evaluate a [`ParserObject`] to a [`LenarValue`]
    fn evaluate_object(
        object: &ParserObject,
        parser: &Arc<Parser>,
        scope: &mut Scope,
        scope_path: &[usize],
    ) -> LenarResult<LenarValue> {
        match object {
            ParserObject::Block { objects } => {
                let mut next_scope_id = scope_path.last().copied().unwrap_or(0);

                for (i, tok) in objects.iter().enumerate() {
                    let is_last = i == objects.len() - 1;
                    let tok = parser.get_object(*tok).unwrap();
                    let res = if matches!(tok, ParserObject::Block { .. }) {
                        next_scope_id += 1;
                        // Create block scope
                        scope.create_scope(scope_path, next_scope_id);

                        // Run the block expression in the new scope
                        let scope_path = &[scope_path, &[next_scope_id]].concat();
                        let return_val = evaluate_object(tok, parser, scope, scope_path);

                        // Remove the scope
                        scope.drop_scope(scope_path, next_scope_id);
                        return_val
                    } else {
                        // Run the expression in the inherited scope
                        evaluate_object(tok, parser, scope, scope_path)
                    };

                    // Return the returned value from the expression as result of this block
                    if is_last {
                        return res;
                    }
                }

                Ok(LenarValue::Void)
            }
            ParserObject::VarDef {
                var_name,
                block_value,
            } => {
                let value = parser.get_object(*block_value).unwrap();
                let res = evaluate_object(value, parser, scope, scope_path)?;
                scope.define_variable(var_name, scope_path, res);

                Ok(LenarValue::Void)
            }
            ParserObject::FunctionCall { arguments, fn_name } => {
                if fn_name == "thread" {
                    let parser = parser.clone();
                    let arguments = *arguments;
                    let fn_name = fn_name.clone();

                    let handle = thread::spawn(move || {
                        let mut scope = Scope::default();

                        scope.setup_globals();
                        let value = parser.get_object(arguments).unwrap();
                        let mut args = Vec::new();
                        if let ParserObject::Block { objects } = value {
                            for tok in objects {
                                let tok = parser.get_object(*tok).unwrap();
                                let res = evaluate_object(tok, &parser, &mut scope, &[]).unwrap();

                                args.push(res);
                            }
                        }

                        scope
                            .call_function(fn_name, &mut [].iter(), args, &parser)
                            .unwrap();
                    });
                    let id = scope.thread_locks.lock().unwrap().insert(handle);
                    Ok(LenarValue::Usize(id))
                } else {
                    let value = parser.get_object(*arguments).unwrap();
                    let mut args = Vec::new();
                    if let ParserObject::Block { objects } = value {
                        for tok in objects {
                            let tok = parser.get_object(*tok).unwrap();
                            let res = evaluate_object(tok, parser, scope, scope_path)?;

                            args.push(res);
                        }
                    }

                    scope.call_function(fn_name, &mut scope_path.iter(), args, parser)
                }
            }
            ParserObject::StringVal { value } => Ok(LenarValue::Str(value.to_string())), // TODO: Optimize this
            ParserObject::BytesVal { value } => Ok(LenarValue::Bytes(value.to_owned())), // TODO: Optimize this
            ParserObject::VarRef { var_name } => {
                scope.get_variable(var_name, &mut scope_path.iter())
            }
            ParserObject::PropertyRef { path } => {
                scope.get_variable_by_path(path, &mut scope_path.iter())
            }
            ParserObject::FnDef {
                arguments_block,
                block_value,
                capture_value,
            } => {
                let capture_area_value = parser.get_object(*capture_value);

                let capture_area = {
                    let mut capture_area = HashMap::default();
                    if let Some(ParserObject::Block { objects }) = capture_area_value {
                        for object_key in objects {
                            let object_value = parser.get_object(*object_key);
                            if let Some(ParserObject::VarRef { var_name }) = object_value {
                                let var_value =
                                    scope.get_variable(var_name, &mut scope_path.iter())?;
                                capture_area.insert(
                                    var_name.clone(),
                                    LenarValue::Ref(Rc::new(RefCell::new(var_value))),
                                );
                            }
                        }
                    }
                    capture_area
                };

                // Anonymous function created at runtime
                #[derive(Debug)]
                struct Function {
                    capture_area: HashMap<String, LenarValue>,
                    arguments_block: usize,
                    block_value: usize,
                }

                impl RuntimeFunction for Function {
                    fn call(
                        &mut self,
                        mut args: Vec<LenarValue>,
                        parser: &Arc<Parser>,
                    ) -> LenarResult<LenarValue> {
                        // Anonymous functions do not capture any values by default.
                        let mut scope = Scope::default();

                        scope.setup_globals();

                        // Define each argument as a variable in the function scope
                        let arguments_block = parser.get_object(self.arguments_block).unwrap();
                        if let ParserObject::Block { objects } = arguments_block {
                            for object in objects {
                                let arg_object = parser.get_object(*object).unwrap();
                                if let ParserObject::VarRef { var_name } = arg_object {
                                    let arg_value = args.remove(0);
                                    scope.variables.insert(var_name.to_owned(), arg_value);
                                }
                            }
                        }

                        // Inject every captured value in the function scope
                        for (captured_var, value) in self.capture_area.iter() {
                            scope
                                .variables
                                .insert(captured_var.to_owned(), value.clone());
                        }

                        let block_object = parser.get_object(self.block_value).unwrap();

                        evaluate_object(block_object, parser, &mut scope, &[])
                    }

                    fn get_name(&self) -> &str {
                        "Anonymous"
                    }
                }
                Ok(LenarValue::Function(Rc::new(RefCell::new(Function {
                    capture_area,
                    arguments_block: *arguments_block,
                    block_value: *block_value,
                }))))
            }
            ParserObject::IfDef {
                condition_block: expr,
                block_value,
            } => {
                let expr_object = parser.get_object(*expr).unwrap();
                let expr_res = evaluate_object(expr_object, parser, scope, scope_path)?;

                // If the condition expression returns a `true` it
                // will evaluate the actual block
                if LenarValue::Bool(true) == expr_res {
                    let expr_body_object = parser.get_object(*block_value).unwrap();
                    evaluate_object(expr_body_object, parser, scope, scope_path)
                } else {
                    Ok(LenarValue::Void)
                }
            }
            ParserObject::NumberVal { value } => Ok(LenarValue::Usize(*value)),
        }
    }
}
