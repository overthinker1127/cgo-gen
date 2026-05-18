use std::path::PathBuf;

use serde::Serialize;

use crate::{domain::kind::RecordKind, parsing::macros::MacroConstantKind};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParsedApi {
    pub headers: Vec<String>,
    pub functions: Vec<CppFunction>,
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
