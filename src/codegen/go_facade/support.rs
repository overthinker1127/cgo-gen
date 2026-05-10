use std::{collections::BTreeMap, path::Path};

use crate::{
    codegen::ir_norm,
    domain::kind::{IrFunctionKind, IrTypeKind},
    ir::{IrFunction, IrType},
    pipeline::context::PipelineContext,
};

use super::{AnalyzedFacadeClass, CallbackUsage};

pub(super) fn zero_value_for_go_type(go_type: &str) -> &'static str {
    match go_type {
        "bool" => "false",
        "string" => "\"\"",
        "float32" | "float64" | "int" | "int8" | "int16" | "int32" | "int64" | "uint8"
        | "uint16" | "uint32" | "uint64" | "uintptr" => "0",
        _ => "0",
    }
}

pub(super) fn go_type_for_ir(ty: &IrType) -> Option<&'static str> {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => Some("string"),
        IrTypeKind::Enum => Some("int64"),
        IrTypeKind::Primitive => {
            primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
        }
        _ => None,
    }
}

pub(super) fn go_value_type(config: &PipelineContext, ty: &IrType) -> Option<String> {
    if ty.kind == IrTypeKind::Enum {
        return config.known_enum_go_type(&ty.cpp_type);
    }
    go_type_for_ir(ty).map(str::to_string)
}

pub(super) fn go_type_for_reference(ty: &IrType) -> Option<&'static str> {
    if ty.kind != IrTypeKind::Reference {
        return None;
    }
    primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
}

pub(super) fn cgo_cast_type(ty: &IrType) -> &'static str {
    primitive_cgo_cast_type(&ty.cpp_type)
        .or_else(|| primitive_cgo_cast_type(&ty.c_type))
        .unwrap_or_else(|| {
            panic!(
                "unsupported type in cgo_cast_type: cpp_type={:?}, c_type={:?}",
                ty.cpp_type, ty.c_type
            )
        })
}

pub fn primitive_go_type_pub(value: &str) -> Option<&'static str> {
    primitive_go_type(value)
}

pub(super) fn primitive_go_type(value: &str) -> Option<&'static str> {
    match normalize_type_key(value).as_str() {
        "bool" => Some("bool"),
        "float" => Some("float32"),
        "double" => Some("float64"),
        "int8" | "int8_t" | "signedchar" => Some("int8"),
        "int16" | "int16_t" | "short" => Some("int16"),
        "int32" | "int32_t" => Some("int32"),
        "int64" | "int64_t" | "long" | "longlong" => Some("int64"),
        "uint8" | "uint8_t" | "unsignedchar" => Some("uint8"),
        "uint16" | "uint16_t" | "unsignedshort" => Some("uint16"),
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => Some("uint32"),
        "int" => Some("int32"),
        "uint64" | "uint64_t" | "unsignedlong" | "unsignedlonglong" => Some("uint64"),
        "size_t" => Some("uintptr"),
        _ => None,
    }
}

pub(super) fn primitive_cgo_cast_type(value: &str) -> Option<&'static str> {
    match normalize_type_key(value).as_str() {
        "bool" => Some("C.bool"),
        "float" => Some("C.float"),
        "double" => Some("C.double"),
        "int8" | "int8_t" | "signedchar" => Some("C.int8_t"),
        "int16" | "int16_t" | "short" => Some("C.int16_t"),
        "int32" | "int32_t" => Some("C.int32_t"),
        "int64" | "int64_t" => Some("C.int64_t"),
        "uint8" | "uint8_t" | "unsignedchar" => Some("C.uint8_t"),
        "uint16" | "uint16_t" | "unsignedshort" => Some("C.uint16_t"),
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => Some("C.uint32_t"),
        "uint64" | "uint64_t" => Some("C.uint64_t"),
        "unsignedlong" => Some("C.ulong"),
        "unsignedlonglong" => Some("C.ulonglong"),
        "int" => Some("C.int"),
        "long" => Some("C.long"),
        "longlong" | "signedlonglong" => Some("C.longlong"),
        "size_t" => Some("C.size_t"),
        _ => None,
    }
}

pub(super) fn fixed_array_c_elem_type(ty: &IrType) -> &str {
    ty.c_type.trim().trim_end_matches('*').trim()
}

pub(super) fn fixed_array_go_elem_type(ty: &IrType) -> &'static str {
    ir_norm::fixed_array_elem_type(&ty.cpp_type)
        .and_then(primitive_go_type)
        .or_else(|| primitive_go_type(fixed_array_c_elem_type(ty)))
        .unwrap_or("int32")
}

pub(super) fn fixed_array_cgo_elem_type(ty: &IrType) -> &'static str {
    ir_norm::fixed_array_elem_type(&ty.cpp_type)
        .and_then(primitive_cgo_cast_type)
        .or_else(|| primitive_cgo_cast_type(fixed_array_c_elem_type(ty)))
        .unwrap_or("C.int32_t")
}

pub(super) fn normalize_type_key(value: &str) -> String {
    value
        .replace(' ', "")
        .trim_start_matches("const")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .to_string()
}

pub(super) fn go_export_name(value: &str) -> String {
    let mut out = String::new();
    for (index, segment) in value
        .split('_')
        .filter(|segment| !segment.is_empty())
        .enumerate()
    {
        if index > 0
            && segment.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            && !out.is_empty()
        {
            out.push('_');
        }
        for token in split_pascal_tokens(segment)
            .into_iter()
            .filter(|token| !token.is_empty())
        {
            let mut chars = token.chars();
            let Some(first) = chars.next() else {
                continue;
            };
            out.push(first.to_ascii_uppercase());
            out.push_str(&chars.collect::<String>());
        }
    }
    out
}

pub(super) fn go_constructor_export_names(class: &AnalyzedFacadeClass<'_>) -> Vec<String> {
    let base_names = class
        .constructors
        .iter()
        .map(|constructor| go_constructor_base_export_name(class, constructor))
        .collect::<Vec<_>>();
    let mut base_counts = BTreeMap::<String, usize>::new();
    for base in &base_names {
        *base_counts.entry(base.clone()).or_insert(0) += 1;
    }

    let mut used_names = BTreeMap::<String, usize>::new();
    class
        .constructors
        .iter()
        .zip(base_names)
        .map(|(constructor, base)| {
            let mut name = base.clone();
            if base_counts.get(&base).copied().unwrap_or(0) > 1 {
                let suffix = go_constructor_overload_suffix(constructor);
                if !suffix.is_empty() {
                    name.push_str(&suffix);
                }
            }

            let count = used_names.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                name.push_str(&count.to_string());
            }
            name
        })
        .collect()
}

pub(super) fn go_constructor_base_export_name(
    class: &AnalyzedFacadeClass<'_>,
    constructor: &IrFunction,
) -> String {
    let base = format!("New{}", class.go_name);
    if class.constructors.len() <= 1 || constructor.params.is_empty() {
        return base;
    }
    if is_copy_constructor(constructor) {
        return format!("{base}FromCopy");
    }

    let param_names = constructor
        .params
        .iter()
        .map(|param| go_export_name(&sanitize_go_token(&param.name)))
        .collect::<String>();
    if param_names.is_empty() {
        base
    } else {
        format!("{base}With{param_names}")
    }
}

pub(super) fn is_copy_constructor(constructor: &IrFunction) -> bool {
    if constructor.kind != IrFunctionKind::Constructor || constructor.params.len() != 1 {
        return false;
    }
    let Some(owner) = constructor.owner_cpp_type.as_deref() else {
        return false;
    };
    let param_ty = &constructor.params[0].ty;
    matches!(
        param_ty.kind,
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
    ) && base_model_cpp_type(&param_ty.cpp_type) == base_model_cpp_type(owner)
}

pub(super) fn go_constructor_overload_suffix(constructor: &IrFunction) -> String {
    constructor
        .params
        .iter()
        .map(|param| go_overload_token(&param.ty))
        .collect()
}

pub(super) fn go_facade_export_name(function: &IrFunction) -> String {
    let base = go_export_name(&leaf_cpp_name(&function.cpp_name));
    if !has_disambiguated_raw_overload_suffix(function) {
        return base;
    }

    format!("{base}{}", go_overload_suffix(function))
}

pub(super) fn go_method_export_name(function: &IrFunction) -> String {
    let base = go_export_name(method_name(function));
    if !has_disambiguated_raw_overload_suffix(function) {
        return base;
    }

    format!("{base}{}", go_overload_suffix(function))
}

pub(super) fn has_disambiguated_raw_overload_suffix(function: &IrFunction) -> bool {
    let raw_suffix = ir_norm::overload_suffix(function);
    let Some((_, tail)) = function.name.rsplit_once("__") else {
        return false;
    };

    if tail == raw_suffix {
        return true;
    }

    let Some(rest) = tail.strip_prefix(&format!("{raw_suffix}_")) else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit())
}

pub(super) fn go_overload_suffix(function: &IrFunction) -> String {
    let params = if function.method_of.is_some() {
        function.params.iter().skip(1).collect::<Vec<_>>()
    } else {
        function.params.iter().collect::<Vec<_>>()
    };

    let mut suffix = params
        .iter()
        .map(|param| go_overload_token(&param.ty))
        .collect::<String>();
    if suffix.is_empty() {
        // No params: const version gets the clean name, non-const gets "Mut"
        if function.is_const != Some(true) {
            suffix = "Mut".to_string();
        }
    } else {
        // Has params: append "Const" to distinguish const overloads
        if function.is_const == Some(true) {
            suffix.push_str("Const");
        }
    }
    suffix
}

pub(super) fn go_overload_token(ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Callback => format!("{}Callback", go_export_name(&leaf_cpp_name(&ty.cpp_type))),
        IrTypeKind::String | IrTypeKind::CString => string_overload_token(ty),
        IrTypeKind::Enum => go_export_name(&sanitize_go_token(&enum_base_cpp_type(&ty.cpp_type))),
        IrTypeKind::Primitive => primitive_overload_token(ty),
        IrTypeKind::ExternStructReference => extern_struct_overload_token(ty, "Ref"),
        IrTypeKind::ExternStructPointer => extern_struct_overload_token(ty, "Ptr"),
        IrTypeKind::ModelReference => format!(
            "{}Ref",
            go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
                &ty.cpp_type
            )))
        ),
        IrTypeKind::ModelPointer => model_pointer_overload_token(ty),
        IrTypeKind::ModelValue => format!(
            "{}Value",
            go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
                &ty.cpp_type
            )))
        ),
        _ => go_export_name(&sanitize_go_token(&ty.cpp_type)),
    }
}

pub(super) fn model_pointer_overload_token(ty: &IrType) -> String {
    let base = go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
        &ty.cpp_type,
    )));
    let depth = model_pointer_depth(ty);
    format!("{base}{}", "Ptr".repeat(depth.max(1)))
}

pub(super) fn model_pointer_depth(ty: &IrType) -> usize {
    let cpp_depth = ty.cpp_type.chars().filter(|ch| *ch == '*').count();
    if cpp_depth > 0 {
        return cpp_depth;
    }
    ty.c_type.chars().filter(|ch| *ch == '*').count().max(1)
}

pub(super) fn extern_struct_overload_token(ty: &IrType, suffix: &str) -> String {
    let base = base_model_cpp_type(&ty.c_type);
    let tag = base.strip_prefix("struct ").unwrap_or(&base);
    format!("{}{}", go_export_name(&sanitize_go_token(tag)), suffix)
}

pub(super) fn primitive_overload_token(ty: &IrType) -> String {
    let cpp_key = normalize_type_key(&ty.cpp_type);
    let c_key = normalize_type_key(&ty.c_type);
    if cpp_key != c_key && !is_builtin_primitive_key(&cpp_key) {
        return go_export_name(&sanitize_go_token(&ty.cpp_type));
    }
    go_type_for_ir(ty)
        .map(go_export_name)
        .unwrap_or_else(|| go_export_name(&sanitize_go_token(&ty.cpp_type)))
}

pub(super) fn string_overload_token(ty: &IrType) -> String {
    let cpp_key = normalize_type_key(&ty.cpp_type);
    let c_key = normalize_type_key(&ty.c_type);
    if cpp_key != c_key && !cpp_key.is_empty() {
        return go_export_name(&sanitize_go_token(&ty.cpp_type));
    }
    "String".to_string()
}

pub(super) fn is_builtin_primitive_key(value: &str) -> bool {
    matches!(
        value,
        "bool"
            | "float"
            | "double"
            | "int8"
            | "int8_t"
            | "signedchar"
            | "int16"
            | "int16_t"
            | "short"
            | "int32"
            | "int32_t"
            | "int"
            | "int64"
            | "int64_t"
            | "long"
            | "longlong"
            | "uint8"
            | "uint8_t"
            | "unsignedchar"
            | "uint16"
            | "uint16_t"
            | "unsignedshort"
            | "uint32"
            | "uint32_t"
            | "unsignedint"
            | "unsigned"
            | "uint64"
            | "uint64_t"
            | "unsignedlong"
            | "unsignedlonglong"
            | "size_t"
    )
}

pub(super) fn callback_state_name(usage: &CallbackUsage<'_>) -> String {
    callback_state_name_from_function(usage.function, usage.param_index)
}

pub(super) fn callback_state_name_from_function(function: &IrFunction, index: usize) -> String {
    format!("{}_cb{}", sanitize_go_token(&function.name), index)
}

pub(super) fn callback_go_export_name(usage: &CallbackUsage<'_>) -> String {
    format!(
        "go_{}_cb{}",
        sanitize_go_token(&usage.function.name),
        usage.param_index
    )
}

pub(super) fn callback_cgo_param_type(ty: &IrType) -> &'static str {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => "*C.char",
        _ => cgo_cast_type_from_c_type(&ty.c_type),
    }
}

pub(super) fn callback_cgo_return_type(ty: &IrType) -> &'static str {
    cgo_cast_type_from_c_type(&ty.c_type)
}

pub(super) fn render_callback_go_arg(ty: &IrType, name: &str) -> String {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => format!("C.GoString({name})"),
        _ => format!("{}({})", callback_go_type(ty), name),
    }
}

pub(super) fn callback_go_type(ty: &IrType) -> &'static str {
    go_type_for_ir(ty).unwrap_or_else(|| go_type_from_c_type(&ty.c_type))
}

pub(super) fn go_type_from_c_type(c_type: &str) -> &'static str {
    match normalize_type_key(c_type).as_str() {
        "bool" => "bool",
        "float" => "float32",
        "double" => "float64",
        "int8" | "int8_t" => "int8",
        "int16" | "int16_t" | "short" => "int16",
        "int32" | "int32_t" | "int" => "int32",
        "int64" | "int64_t" | "long" => "int64",
        "uint8" | "uint8_t" => "uint8",
        "uint16" | "uint16_t" => "uint16",
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => "uint32",
        "uint64" | "uint64_t" | "unsignedlong" | "unsignedlonglong" => "uint64",
        "size_t" => "uintptr",
        _ => "int",
    }
}

pub(super) fn cgo_cast_type_from_c_type(c_type: &str) -> &'static str {
    match normalize_type_key(c_type).as_str() {
        "bool" => "C.bool",
        "float" => "C.float",
        "double" => "C.double",
        "int8" | "int8_t" => "C.int8_t",
        "int16" | "int16_t" => "C.int16_t",
        "int32" | "int32_t" => "C.int32_t",
        "int64" | "int64_t" => "C.int64_t",
        "uint8" | "uint8_t" => "C.uint8_t",
        "uint16" | "uint16_t" => "C.uint16_t",
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => "C.uint32_t",
        "uint64" | "uint64_t" => "C.uint64_t",
        "unsignedlonglong" => "C.ulonglong",
        "longlong" | "signedlonglong" => "C.longlong",
        "ulong" | "unsignedlong" => "C.ulong",
        "short" => "C.short",
        "long" => "C.long",
        "size_t" => "C.size_t",
        _ => "C.int",
    }
}

pub(super) fn base_model_cpp_type(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

pub(super) fn enum_base_cpp_type(value: &str) -> String {
    let base = base_model_cpp_type(value);
    base.strip_prefix("enum ")
        .unwrap_or(&base)
        .trim()
        .to_string()
}

pub(super) fn extern_struct_go_type(ty: &IrType) -> Option<String> {
    let base = base_model_cpp_type(&ty.c_type);
    let tag = base.strip_prefix("struct ")?;
    Some(format!("*C.struct_{}", sanitize_go_token(tag)))
}

pub(super) fn ir_uses_struct_timeval(
    functions: &[&IrFunction],
    classes: &[AnalyzedFacadeClass<'_>],
) -> bool {
    functions
        .iter()
        .flat_map(|function| {
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        })
        .chain(classes.iter().flat_map(|class| {
            class
                .constructors
                .iter()
                .flat_map(|ctor| {
                    std::iter::once(&ctor.returns)
                        .chain(ctor.params.iter().map(|param| &param.ty))
                        .collect::<Vec<_>>()
                })
                .chain(std::iter::once(&class.destructor.returns))
                .chain(class.destructor.params.iter().map(|param| &param.ty))
                .chain(class.methods.iter().flat_map(|function| {
                    std::iter::once(&function.returns)
                        .chain(function.params.iter().map(|param| &param.ty))
                }))
        }))
        .any(|ty| {
            matches!(
                ty.kind,
                IrTypeKind::ExternStructReference | IrTypeKind::ExternStructPointer
            ) && base_model_cpp_type(&ty.c_type) == "struct timeval"
        })
}

pub(super) fn sanitize_go_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

pub(super) fn method_name(function: &IrFunction) -> &str {
    function
        .cpp_name
        .rsplit("::")
        .next()
        .unwrap_or(&function.cpp_name)
}

pub(super) fn receiver_name(value: &str) -> String {
    value
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase().to_string())
        .unwrap_or_else(|| "v".to_string())
}

pub(super) fn split_pascal_tokens(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let mut start = 0;
    for index in 1..chars.len() {
        let prev = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        let boundary = (prev.is_lowercase() && current.is_uppercase())
            || (prev.is_ascii_digit() && !current.is_ascii_digit())
            || (!prev.is_ascii_digit() && current.is_ascii_digit())
            || (prev.is_uppercase()
                && current.is_uppercase()
                && next.map(|ch| ch.is_lowercase()).unwrap_or(false));

        if boundary {
            tokens.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    tokens.push(chars[start..].iter().collect::<String>());
    tokens
}

pub(super) fn leaf_cpp_name(value: &str) -> String {
    value.rsplit("::").next().unwrap_or(value).to_string()
}

pub(super) fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
}

pub(super) fn go_package_name(path: &Path) -> String {
    let source = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("bindings");
    let sanitized = source
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "bindings".to_string()
    } else {
        sanitized
    }
}
