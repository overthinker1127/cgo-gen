use crate::{
    config::{Config, WRAPPER_PREFIX},
    domain::kind::{IrFunctionKind, IrTypeKind},
};

use super::{
    IrFunction, IrType, base_model_cpp_type, byte_array_length, enum_base_cpp_type,
    fixed_array_elem_type, fixed_array_length,
};

pub(super) fn symbol_name(
    _config: &Config,
    namespace: &[String],
    owner: &str,
    tail: &str,
) -> String {
    let mut parts = vec![WRAPPER_PREFIX.to_string()];
    parts.extend(namespace.iter().map(|item| format_symbol_part(item)));
    if !owner.is_empty() {
        parts.push(format_symbol_part(owner));
    }
    parts.push(format_symbol_part(tail));
    parts.join("_")
}

pub(crate) fn overload_suffix(function: &IrFunction) -> String {
    let params = if function.method_of.is_some()
        && matches!(
            function.kind,
            IrFunctionKind::Method | IrFunctionKind::Destructor
        ) {
        &function.params[1..]
    } else {
        &function.params[..]
    };

    let mut parts = if params.is_empty() {
        vec!["void".to_string()]
    } else {
        params
            .iter()
            .map(|param| type_signature_token(&param.ty))
            .collect::<Vec<_>>()
    };

    if function.kind == IrFunctionKind::Method {
        parts.push(
            if function.is_const == Some(true) {
                "const"
            } else {
                "mut"
            }
            .to_string(),
        );
    }

    parts.join("_")
}

fn type_signature_token(ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Primitive | IrTypeKind::Void => sanitize_symbol_token(&ty.cpp_type),
        IrTypeKind::Enum => format!(
            "enum_{}",
            sanitize_symbol_token(&enum_base_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::CString => {
            if ty.cpp_type.contains("const")
                || matches!(ty.cpp_type.as_str(), "NPCSTR" | "NPSTRC" | "NPCSTRC")
            {
                "c_str".to_string()
            } else {
                "mut_c_str".to_string()
            }
        }
        IrTypeKind::FixedByteArray => {
            let n = byte_array_length(&ty.cpp_type).unwrap_or(0);
            format!("byte_array_{n}")
        }
        IrTypeKind::String => "string".to_string(),
        IrTypeKind::Pointer => format!(
            "ptr_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('*'))
        ),
        IrTypeKind::Reference => format!(
            "ref_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('&'))
        ),
        IrTypeKind::ExternStructPointer => format!(
            "extern_ptr_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.c_type))
        ),
        IrTypeKind::ExternStructReference => format!(
            "extern_ref_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.c_type))
        ),
        IrTypeKind::Opaque => format!(
            "opaque_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelReference => format!(
            "model_ref_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelPointer => format!(
            "model_ptr_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelValue => format!(
            "model_value_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::Callback => format!("callback_{}", sanitize_symbol_token(&ty.cpp_type)),
        IrTypeKind::FixedArray => {
            let n = fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let elem = fixed_array_elem_type(&ty.cpp_type).unwrap_or("unknown");
            format!("array_{n}_{}", sanitize_symbol_token(elem))
        }
        IrTypeKind::FixedModelArray => {
            let n = fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let handle = ty.handle.as_deref().unwrap_or("unknown");
            format!("model_array_{n}_{}", sanitize_symbol_token(handle))
        }
    }
}

pub(super) fn sanitize_symbol_token(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;

    for ch in value.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else {
            None
        };

        match normalized {
            Some(ch) => {
                out.push(ch);
                last_was_underscore = false;
            }
            None if !last_was_underscore => {
                out.push('_');
                last_was_underscore = true;
            }
            None => {}
        }
    }

    out.trim_matches('_').to_string()
}

fn format_symbol_part(value: &str) -> String {
    value.to_string()
}

pub(super) fn cpp_qualified(namespace: &[String], leaf: &str) -> String {
    if namespace.is_empty() {
        leaf.to_string()
    } else {
        format!("{}::{}", namespace.join("::"), leaf)
    }
}

pub fn flatten_cpp_name(namespace: &[String], leaf: &str) -> String {
    if namespace.is_empty() {
        leaf.to_string()
    } else {
        format!("{}{}", namespace.join(""), leaf)
    }
}

pub(super) fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
}
