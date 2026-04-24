use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::{
    codegen::{go_facade, ir_norm},
    domain::{
        kind::{IrFunctionKind, IrTypeKind},
        model_projection::{ModelProjection, ModelProjectionField},
    },
    ir::{IrFunction, IrModule, IrType},
    pipeline::context::PipelineContext,
};

pub fn collect_known_model_projections(
    ctx: &PipelineContext,
    ir: &IrModule,
) -> Result<Vec<ModelProjection>> {
    build_all_model_projections(ctx, ir)
}

fn build_all_model_projections(
    ctx: &PipelineContext,
    ir: &IrModule,
) -> Result<Vec<ModelProjection>> {
    let constructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Constructor)
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner.to_string(), function))
        })
        .collect::<BTreeMap<_, _>>();
    let destructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Destructor)
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner.to_string(), function))
        })
        .collect::<BTreeMap<_, _>>();

    let mut methods_by_owner = BTreeMap::<String, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Method)
    {
        let Some(owner) = &function.owner_cpp_type else {
            continue;
        };
        methods_by_owner
            .entry(owner.clone())
            .or_default()
            .push(function);
    }

    let mut projections = Vec::new();
    for (owner, class_methods) in methods_by_owner {
        if let Some(projection) = build_model_projection(
            ctx,
            &owner,
            &class_methods,
            constructors.get(&owner),
            destructors.get(&owner),
        )? {
            projections.push(projection);
        }
    }

    Ok(projections)
}

fn build_model_projection(
    ctx: &PipelineContext,
    owner: &str,
    class_methods: &[&IrFunction],
    constructor: Option<&&IrFunction>,
    destructor: Option<&&IrFunction>,
) -> Result<Option<ModelProjection>> {
    let setters = class_methods
        .iter()
        .filter_map(|function| {
            setter_suffix(function).map(|suffix| (suffix.to_string(), *function))
        })
        .collect::<BTreeMap<_, _>>();

    let mut fields = Vec::new();
    let mut seen = BTreeMap::<String, ()>::new();
    for function in class_methods {
        let Some(suffix) = getter_suffix(function) else {
            continue;
        };
        let Some(setter) = setters.get(suffix) else {
            continue;
        };
        if seen.insert(suffix.to_string(), ()).is_some() {
            continue;
        }

        let Some(getter_ty) = go_model_field_type(ctx, &function.returns) else {
            continue;
        };
        let Some(setter_param) = setter.params.get(1) else {
            bail!(
                "setter `{}` on `{owner}` is missing its value parameter",
                setter.cpp_name
            );
        };
        let Some(setter_ty) = go_model_field_type(ctx, &setter_param.ty) else {
            continue;
        };

        if getter_ty != setter_ty {
            continue;
        }

        fields.push(ModelProjectionField {
            go_name: go_field_name(suffix),
            go_type: getter_ty.to_string(),
            getter_symbol: function.name.clone(),
            setter_symbol: setter.name.clone(),
            return_kind: function.returns.kind,
        });
    }

    if fields.is_empty() {
        return Ok(None);
    }

    let constructor = constructor.ok_or_else(|| {
        anyhow::anyhow!("model projection `{owner}` is missing a constructor wrapper")
    })?;
    let destructor = destructor.ok_or_else(|| {
        anyhow::anyhow!("model projection `{owner}` is missing a destructor wrapper")
    })?;
    let handle_name = constructor
        .returns
        .handle
        .clone()
        .or_else(|| {
            destructor
                .params
                .first()
                .and_then(|param| param.ty.handle.clone())
        })
        .unwrap_or_else(|| format!("{}Handle", flatten_qualified_cpp_name(owner)));
    let go_name = handle_name
        .strip_suffix("Handle")
        .map(go_export_name)
        .unwrap_or_else(|| go_export_name(&leaf_cpp_name(owner)));

    Ok(Some(ModelProjection {
        cpp_type: owner.to_string(),
        go_name,
        handle_name,
        constructor_symbol: constructor.name.clone(),
        destructor_symbol: destructor.name.clone(),
        fields,
    }))
}

fn go_model_field_type(ctx: &PipelineContext, ty: &IrType) -> Option<String> {
    match ty.kind {
        IrTypeKind::Enum => ctx
            .known_enum_go_type(&ty.cpp_type)
            .or_else(|| go_type_for_ir(ty).map(str::to_string)),
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
            Some(format!("*{}", go_model_return_type(ctx, ty)))
        }
        IrTypeKind::FixedByteArray => Some("[]byte".to_string()),
        IrTypeKind::FixedArray => {
            let elem = ir_norm::fixed_array_elem_type(&ty.cpp_type)?;
            let go_elem = go_facade::primitive_go_type_pub(elem)?;
            Some(format!("[]{go_elem}"))
        }
        IrTypeKind::FixedModelArray => {
            let go_name = go_model_return_type(ctx, ty);
            Some(format!("[]*{go_name}"))
        }
        _ => go_type_for_ir(ty).map(str::to_string),
    }
}

fn getter_suffix(function: &IrFunction) -> Option<&str> {
    if function.kind != IrFunctionKind::Method
        || function.params.len() != 1
        || function.returns.kind == IrTypeKind::Void
    {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Get"))
        .filter(|suffix| !suffix.is_empty())
}

fn setter_suffix(function: &IrFunction) -> Option<&str> {
    if function.kind != IrFunctionKind::Method
        || function.params.len() != 2
        || function.returns.kind != IrTypeKind::Void
    {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Set"))
        .filter(|suffix| !suffix.is_empty())
}

fn go_field_name(value: &str) -> String {
    value
        .split('_')
        .flat_map(split_pascal_tokens)
        .map(|token| match token.to_ascii_lowercase().as_str() {
            "id" => "ID".to_string(),
            "url" => "URL".to_string(),
            "db" => "DB".to_string(),
            "api" => "API".to_string(),
            "http" => "HTTP".to_string(),
            "https" => "HTTPS".to_string(),
            "json" => "JSON".to_string(),
            "xml" => "XML".to_string(),
            other if token.chars().all(|ch| ch.is_uppercase()) => other.to_ascii_uppercase(),
            _ => token,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn go_model_return_type(ctx: &PipelineContext, ty: &IrType) -> String {
    ctx.known_model_projection(&ty.cpp_type)
        .map(|projection| projection.go_name.clone())
        .unwrap_or_else(|| leaf_cpp_name(&base_model_cpp_type(&ty.cpp_type)))
}

fn go_type_for_ir(ty: &IrType) -> Option<&'static str> {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => Some("string"),
        IrTypeKind::FixedByteArray => Some("[]byte"),
        IrTypeKind::Enum => Some("int64"),
        IrTypeKind::Primitive => {
            primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
        }
        _ => None,
    }
}

fn primitive_go_type(value: &str) -> Option<&'static str> {
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
        "int" => Some("int"),
        "uint64" | "uint64_t" | "unsignedlong" | "unsignedlonglong" => Some("uint64"),
        "size_t" => Some("uintptr"),
        _ => None,
    }
}

fn normalize_type_key(value: &str) -> String {
    value
        .replace(' ', "")
        .trim_start_matches("const")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .to_string()
}

fn go_export_name(value: &str) -> String {
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

fn split_pascal_tokens(value: &str) -> Vec<String> {
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

fn leaf_cpp_name(value: &str) -> String {
    value.rsplit("::").next().unwrap_or(value).to_string()
}

fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
}

fn base_model_cpp_type(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        ir::{IrFunction, IrParam},
    };

    fn test_context() -> PipelineContext {
        PipelineContext::new(Config::default()).with_known_model_projections(vec![
            ModelProjection {
                cpp_type: "ThingModel".to_string(),
                handle_name: "ThingModelHandle".to_string(),
                go_name: "ThingModel".to_string(),
                constructor_symbol: "cgowrap_ThingModel_new".to_string(),
                destructor_symbol: "cgowrap_ThingModel_delete".to_string(),
                fields: vec![],
            },
        ])
    }

    fn model_type(kind: IrTypeKind, cpp_type: &str) -> IrType {
        IrType {
            kind,
            cpp_type: cpp_type.to_string(),
            c_type: "ThingModelHandle*".to_string(),
            handle: Some("ThingModelHandle".to_string()),
        }
    }

    #[test]
    fn projects_getter_setter_pairs_into_model_projection() {
        let ctx = test_context();
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: crate::ir::SupportMetadata {
                parser_backend: "libclang".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
            functions: vec![
                IrFunction {
                    name: "cgowrap_ThingModel_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "ThingModel".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "ThingModel".to_string(),
                        c_type: "ThingModelHandle*".to_string(),
                        handle: Some("ThingModelHandle".to_string()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_ThingModel_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~ThingModel".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![IrParam {
                        name: "self".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "ThingModel*".to_string(),
                            c_type: "ThingModelHandle*".to_string(),
                            handle: Some("ThingModelHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap_ThingModel_GetValue".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "ThingModel::GetValue".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Primitive,
                        cpp_type: "int".to_string(),
                        c_type: "int".to_string(),
                        handle: None,
                    },
                    params: vec![IrParam {
                        name: "self".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "const ThingModel*".to_string(),
                            c_type: "ThingModelHandle*".to_string(),
                            handle: Some("ThingModelHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap_ThingModel_SetValue".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "ThingModel::SetValue".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: Some(false),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![
                        IrParam {
                            name: "self".to_string(),
                            ty: IrType {
                                kind: IrTypeKind::Opaque,
                                cpp_type: "ThingModel*".to_string(),
                                c_type: "ThingModelHandle*".to_string(),
                                handle: Some("ThingModelHandle".to_string()),
                            },
                        },
                        IrParam {
                            name: "value".to_string(),
                            ty: IrType {
                                kind: IrTypeKind::Primitive,
                                cpp_type: "int".to_string(),
                                c_type: "int".to_string(),
                                handle: None,
                            },
                        },
                    ],
                },
                IrFunction {
                    name: "cgowrap_ThingModel_GetChild".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "ThingModel::GetChild".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: model_type(IrTypeKind::ModelValue, "ThingModel"),
                    params: vec![IrParam {
                        name: "self".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "const ThingModel*".to_string(),
                            c_type: "ThingModelHandle*".to_string(),
                            handle: Some("ThingModelHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap_ThingModel_SetChild".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "ThingModel::SetChild".to_string(),
                    method_of: Some("ThingModelHandle".to_string()),
                    owner_cpp_type: Some("ThingModel".to_string()),
                    is_const: Some(false),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![
                        IrParam {
                            name: "self".to_string(),
                            ty: IrType {
                                kind: IrTypeKind::Opaque,
                                cpp_type: "ThingModel*".to_string(),
                                c_type: "ThingModelHandle*".to_string(),
                                handle: Some("ThingModelHandle".to_string()),
                            },
                        },
                        IrParam {
                            name: "value".to_string(),
                            ty: model_type(IrTypeKind::ModelValue, "ThingModel"),
                        },
                    ],
                },
            ],
        };

        let projections = collect_known_model_projections(&ctx, &ir).unwrap();
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].fields.len(), 2);
        assert_eq!(projections[0].fields[0].go_name, "Value");
        assert_eq!(projections[0].fields[1].go_type, "*ThingModel");
    }

    #[test]
    fn projection_handle_name_comes_from_ir_handles_not_owner_name() {
        let ctx = test_context();
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: crate::ir::SupportMetadata {
                parser_backend: "libclang".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
            functions: vec![
                IrFunction {
                    name: "cgowrap__DCSHISTORY_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "_DCSHISTORY".to_string(),
                    method_of: Some("DCSHISTORYHandle".to_string()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "_DCSHISTORY".to_string(),
                        c_type: "DCSHISTORYHandle*".to_string(),
                        handle: Some("DCSHISTORYHandle".to_string()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap__DCSHISTORY_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~_DCSHISTORY".to_string(),
                    method_of: Some("DCSHISTORYHandle".to_string()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![IrParam {
                        name: "self".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "_DCSHISTORY*".to_string(),
                            c_type: "DCSHISTORYHandle*".to_string(),
                            handle: Some("DCSHISTORYHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap__DCSHISTORY_GetCount".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "_DCSHISTORY::GetCount".to_string(),
                    method_of: Some("DCSHISTORYHandle".to_string()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Primitive,
                        cpp_type: "int".to_string(),
                        c_type: "int".to_string(),
                        handle: None,
                    },
                    params: vec![IrParam {
                        name: "self".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "const _DCSHISTORY*".to_string(),
                            c_type: "const DCSHISTORYHandle*".to_string(),
                            handle: Some("DCSHISTORYHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap__DCSHISTORY_SetCount".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "_DCSHISTORY::SetCount".to_string(),
                    method_of: Some("DCSHISTORYHandle".to_string()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: Some(false),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![
                        IrParam {
                            name: "self".to_string(),
                            ty: IrType {
                                kind: IrTypeKind::Opaque,
                                cpp_type: "_DCSHISTORY*".to_string(),
                                c_type: "DCSHISTORYHandle*".to_string(),
                                handle: Some("DCSHISTORYHandle".to_string()),
                            },
                        },
                        IrParam {
                            name: "value".to_string(),
                            ty: IrType {
                                kind: IrTypeKind::Primitive,
                                cpp_type: "int".to_string(),
                                c_type: "int".to_string(),
                                handle: None,
                            },
                        },
                    ],
                },
            ],
        };

        let projections = collect_known_model_projections(&ctx, &ir).unwrap();
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].handle_name, "DCSHISTORYHandle");
        assert_eq!(projections[0].go_name, "DCSHISTORY");
    }
}
