use std::path::PathBuf;

use serde::Serialize;

use crate::{domain::kind::RecordKind, parsing::macros::MacroConstantKind};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParsedApi {
    pub headers: Vec<String>,
    pub functions: Vec<CppFunction>,
    pub free_operators: Vec<CppOperator>,
    pub records: Vec<CppRecord>,
    pub enums: Vec<CppEnum>,
    pub macros: Vec<CppMacroConstant>,
    pub callbacks: Vec<CppCallbackTypedef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppRecord {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub kind: RecordKind,
    pub fields: Vec<CppField>,
    pub methods: Vec<CppMethod>,
    pub operators: Vec<CppOperator>,
    pub constructors: Vec<CppConstructor>,
    pub has_destructor: bool,
    pub has_declared_constructor: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppFunction {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppMethod {
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppOperator {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub owner: Option<String>,
    pub spelling: String,
    pub token: CppOperatorToken,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
    pub is_const: bool,
    pub has_header_definition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CppOperatorToken {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Equal,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    NotEqual,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    Spaceship,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Not,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    LessLess,
    GreaterGreater,
    LessLessAssign,
    GreaterGreaterAssign,
    AndAnd,
    OrOr,
    Comma,
    Arrow,
    ArrowStar,
    Array,
    Func,
    Increment,
    Decrement,
    Conversion(String),
    Unsupported(String),
}

impl Serialize for CppOperatorToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.wire_name())
    }
}

impl CppOperatorToken {
    fn wire_name(&self) -> String {
        match self {
            Self::Plus => "plus".to_string(),
            Self::Minus => "minus".to_string(),
            Self::Multiply => "multiply".to_string(),
            Self::Divide => "divide".to_string(),
            Self::Modulo => "modulo".to_string(),
            Self::Equal => "equal".to_string(),
            Self::Assign => "assign".to_string(),
            Self::PlusAssign => "plus_assign".to_string(),
            Self::MinusAssign => "minus_assign".to_string(),
            Self::MultiplyAssign => "multiply_assign".to_string(),
            Self::DivideAssign => "divide_assign".to_string(),
            Self::ModuloAssign => "modulo_assign".to_string(),
            Self::NotEqual => "not_equal".to_string(),
            Self::Less => "less".to_string(),
            Self::LessEq => "less_eq".to_string(),
            Self::Greater => "greater".to_string(),
            Self::GreaterEq => "greater_eq".to_string(),
            Self::Spaceship => "spaceship".to_string(),
            Self::Amp => "amp".to_string(),
            Self::Pipe => "pipe".to_string(),
            Self::Caret => "caret".to_string(),
            Self::Tilde => "tilde".to_string(),
            Self::Not => "not".to_string(),
            Self::AmpAssign => "amp_assign".to_string(),
            Self::PipeAssign => "pipe_assign".to_string(),
            Self::CaretAssign => "caret_assign".to_string(),
            Self::LessLess => "less_less".to_string(),
            Self::GreaterGreater => "greater_greater".to_string(),
            Self::LessLessAssign => "less_less_assign".to_string(),
            Self::GreaterGreaterAssign => "greater_greater_assign".to_string(),
            Self::AndAnd => "and_and".to_string(),
            Self::OrOr => "or_or".to_string(),
            Self::Comma => "comma".to_string(),
            Self::Arrow => "arrow".to_string(),
            Self::ArrowStar => "arrow_star".to_string(),
            Self::Array => "array".to_string(),
            Self::Func => "func".to_string(),
            Self::Increment => "increment".to_string(),
            Self::Decrement => "decrement".to_string(),
            Self::Conversion(target) => format!("conversion_{}", token_suffix(target)),
            Self::Unsupported(token) => format!("unsupported_{}", token_suffix(token)),
        }
    }
}

fn token_suffix(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            out.push('_');
            last_was_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppConstructor {
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppField {
    pub name: String,
    pub ty: String,
    pub canonical_ty: String,
    pub is_function_pointer: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppParam {
    pub name: String,
    pub ty: String,
    pub canonical_ty: String,
    pub is_function_pointer: bool,
    pub callback_typedef: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_default: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppCallbackTypedef {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnum {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    #[serde(skip)]
    pub is_anonymous: bool,
    pub variants: Vec<CppEnumVariant>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnumVariant {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppMacroConstant {
    pub source_header: PathBuf,
    pub name: String,
    pub kind: MacroConstantKind,
    pub value: String,
}

fn is_false(value: &bool) -> bool {
    !*value
}
