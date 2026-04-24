use serde::{Deserialize, Serialize};

/// Classifies the type of an IR type node.
/// Serializes to the same string values as the previous String-based field
/// to maintain YAML/JSON wire format compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrTypeKind {
    #[serde(rename = "void")]
    Void,
    #[serde(rename = "primitive")]
    Primitive,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "c_string")]
    CString,
    #[serde(rename = "fixed_byte_array")]
    FixedByteArray,
    #[serde(rename = "fixed_array")]
    FixedArray,
    #[serde(rename = "fixed_model_array")]
    FixedModelArray,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "pointer")]
    Pointer,
    #[serde(rename = "reference")]
    Reference,
    #[serde(rename = "opaque")]
    Opaque,
    #[serde(rename = "callback")]
    Callback,
    #[serde(rename = "extern_struct_reference")]
    ExternStructReference,
    #[serde(rename = "extern_struct_pointer")]
    ExternStructPointer,
    #[serde(rename = "model_reference")]
    ModelReference,
    #[serde(rename = "model_pointer")]
    ModelPointer,
    #[serde(rename = "model_value")]
    ModelValue,
}

/// Distinguishes parsed C++ record declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecordKind {
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "class")]
    Class,
}

/// Classifies the role of an IR function node.
/// Serializes to the same string values as the previous String-based field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrFunctionKind {
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "constructor")]
    Constructor,
    #[serde(rename = "destructor")]
    Destructor,
}

/// Distinguishes getter vs setter field accessors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldAccessKind {
    #[serde(rename = "get")]
    Get,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "get_at")]
    GetAt,
    #[serde(rename = "set_at")]
    SetAt,
}
