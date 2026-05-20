use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use crate::{
    domain::kind::{IrFunctionKind, IrTypeKind},
    ir::{IrCallback, IrEnum, IrFunction, IrMacroConstant, IrModule, IrType, OpaqueType},
    parsing::macros::MacroConstantKind,
    pipeline::context::PipelineContext,
};

mod callbacks;
mod calls;
mod classes;
mod support;

use callbacks::{
    collect_callback_usages, render_callback_export, render_callback_registry,
    render_callback_type, used_callbacks,
};
use calls::{
    collect_free_function_dispatchers, collect_method_dispatchers, has_byte_array_params,
    has_pointer_params, has_string_params, has_void_model_params, render_free_function,
    render_free_function_dispatcher, render_method_dispatcher,
};
use classes::{
    collect_facade_classes, collect_owned_opaque_model_value_handles, render_facade_class,
    render_facade_close, render_facade_constructor, render_general_api_method,
    render_handle_helpers, render_owned_opaque_wrapper,
};
pub use support::primitive_go_type_pub;
use support::*;

#[derive(Debug)]
pub struct GeneratedGoFile {
    pub filename: String,
    pub contents: String,
}

#[derive(Debug)]
struct AnalyzedFacadeClass<'a> {
    go_name: String,
    handle_name: String,
    constructors: Vec<&'a IrFunction>,
    destructor: &'a IrFunction,
    methods: Vec<&'a IrFunction>,
}

#[derive(Debug, Default)]
struct RenderedCallPrep {
    setup_lines: Vec<String>,
    defer_lines: Vec<String>,
    post_call_lines: Vec<String>,
    args: Vec<String>,
}

#[derive(Debug)]
struct OverloadDispatcher<'a> {
    export_name: String,
    functions: Vec<&'a IrFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DispatcherKey {
    param_go_types: Vec<String>,
    return_sig: String,
}

#[derive(Debug, Clone)]
struct CallbackUsage<'a> {
    callback: &'a IrCallback,
    function: &'a IrFunction,
    param_index: usize,
}

struct GoFacadeFile<'a, 'ir> {
    config: &'a PipelineContext,
    constants: &'a [&'ir IrMacroConstant],
    enums: &'a [&'ir IrEnum],
    functions: &'a [&'ir IrFunction],
    classes: &'a [AnalyzedFacadeClass<'ir>],
    callback_usages: &'a [CallbackUsage<'ir>],
    opaque_types: &'a [&'ir OpaqueType],
    globally_emitted_opaques: &'a BTreeSet<String>,
    owned_opaque_value_handles: &'a BTreeSet<String>,
    local_owned_opaque_value_handles: &'a BTreeSet<String>,
}

pub fn render_go_facade(
    config: &PipelineContext,
    ir: &IrModule,
    globally_emitted_opaques: &BTreeSet<String>,
) -> Result<Vec<GeneratedGoFile>> {
    render_go_facade_with_owned_opaques(
        config,
        ir,
        globally_emitted_opaques,
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

pub(crate) fn render_go_facade_with_owned_opaques(
    config: &PipelineContext,
    ir: &IrModule,
    globally_emitted_opaques: &BTreeSet<String>,
    global_owned_opaque_value_handles: &BTreeSet<String>,
    local_owned_opaque_value_handles: &BTreeSet<String>,
) -> Result<Vec<GeneratedGoFile>> {
    let functions = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Function)
        .filter(|function| free_function_supported(config, function))
        .collect::<Vec<_>>();
    let constants = ir.constants.iter().collect::<Vec<_>>();
    let enums = ir.enums.iter().collect::<Vec<_>>();
    let classes = collect_facade_classes(config, ir)?;
    let callback_usages = collect_callback_usages(&functions, &classes, ir);
    let owned_opaque_value_handles = if global_owned_opaque_value_handles.is_empty()
        && local_owned_opaque_value_handles.is_empty()
    {
        collect_owned_opaque_model_value_handles(config, &functions, &classes)
    } else {
        global_owned_opaque_value_handles.clone()
    };
    let local_owned_opaque_value_handles = if global_owned_opaque_value_handles.is_empty()
        && local_owned_opaque_value_handles.is_empty()
    {
        owned_opaque_value_handles.clone()
    } else {
        local_owned_opaque_value_handles.clone()
    };

    if functions.is_empty() && classes.is_empty() && enums.is_empty() && constants.is_empty() {
        return Ok(Vec::new());
    }

    ensure_unique_go_exports(&functions)?;

    // Exclude opaque types already declared in another file (primary class handles and
    // any non-class opaque types claimed by a previously-processed header).
    let local_opaque_types: Vec<&OpaqueType> = ir
        .opaque_types
        .iter()
        .filter(|ot| {
            !globally_emitted_opaques.contains(&ot.name)
                || local_owned_opaque_value_handles.contains(&ot.name)
        })
        .collect();

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_facade_file(GoFacadeFile {
            config,
            constants: &constants,
            enums: &enums,
            functions: &functions,
            classes: &classes,
            callback_usages: &callback_usages,
            opaque_types: &local_opaque_types,
            globally_emitted_opaques,
            owned_opaque_value_handles: &owned_opaque_value_handles,
            local_owned_opaque_value_handles: &local_owned_opaque_value_handles,
        }),
    }])
}

fn render_go_facade_file(input: GoFacadeFile<'_, '_>) -> String {
    let GoFacadeFile {
        config,
        constants,
        enums,
        functions,
        classes,
        callback_usages,
        opaque_types,
        globally_emitted_opaques,
        owned_opaque_value_handles,
        local_owned_opaque_value_handles,
    } = input;

    let package_name = go_package_name(&config.output.dir);
    let requires_cgo = !functions.is_empty() || !classes.is_empty();
    let free_function_dispatchers = collect_free_function_dispatchers(config, functions);
    let method_dispatchers = classes
        .iter()
        .map(|class| (class, collect_method_dispatchers(config, class)))
        .collect::<Vec<_>>();
    let requires_fmt = !free_function_dispatchers.is_empty()
        || method_dispatchers
            .iter()
            .any(|(_, dispatchers)| !dispatchers.is_empty());
    let requires_errors = classes.iter().any(|class| !class.constructors.is_empty())
        || functions.iter().any(|function| {
            matches!(
                function.returns.kind,
                IrTypeKind::String
                    | IrTypeKind::CString
                    | IrTypeKind::FixedByteArray
                    | IrTypeKind::FixedArray
                    | IrTypeKind::FixedModelArray
            )
        })
        || classes.iter().any(|class| {
            class.methods.iter().any(|function| {
                matches!(
                    function.returns.kind,
                    IrTypeKind::String
                        | IrTypeKind::CString
                        | IrTypeKind::FixedByteArray
                        | IrTypeKind::FixedArray
                        | IrTypeKind::FixedModelArray
                )
            })
        });
    let requires_unsafe = functions.iter().any(|function| {
        has_string_params(function.params.iter())
            || has_pointer_params(function.params.iter())
            || has_byte_array_params(function.params.iter())
            || has_void_model_params(function.params.iter())
            || matches!(
                function.returns.kind,
                IrTypeKind::Pointer
                    | IrTypeKind::FixedByteArray
                    | IrTypeKind::FixedArray
                    | IrTypeKind::FixedModelArray
            )
    }) || classes.iter().any(|class| {
        class.constructors.iter().any(|ctor| {
            has_string_params(ctor.params.iter())
                || has_pointer_params(ctor.params.iter())
                || has_byte_array_params(ctor.params.iter())
                || has_void_model_params(ctor.params.iter())
        }) || class.methods.iter().any(|function| {
            has_string_params(function.params.iter().skip(1))
                || has_pointer_params(function.params.iter().skip(1))
                || has_byte_array_params(function.params.iter().skip(1))
                || has_void_model_params(function.params.iter().skip(1))
                || matches!(
                    function.returns.kind,
                    IrTypeKind::Pointer | IrTypeKind::FixedByteArray
                )
        })
    });
    let requires_sync = !callback_usages.is_empty();

    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));
    if requires_cgo {
        out.push_str("/*\n");
        out.push_str("#include <stdlib.h>\n");
        if ir_uses_struct_timeval(functions, classes) {
            out.push_str("#include <sys/time.h>\n");
        }
        out.push_str(&format!(
            "#include \"{}\"\n",
            config.generated_header_include(&config.output.header)
        ));
        out.push_str("*/\n");
        out.push_str("import \"C\"\n\n");
    }
    if requires_errors {
        out.push_str("import \"errors\"\n\n");
    }
    if requires_fmt {
        out.push_str("import \"fmt\"\n\n");
    }
    if requires_unsafe {
        out.push_str("import \"unsafe\"\n\n");
    }
    if requires_sync {
        out.push_str("import \"sync\"\n\n");
    }

    if !constants.is_empty() {
        out.push_str(&render_go_constants(constants));
        out.push('\n');
    }
    for item in enums {
        out.push_str(&render_go_enum(item));
        out.push('\n');
    }
    for callback in used_callbacks(callback_usages) {
        out.push_str(&render_callback_type(callback));
        out.push('\n');
    }
    for usage in callback_usages {
        out.push_str(&render_callback_registry(usage));
        out.push('\n');
        out.push_str(&render_callback_export(usage));
        out.push('\n');
    }

    let mut covered_handles: BTreeSet<String> = classes
        .iter()
        .map(|class| class.handle_name.clone())
        .collect();
    covered_handles.extend(
        config
            .known_model_projections
            .iter()
            .map(|projection| projection.handle_name.clone()),
    );

    for function in functions {
        out.push_str(&render_free_function(
            config,
            function,
            &covered_handles,
            owned_opaque_value_handles,
        ));
        out.push('\n');
    }
    for dispatcher in &free_function_dispatchers {
        out.push_str(&render_free_function_dispatcher(config, dispatcher));
        out.push('\n');
    }

    // Also track Go names used by primary class wrappers to catch cases where a typedef
    // and a class produce the same Go name (e.g. _LegId class → "LegId", LegId opaque → "LegId").
    let mut covered_go_names: BTreeSet<String> =
        classes.iter().map(|class| class.go_name.clone()).collect();
    covered_go_names.extend(
        config
            .known_model_projections
            .iter()
            .map(|projection| projection.go_name.clone()),
    );

    for opaque in opaque_types {
        let is_local_owned_opaque = local_owned_opaque_value_handles.contains(&opaque.name);
        if covered_handles.contains(&opaque.name) {
            continue;
        }
        if globally_emitted_opaques.contains(&opaque.name) && !is_local_owned_opaque {
            continue;
        }
        let base = opaque.name.strip_suffix("Handle").unwrap_or(&opaque.name);
        let go_name = go_export_name(base);
        if covered_go_names.contains(&go_name) {
            continue;
        }
        if is_local_owned_opaque {
            out.push_str(&render_owned_opaque_wrapper(&go_name, &opaque.name));
        } else {
            out.push_str(&format!(
                "type {} struct {{\n    ptr *C.{}\n}}\n\n",
                go_name, opaque.name
            ));
        }
    }

    for (class, dispatchers) in method_dispatchers {
        out.push_str(&render_facade_class(class));
        out.push('\n');
        let constructor_names = go_constructor_export_names(class);
        for (constructor, constructor_name) in
            class.constructors.iter().zip(constructor_names.iter())
        {
            out.push_str(&render_facade_constructor(
                config,
                class,
                constructor,
                constructor_name,
            ));
            out.push('\n');
        }
        out.push_str(&render_facade_close(class));
        out.push('\n');
        out.push_str(&render_handle_helpers(class));
        out.push('\n');
        for method in &class.methods {
            out.push_str(&render_general_api_method(
                config,
                class,
                method,
                &covered_handles,
                owned_opaque_value_handles,
            ));
            out.push('\n');
        }
        for dispatcher in &dispatchers {
            out.push_str(&render_method_dispatcher(config, class, dispatcher));
            out.push('\n');
        }
    }

    out
}

fn render_go_constants(constants: &[&IrMacroConstant]) -> String {
    let mut out = String::new();
    out.push_str("const (\n");
    for item in constants {
        let value = match item.kind {
            MacroConstantKind::Integer | MacroConstantKind::Float | MacroConstantKind::String => {
                &item.value
            }
        };
        out.push_str(&format!("    {} = {}\n", item.name, value));
    }
    out.push_str(")\n");
    out
}

fn render_go_enum(item: &IrEnum) -> String {
    let mut out = String::new();
    if item.is_anonymous {
        out.push_str("const (\n");
        for variant in &item.variants {
            let value = variant.value.as_deref().unwrap_or("0");
            out.push_str(&format!("    {} = {}\n", variant.name, value));
        }
        out.push_str(")\n");
    } else {
        let name = leaf_cpp_name(&item.cpp_name);
        out.push_str(&format!("type {} int64\n\n", name));
        out.push_str("const (\n");
        for variant in &item.variants {
            let value = variant.value.as_deref().unwrap_or("0");
            out.push_str(&format!("    {} {} = {}\n", variant.name, name, value));
        }
        out.push_str(")\n");
    }
    out
}

fn ensure_unique_go_exports(functions: &[&IrFunction]) -> Result<()> {
    let mut by_export = BTreeMap::<String, Vec<String>>::new();
    for function in functions {
        by_export
            .entry(go_facade_export_name(function))
            .or_default()
            .push(function.cpp_name.clone());
    }

    let collisions = by_export
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return Ok(());
    }

    let detail = collisions
        .into_iter()
        .map(|(export, names)| {
            format!(
                "Go facade export `{export}` collides for: {}",
                names.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!("facade export collision detected: {detail}");
}

fn ensure_unique_method_exports(owner: &str, methods: &[&IrFunction]) -> Result<()> {
    let mut by_export = BTreeMap::<String, Vec<String>>::new();
    for function in methods {
        by_export
            .entry(go_method_export_name(function))
            .or_default()
            .push(function.cpp_name.clone());
    }

    let collisions = by_export
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return Ok(());
    }

    let detail = collisions
        .into_iter()
        .map(|(export, names)| {
            format!(
                "Go facade method `{owner}.{export}` collides for: {}",
                names.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!("facade export collision detected: {detail}");
}

fn free_function_supported(config: &PipelineContext, function: &IrFunction) -> bool {
    go_return_supported(config, &function.returns)
        && function
            .params
            .iter()
            .all(|param| go_param_supported(config, &param.ty))
}

fn method_supported(config: &PipelineContext, function: &IrFunction) -> bool {
    go_return_supported(config, &function.returns)
        && function
            .params
            .iter()
            .skip(1)
            .all(|param| go_param_supported(config, &param.ty))
}

fn go_param_supported(config: &PipelineContext, ty: &IrType) -> bool {
    go_param_type(config, ty).is_some()
}

fn go_param_type(config: &PipelineContext, ty: &IrType) -> Option<String> {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => Some("string".to_string()),
        IrTypeKind::FixedByteArray => Some("[]byte".to_string()),
        IrTypeKind::FixedArray => Some(format!("[]{}", fixed_array_go_elem_type(ty))),
        IrTypeKind::FixedModelArray => {
            let go_name = go_model_return_type(config, ty);
            Some(format!("[]*{go_name}"))
        }
        IrTypeKind::Primitive | IrTypeKind::Enum => go_value_type(config, ty),
        IrTypeKind::Reference => go_type_for_reference(ty).map(|go_type| format!("*{go_type}")),
        IrTypeKind::Pointer => {
            let base = ty.cpp_type.trim_end_matches('*').trim();
            primitive_go_type(base)
                .or_else(|| primitive_go_type(ty.c_type.trim_end_matches('*').trim()))
                .map(|go_type| format!("*{go_type}"))
        }
        IrTypeKind::ExternStructPointer | IrTypeKind::ExternStructReference => {
            extern_struct_go_type(ty)
        }
        IrTypeKind::Callback => Some(leaf_cpp_name(&ty.cpp_type)),
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
            if base_model_cpp_type(&ty.cpp_type) == "void" {
                return Some("unsafe.Pointer".to_string());
            }
            config
                .known_model_projection(&ty.cpp_type)
                .map(|projection| format!("*{}", projection.go_name))
                .or_else(|| {
                    ty.handle
                        .as_deref()
                        .and_then(|h| h.strip_suffix("Handle"))
                        .map(|base| format!("*{}", go_export_name(base)))
                })
        }
        _ => None,
    }
}

fn go_return_supported(_config: &PipelineContext, ty: &IrType) -> bool {
    ty.kind == IrTypeKind::Void
        || matches!(
            ty.kind,
            IrTypeKind::String
                | IrTypeKind::CString
                | IrTypeKind::FixedByteArray
                | IrTypeKind::FixedArray
                | IrTypeKind::FixedModelArray
                | IrTypeKind::Enum
        )
        || (ty.kind == IrTypeKind::Primitive && go_type_for_ir(ty).is_some())
        || (ty.kind == IrTypeKind::Pointer && go_pointer_return_type(ty).is_some())
        || matches!(
            ty.kind,
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
        )
}

fn go_pointer_return_type(ty: &IrType) -> Option<String> {
    if ty.kind != IrTypeKind::Pointer {
        return None;
    }
    let base = ty.cpp_type.trim_end_matches('*').trim();
    primitive_go_type(base)
        .or_else(|| primitive_go_type(ty.c_type.trim_end_matches('*').trim()))
        .map(|go_type| format!("*{go_type}"))
}

fn go_model_return_type(config: &PipelineContext, ty: &IrType) -> String {
    if base_model_cpp_type(&ty.cpp_type) == "void" {
        return "unsafe.Pointer".to_string();
    }
    config
        .known_model_projection(&ty.cpp_type)
        .map(|projection| projection.go_name.clone())
        .unwrap_or_else(|| {
            ty.handle
                .as_deref()
                .and_then(|h| h.strip_suffix("Handle"))
                .map(go_export_name)
                .unwrap_or_else(|| flatten_qualified_cpp_name(&base_model_cpp_type(&ty.cpp_type)))
        })
}

fn is_model_wrapper_return(ty: &IrType) -> bool {
    matches!(
        ty.kind,
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
    )
}

fn model_return_is_owned(config: &PipelineContext, function: &IrFunction, ty: &IrType) -> bool {
    ty.kind == IrTypeKind::ModelValue
        || (ty.kind == IrTypeKind::ModelPointer && config.owner_marks_callable(&function.cpp_name))
}

fn model_return_uses_inline_owned_literal(
    config: &PipelineContext,
    function: &IrFunction,
    ty: &IrType,
) -> bool {
    ty.kind == IrTypeKind::ModelPointer && config.owner_marks_callable(&function.cpp_name)
}

fn model_return_has_wrapper_helpers(
    config: &PipelineContext,
    ty: &IrType,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> bool {
    config.known_model_projection(&ty.cpp_type).is_some()
        || ty.handle.as_ref().is_some_and(|handle| {
            covered_handles.contains(handle) || owned_opaque_value_handles.contains(handle)
        })
}

/// Returns the Go return type signature string (without surrounding parens for single values).
/// e.g. `""` for void, `"(string, error)"` for string, `"([]*Foo, error)"` for FixedModelArray.
fn go_return_sig(config: &PipelineContext, ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Void => String::new(),
        IrTypeKind::String | IrTypeKind::CString => "(string, error)".to_string(),
        IrTypeKind::FixedByteArray => "([]byte, error)".to_string(),
        IrTypeKind::FixedArray => format!("([]{}, error)", fixed_array_go_elem_type(ty)),
        IrTypeKind::FixedModelArray => {
            let go_name = go_model_return_type(config, ty);
            format!("([]*{go_name}, error)")
        }
        IrTypeKind::Pointer => go_pointer_return_type(ty).unwrap_or_default(),
        _ if is_model_wrapper_return(ty) => {
            let model_ret = go_model_return_type(config, ty);
            if model_ret == "unsafe.Pointer" {
                "unsafe.Pointer".to_string()
            } else {
                format!("*{model_ret}")
            }
        }
        _ => go_value_type(config, ty).unwrap_or_else(|| "int32".to_string()),
    }
}

/// Returns the nil/zero return statement used inside the receiver-nil guard block.
fn go_nil_return_stmt(ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Void => "return".to_string(),
        IrTypeKind::String | IrTypeKind::CString => {
            "return \"\", errors.New(\"facade receiver is nil\")".to_string()
        }
        IrTypeKind::FixedByteArray | IrTypeKind::FixedArray | IrTypeKind::FixedModelArray => {
            "return nil, errors.New(\"facade receiver is nil\")".to_string()
        }
        IrTypeKind::Pointer => "return nil".to_string(),
        _ if is_model_wrapper_return(ty) => "return nil".to_string(),
        _ => format!(
            "return {}",
            zero_value_for_go_type(go_type_for_ir(ty).unwrap_or("int32"))
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        domain::model_projection::{ModelProjection, ModelProjectionField},
        ir::IrParam,
        pipeline::context::PipelineContext,
    };
    use std::collections::BTreeSet;

    fn test_context_with_known_model() -> PipelineContext {
        PipelineContext::new(Config::default()).with_known_model_projections(vec![
            ModelProjection {
                cpp_type: "ThingModel".to_string(),
                handle_name: "ThingModelHandle".to_string(),
                go_name: "ThingModel".to_string(),
                constructor_symbol: "cgowrap_ThingModel_new".to_string(),
                destructor_symbol: "cgowrap_ThingModel_delete".to_string(),
                fields: vec![ModelProjectionField {
                    go_name: "Value".to_string(),
                    go_type: "int".to_string(),
                    getter_symbol: "cgowrap_ThingModel_GetValue".to_string(),
                    setter_symbol: "cgowrap_ThingModel_SetValue".to_string(),
                    return_kind: IrTypeKind::Primitive,
                }],
            },
        ])
    }

    fn primitive_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: IrTypeKind::Primitive,
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    fn model_type(kind: IrTypeKind, cpp_type: &str) -> IrType {
        IrType {
            kind,
            cpp_type: cpp_type.to_string(),
            c_type: format!("{cpp_type}Handle*"),
            handle: Some(format!("{cpp_type}Handle")),
        }
    }

    fn reference_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: IrTypeKind::Reference,
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    #[test]
    fn method_supports_known_model_reference_params() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_type(IrTypeKind::ModelReference, "ThingModel"),
                },
                IrParam {
                    name: "id".to_string(),
                    ty: primitive_type("int", "int"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
    }

    #[test]
    fn method_supports_unknown_model_params_as_handles() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "value".to_string(),
                    ty: model_type(IrTypeKind::ModelReference, "UnknownThing"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
    }

    #[test]
    fn method_supports_primitive_reference_and_known_model_params() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_NextThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::NextThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "pos".to_string(),
                    ty: reference_type("int32&", "int32_t*"),
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_type(IrTypeKind::ModelReference, "ThingModel"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
        assert_eq!(
            go_param_type(&config, &function.params[1].ty),
            Some("*int32".to_string())
        );
    }

    #[test]
    fn overload_tokens_distinguish_model_ref_and_ptr() {
        assert_eq!(
            go_overload_token(&model_type(IrTypeKind::ModelReference, "ThingModel")),
            "ThingModelRef"
        );
        assert_eq!(
            go_overload_token(&model_type(IrTypeKind::ModelPointer, "ThingModel")),
            "ThingModelPtr"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: IrTypeKind::ModelPointer,
                cpp_type: "ThingModel**".to_string(),
                c_type: "ThingModelHandle**".to_string(),
                handle: Some("ThingModelHandle".to_string()),
            }),
            "ThingModelPtrPtr"
        );
    }

    #[test]
    fn overload_tokens_preserve_typedef_identity_for_alias_backed_scalars() {
        assert_eq!(
            go_overload_token(&primitive_type("time_t", "int64_t")),
            "TimeT"
        );
        assert_eq!(
            go_overload_token(&primitive_type("uint32", "uint32_t")),
            "Uint32"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: IrTypeKind::CString,
                cpp_type: "NPCSTR".to_string(),
                c_type: "const char*".to_string(),
                handle: None,
            }),
            "NPCSTR"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: IrTypeKind::String,
                cpp_type: "NPSTR".to_string(),
                c_type: "char*".to_string(),
                handle: None,
            }),
            "NPSTR"
        );
    }

    #[test]
    fn go_export_name_capitalizes_lowercase_first_letter() {
        assert_eq!(go_export_name("myApi"), "MyApi");
        assert_eq!(go_export_name("thingModel"), "ThingModel");
        assert_eq!(go_export_name("iApiClient"), "IApiClient");
        assert_eq!(go_export_name("UserRecord"), "UserRecord");
    }

    #[test]
    fn false_double_underscore_from_owner_name_does_not_trigger_go_overload_suffix() {
        let function = IrFunction {
            name: "cgowrap__SYS_IF_MONITOR_IODSM_SetBModifyFlag".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "_SYS_IF_MONITOR_IODSM::SetBModifyFlag".to_string(),
            method_of: Some("SYS_IF_MONITOR_IODSMHandle".to_string()),
            owner_cpp_type: Some("_SYS_IF_MONITOR_IODSM".to_string()),
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
                        cpp_type: "_SYS_IF_MONITOR_IODSM*".to_string(),
                        c_type: "SYS_IF_MONITOR_IODSMHandle*".to_string(),
                        handle: Some("SYS_IF_MONITOR_IODSMHandle".to_string()),
                    },
                },
                IrParam {
                    name: "value".to_string(),
                    ty: primitive_type("bool", "bool"),
                },
            ],
        };

        assert!(!has_disambiguated_raw_overload_suffix(&function));
        assert_eq!(go_method_export_name(&function), "SetBModifyFlag");
    }

    #[test]
    fn explicit_raw_overload_suffix_still_triggers_go_overload_suffix() {
        let function = IrFunction {
            name: "cgowrap_Api_SetFlag__bool_mut".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::SetFlag".to_string(),
            method_of: Some("ApiHandle".to_string()),
            owner_cpp_type: Some("Api".to_string()),
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
                        cpp_type: "Api*".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "value".to_string(),
                    ty: primitive_type("bool", "bool"),
                },
            ],
        };

        assert!(has_disambiguated_raw_overload_suffix(&function));
        assert_eq!(go_method_export_name(&function), "SetFlagBool");
    }

    #[test]
    fn render_go_facade_uses_capitalized_struct_name_for_lowercase_cpp_class() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let handle_name = "myApiHandle".to_string();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "myApi".to_string(),
                c_type: "myApiHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![OpaqueType {
                name: handle_name.clone(),
                cpp_type: "myApi".to_string(),
            }],
            functions: vec![
                IrFunction {
                    name: "cgowrap_myApi_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "myApi".to_string(),
                    method_of: None,
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "myApi".to_string(),
                        c_type: "myApiHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_myApi_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "myApi".to_string(),
                    method_of: None,
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_myApi_IsReady".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "myApi::IsReady".to_string(),
                    method_of: Some("myApi".to_string()),
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: primitive_type("bool", "bool"),
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let config = PipelineContext::new(Config::default());
        let files = render_go_facade(&config, &ir, &BTreeSet::new()).unwrap();
        assert!(!files.is_empty(), "expected at least one Go file");
        let contents = &files[0].contents;
        assert!(
            contents.contains("type MyApi struct {"),
            "expected 'type MyApi struct {{' but got:\n{contents}"
        );
        assert!(
            contents.contains("func NewMyApi()"),
            "expected 'func NewMyApi()' but got:\n{contents}"
        );
    }

    #[test]
    fn render_go_facade_emits_all_overloaded_constructors_with_explicit_names() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let handle_name = "WidgetHandle".to_string();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Widget*".to_string(),
                c_type: "WidgetHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![OpaqueType {
                name: handle_name.clone(),
                cpp_type: "Widget".to_string(),
            }],
            functions: vec![
                IrFunction {
                    name: "cgowrap_Widget_new__void".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Widget".to_string(),
                        c_type: "WidgetHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_Widget_new__int".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Widget".to_string(),
                        c_type: "WidgetHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![IrParam {
                        name: "nItemMax".to_string(),
                        ty: primitive_type("int", "int"),
                    }],
                },
                IrFunction {
                    name: "cgowrap_Widget_new__model_ref_widget".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Widget".to_string(),
                        c_type: "WidgetHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![IrParam {
                        name: "copy".to_string(),
                        ty: model_type(IrTypeKind::ModelReference, "Widget"),
                    }],
                },
                IrFunction {
                    name: "cgowrap_Widget_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_Widget_GetSize".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "Widget::GetSize".to_string(),
                    method_of: Some(handle_name),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: primitive_type("int", "int"),
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let files = render_go_facade(
            &PipelineContext::new(Config::default()),
            &ir,
            &BTreeSet::new(),
        )
        .unwrap();
        let contents = &files[0].contents;
        assert!(
            contents.contains("func NewWidget() (*Widget, error) {"),
            "expected zero-arg constructor but got:\n{contents}"
        );
        assert!(
            contents.contains("func NewWidgetWithNItemMax(nItemMax int32) (*Widget, error) {"),
            "expected named int constructor but got:\n{contents}"
        );
        assert!(
            contents.contains("func NewWidgetFromCopy(copy *Widget) (*Widget, error) {"),
            "expected copy constructor name but got:\n{contents}"
        );
    }

    #[test]
    fn constructor_names_disambiguate_same_param_name_overloads() {
        let handle_name = "WidgetHandle".to_string();
        let constructor_int = IrFunction {
            name: "cgowrap_Widget_new__int".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Widget".to_string(),
            method_of: Some(handle_name.clone()),
            owner_cpp_type: Some("Widget".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Widget".to_string(),
                c_type: "WidgetHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
            params: vec![IrParam {
                name: "value".to_string(),
                ty: primitive_type("int", "int"),
            }],
        };
        let constructor_double = IrFunction {
            name: "cgowrap_Widget_new__double".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Widget".to_string(),
            method_of: Some(handle_name.clone()),
            owner_cpp_type: Some("Widget".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Widget".to_string(),
                c_type: "WidgetHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
            params: vec![IrParam {
                name: "value".to_string(),
                ty: primitive_type("double", "double"),
            }],
        };
        let destructor = IrFunction {
            name: "cgowrap_Widget_delete".to_string(),
            kind: IrFunctionKind::Destructor,
            cpp_name: "~Widget".to_string(),
            method_of: Some("WidgetHandle".to_string()),
            owner_cpp_type: Some("Widget".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Void,
                cpp_type: "void".to_string(),
                c_type: "void".to_string(),
                handle: None,
            },
            params: vec![],
        };
        let class = AnalyzedFacadeClass {
            go_name: "Widget".to_string(),
            handle_name,
            constructors: vec![&constructor_int, &constructor_double],
            destructor: &destructor,
            methods: vec![],
        };

        let names = go_constructor_export_names(&class);
        assert_eq!(
            names,
            vec!["NewWidgetWithValueInt32", "NewWidgetWithValueFloat64"]
        );
    }

    #[test]
    fn unsupported_constructor_does_not_drop_supported_constructors() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let handle_name = "WidgetHandle".to_string();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Widget*".to_string(),
                c_type: "WidgetHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![OpaqueType {
                name: handle_name.clone(),
                cpp_type: "Widget".to_string(),
            }],
            functions: vec![
                IrFunction {
                    name: "cgowrap_Widget_new__void".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Widget".to_string(),
                        c_type: "WidgetHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_Widget_new__opaque".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Widget".to_string(),
                        c_type: "WidgetHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![IrParam {
                        name: "raw".to_string(),
                        ty: IrType {
                            kind: IrTypeKind::Opaque,
                            cpp_type: "InternalHandle".to_string(),
                            c_type: "InternalHandle*".to_string(),
                            handle: Some("InternalHandle".to_string()),
                        },
                    }],
                },
                IrFunction {
                    name: "cgowrap_Widget_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~Widget".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_Widget_GetSize".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "Widget::GetSize".to_string(),
                    method_of: Some(handle_name),
                    owner_cpp_type: Some("Widget".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: primitive_type("int", "int"),
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let files = render_go_facade(
            &PipelineContext::new(Config::default()),
            &ir,
            &BTreeSet::new(),
        )
        .unwrap();
        let contents = &files[0].contents;
        assert!(
            contents.contains("func NewWidget() (*Widget, error) {"),
            "expected supported constructor to remain but got:\n{contents}"
        );
        assert!(
            !contents.contains("NewWidgetWithRaw"),
            "unexpected unsupported constructor facade in:\n{contents}"
        );
    }

    #[test]
    fn class_wrapper_uses_stable_handle_from_constructor_instead_of_owner_name() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let handle_name = "DCSHISTORYHandle".to_string();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "_DCSHISTORY*".to_string(),
                c_type: "DCSHISTORYHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![OpaqueType {
                name: handle_name.clone(),
                cpp_type: "_DCSHISTORY".to_string(),
            }],
            functions: vec![
                IrFunction {
                    name: "cgowrap__DCSHISTORY_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "_DCSHISTORY".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "_DCSHISTORY".to_string(),
                        c_type: "DCSHISTORYHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap__DCSHISTORY_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~_DCSHISTORY".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap__DCSHISTORY_GetCount".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "_DCSHISTORY::GetCount".to_string(),
                    method_of: Some(handle_name.clone()),
                    owner_cpp_type: Some("_DCSHISTORY".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: primitive_type("int", "int"),
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let files = render_go_facade(
            &PipelineContext::new(Config::default()),
            &ir,
            &BTreeSet::new(),
        )
        .unwrap();
        let contents = &files[0].contents;
        assert!(
            contents.contains(
                "type DCSHISTORY struct {\n    ptr *C.DCSHISTORYHandle\n    owned bool\n    root *bool\n}"
            ),
            "expected stable public handle in class wrapper but got:\n{contents}"
        );
    }

    #[test]
    fn model_value_return_is_supported() {
        let ty = model_type(IrTypeKind::ModelValue, "ThingModel");
        let config = test_context_with_known_model();
        assert!(go_return_supported(&config, &ty));
    }

    #[test]
    fn model_value_return_renders_wrap_pattern() {
        let config = test_context_with_known_model();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let void_type = IrType {
            kind: IrTypeKind::Void,
            cpp_type: "void".to_string(),
            c_type: "void".to_string(),
            handle: None,
        };
        let constructor = IrFunction {
            name: "cgowrap_Api_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
            params: vec![],
        };
        let destructor = IrFunction {
            name: "cgowrap_Api_delete".to_string(),
            kind: IrFunctionKind::Destructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: void_type,
            params: vec![self_param.clone()],
        };
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: model_type(IrTypeKind::ModelValue, "ThingModel"),
            params: vec![self_param],
        };

        assert!(method_supported(&config, &function));

        let class = AnalyzedFacadeClass {
            go_name: "Api".to_string(),
            handle_name: "ApiHandle".to_string(),
            constructors: vec![&constructor],
            destructor: &destructor,
            methods: vec![&function],
        };
        let code = render_general_api_method(
            &config,
            &class,
            &function,
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        assert!(
            code.contains("*ThingModel"),
            "expected return type *ThingModel but got:\n{code}"
        );
        assert!(
            code.contains("return nil"),
            "expected nil check but got:\n{code}"
        );
        assert!(
            code.contains("return newOwnedThingModel(raw)"),
            "expected newOwnedThingModel(raw) but got:\n{code}"
        );
    }

    #[test]
    fn model_pointer_return_renders_borrowed_wrap_pattern() {
        let config = test_context_with_known_model();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let void_type = IrType {
            kind: IrTypeKind::Void,
            cpp_type: "void".to_string(),
            c_type: "void".to_string(),
            handle: None,
        };
        let constructor = IrFunction {
            name: "cgowrap_Api_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
            params: vec![],
        };
        let destructor = IrFunction {
            name: "cgowrap_Api_delete".to_string(),
            kind: IrFunctionKind::Destructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: void_type,
            params: vec![self_param.clone()],
        };
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: model_type(IrTypeKind::ModelPointer, "ThingModel"),
            params: vec![self_param],
        };

        assert!(method_supported(&config, &function));

        let class = AnalyzedFacadeClass {
            go_name: "Api".to_string(),
            handle_name: "ApiHandle".to_string(),
            constructors: vec![&constructor],
            destructor: &destructor,
            methods: vec![&function],
        };
        let code = render_general_api_method(
            &config,
            &class,
            &function,
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        assert!(
            code.contains("return newBorrowedThingModel(raw, a.root)"),
            "expected newBorrowedThingModel(raw, a.root) but got:\n{code}"
        );
    }

    #[test]
    fn const_model_borrow_returns_are_supported_in_go_facade() {
        let config = test_context_with_known_model();
        let ty = IrType {
            kind: IrTypeKind::ModelReference,
            cpp_type: "const ThingModel&".to_string(),
            c_type: "const ThingModelHandle*".to_string(),
            handle: Some("ThingModelHandle".to_string()),
        };
        assert!(go_return_supported(&config, &ty));

        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("ApiHandle".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(true),
            field_accessor: None,
            returns: ty,
            params: vec![IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: "const Api*".to_string(),
                    c_type: "const ApiHandle*".to_string(),
                    handle: Some("ApiHandle".to_string()),
                },
            }],
        };
        assert!(method_supported(&config, &function));
    }

    #[test]
    fn const_model_borrow_return_renders_borrowed_wrap_pattern() {
        let config = test_context_with_known_model();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "const Api*".to_string(),
                c_type: "const ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let void_type = IrType {
            kind: IrTypeKind::Void,
            cpp_type: "void".to_string(),
            c_type: "void".to_string(),
            handle: None,
        };
        let constructor = IrFunction {
            name: "cgowrap_Api_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
            params: vec![],
        };
        let destructor = IrFunction {
            name: "cgowrap_Api_delete".to_string(),
            kind: IrFunctionKind::Destructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: void_type,
            params: vec![IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: "Api".to_string(),
                    c_type: "ApiHandle*".to_string(),
                    handle: Some("ApiHandle".to_string()),
                },
            }],
        };
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(true),
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::ModelReference,
                cpp_type: "const ThingModel&".to_string(),
                c_type: "const ThingModelHandle*".to_string(),
                handle: Some("ThingModelHandle".to_string()),
            },
            params: vec![self_param],
        };

        assert!(method_supported(&config, &function));

        let class = AnalyzedFacadeClass {
            go_name: "Api".to_string(),
            handle_name: "ApiHandle".to_string(),
            constructors: vec![&constructor],
            destructor: &destructor,
            methods: vec![&function],
        };
        let code = render_general_api_method(
            &config,
            &class,
            &function,
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        assert!(
            code.contains("return newBorrowedThingModel(raw, a.root)"),
            "expected newBorrowedThingModel(raw, a.root) but got:\n{code}"
        );
    }

    #[test]
    fn free_function_borrowed_return_inherits_unique_model_param_root() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_GetThingChild".to_string(),
            kind: IrFunctionKind::Function,
            cpp_name: "GetThingChild".to_string(),
            method_of: None,
            owner_cpp_type: None,
            is_const: None,
            field_accessor: None,
            returns: model_type(IrTypeKind::ModelPointer, "ThingModel"),
            params: vec![IrParam {
                name: "parent".to_string(),
                ty: model_type(IrTypeKind::ModelPointer, "ThingModel"),
            }],
        };

        let code = render_free_function(&config, &function, &BTreeSet::new(), &BTreeSet::new());
        assert!(
            code.contains("return newBorrowedThingModel(raw, parent.root)"),
            "expected newBorrowedThingModel(raw, parent.root) but got:\n{code}"
        );
    }

    #[test]
    fn class_helpers_track_owned_and_borrowed_lifetimes() {
        let destructor = IrFunction {
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
            params: vec![],
        };
        let class = AnalyzedFacadeClass {
            go_name: "ThingModel".to_string(),
            handle_name: "ThingModelHandle".to_string(),
            constructors: vec![],
            destructor: &destructor,
            methods: vec![],
        };
        let helpers = render_handle_helpers(&class);
        let close = render_facade_close(&class);
        assert!(helpers.contains("root := new(bool)"));
        assert!(helpers.contains("return &ThingModel{ptr: ptr, owned: true, root: root}"));
        assert!(helpers.contains("return &ThingModel{ptr: ptr, root: root}"));
        assert!(helpers.contains("panic(\"ThingModel handle is closed\")"));
        assert!(close.contains("if !t.owned {"));
        assert!(close.contains("*t.root = true"));
    }

    #[test]
    fn known_model_projection_prevents_duplicate_opaque_and_underscore_handle_casts() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let config = PipelineContext::new(Config::default()).with_known_model_projections(vec![
            ModelProjection {
                cpp_type: "_DCSHISTORY".to_string(),
                handle_name: "DCSHISTORYHandle".to_string(),
                go_name: "DCSHISTORY".to_string(),
                constructor_symbol: "cgowrap__DCSHISTORY_new".to_string(),
                destructor_symbol: "cgowrap__DCSHISTORY_delete".to_string(),
                fields: vec![],
            },
        ]);
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![
                OpaqueType {
                    name: "ApiHandle".to_string(),
                    cpp_type: "Api".to_string(),
                },
                OpaqueType {
                    name: "DCSHISTORYHandle".to_string(),
                    cpp_type: "_DCSHISTORY".to_string(),
                },
            ],
            functions: vec![
                IrFunction {
                    name: "cgowrap_Api_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Api".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api*".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_Api_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~Api".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_Api_GetHistory".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "Api::GetHistory".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::ModelValue,
                        cpp_type: "_DCSHISTORY*".to_string(),
                        c_type: "DCSHISTORYHandle*".to_string(),
                        handle: Some("DCSHISTORYHandle".to_string()),
                    },
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let files = render_go_facade(&config, &ir, &BTreeSet::new()).unwrap();
        let contents = &files[0].contents;
        assert!(
            !contents.contains("type DCSHISTORY struct {"),
            "unexpected duplicate DCSHISTORY wrapper:\n{contents}"
        );
        assert!(
            contents.contains("return newOwnedDCSHISTORY(raw)"),
            "expected stable-handle helper wrap but got:\n{contents}"
        );
        assert!(
            !contents.contains("_DCSHISTORYHandle"),
            "unexpected underscore handle cast in Go facade:\n{contents}"
        );
    }

    #[test]
    fn opaque_model_value_return_emits_unknown_opaque_wrapper() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            records: vec![],
            opaque_types: vec![
                OpaqueType {
                    name: "ApiHandle".to_string(),
                    cpp_type: "Api".to_string(),
                },
                OpaqueType {
                    name: "CIosShmHandle".to_string(),
                    cpp_type: "CIosShm".to_string(),
                },
            ],
            functions: vec![
                IrFunction {
                    name: "cgowrap_Api_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "Api".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api*".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_Api_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "~Api".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_Api_GetIos".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "Api::GetIos".to_string(),
                    method_of: Some("ApiHandle".to_string()),
                    owner_cpp_type: Some("Api".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::ModelValue,
                        cpp_type: "CIosShm*".to_string(),
                        c_type: "CIosShmHandle*".to_string(),
                        handle: Some("CIosShmHandle".to_string()),
                    },
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            constants: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let files = render_go_facade(
            &PipelineContext::new(Config::default()),
            &ir,
            &BTreeSet::new(),
        )
        .unwrap();
        let contents = &files[0].contents;
        assert!(
            contents.contains(
                "type CIosShm struct {\n    ptr *C.CIosShmHandle\n    owned bool\n    root *bool\n}"
            ),
            "expected owned opaque CIosShm wrapper but got:\n{contents}"
        );
        assert!(
            contents.contains("func (c *CIosShm) Close() {"),
            "expected CIosShm Close method but got:\n{contents}"
        );
        assert!(
            contents.contains("func (a *Api) GetIos() *CIosShm"),
            "expected GetIos method signature but got:\n{contents}"
        );
        assert!(
            contents.contains("return newOwnedCIosShm(raw)"),
            "expected owned opaque CIosShm wrap pattern but got:\n{contents}"
        );
    }

    #[test]
    fn model_value_return_uses_leaf_name_for_unknown_model() {
        let config = test_context_with_known_model();
        let ty = model_type(IrTypeKind::ModelValue, "UnknownClass");
        let go_name = go_model_return_type(&config, &ty);
        assert_eq!(go_name, "UnknownClass");
    }
}
