use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::{
    codegen::ir_norm,
    domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind},
    ir::{IrFunction, IrModule},
    pipeline::context::PipelineContext,
};

use super::{
    AnalyzedFacadeClass,
    calls::{
        cast_raw_to_projection_handle, has_callback_param, indented_lines, render_call_prep,
        render_callback_method, render_go_call_return, render_model_handle_setup,
        render_param_list,
    },
    ensure_unique_method_exports, go_model_return_type, go_nil_return_stmt, go_param_supported,
    go_param_type, go_return_sig, method_supported,
    support::*,
};

pub(super) fn collect_facade_classes<'a>(
    config: &PipelineContext,
    ir: &'a IrModule,
) -> Result<Vec<AnalyzedFacadeClass<'a>>> {
    let mut methods_by_owner = BTreeMap::<&str, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Method)
    {
        let Some(owner) = function.owner_cpp_type.as_deref() else {
            continue;
        };
        if method_supported(config, function) {
            methods_by_owner.entry(owner).or_default().push(function);
        }
    }

    let mut constructors_by_owner = BTreeMap::<&str, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Constructor)
    {
        let Some(owner) = function.owner_cpp_type.as_deref() else {
            continue;
        };
        constructors_by_owner
            .entry(owner)
            .or_default()
            .push(function);
    }
    let destructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Destructor)
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner, function))
        })
        .collect::<BTreeMap<_, _>>();

    let mut classes = Vec::new();
    for (owner, methods) in methods_by_owner {
        ensure_unique_method_exports(owner, &methods)?;

        let Some(destructor) = destructors.get(owner).copied() else {
            continue;
        };
        let constructors = constructors_by_owner
            .get(owner)
            .into_iter()
            .flat_map(|constructors| constructors.iter().copied())
            .filter(|ctor| {
                ctor.params
                    .iter()
                    .all(|param| go_param_supported(config, &param.ty))
            })
            .collect::<Vec<_>>();
        let handle_name = constructors
            .first()
            .and_then(|ctor| ctor.returns.handle.clone())
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

        classes.push(AnalyzedFacadeClass {
            go_name,
            handle_name,
            constructors,
            destructor,
            methods,
        });
    }

    Ok(classes)
}

pub(super) fn collect_owned_opaque_model_value_handles(
    config: &PipelineContext,
    functions: &[&IrFunction],
    classes: &[AnalyzedFacadeClass<'_>],
) -> BTreeSet<String> {
    let mut covered_handles = classes
        .iter()
        .map(|class| class.handle_name.clone())
        .collect::<BTreeSet<_>>();
    covered_handles.extend(
        config
            .known_model_projections
            .iter()
            .map(|projection| projection.handle_name.clone()),
    );

    functions
        .iter()
        .copied()
        .chain(
            classes
                .iter()
                .flat_map(|class| class.methods.iter().copied()),
        )
        .filter(|function| function.returns.kind == IrTypeKind::ModelValue)
        .filter_map(|function| function.returns.handle.clone())
        .filter(|handle| !covered_handles.contains(handle))
        .collect()
}

pub(super) fn render_facade_class(class: &AnalyzedFacadeClass<'_>) -> String {
    format!(
        "type {} struct {{\n    ptr *C.{}\n    owned bool\n    root *bool\n}}\n",
        class.go_name, class.handle_name
    )
}

pub(super) fn render_owned_opaque_wrapper(go_name: &str, handle: &str) -> String {
    let receiver = receiver_name(go_name);
    let delete_symbol = opaque_delete_symbol(handle);
    format!(
        "type {go_name} struct {{\n    ptr *C.{handle}\n    owned bool\n    root *bool\n}}\n\n\
         func ({receiver} *{go_name}) Close() {{\n\
         \x20   if {receiver} == nil || {receiver}.ptr == nil {{\n\
         \x20       return\n\
         \x20   }}\n\
         \x20   if !{receiver}.owned {{\n\
         \x20       return\n\
         \x20   }}\n\
         \x20   if {receiver}.root != nil {{\n\
         \x20       *{receiver}.root = true\n\
         \x20   }}\n\
         \x20   C.{delete_symbol}({receiver}.ptr)\n\
         \x20   {receiver}.ptr = nil\n\
         }}\n\n\
         func newOwned{go_name}(ptr *C.{handle}) *{go_name} {{\n\
         \x20   if ptr == nil {{\n\
         \x20       return nil\n\
         \x20   }}\n\
         \x20   root := new(bool)\n\
         \x20   return &{go_name}{{ptr: ptr, owned: true, root: root}}\n\
         }}\n\n\
         func newBorrowed{go_name}(ptr *C.{handle}, root *bool) *{go_name} {{\n\
         \x20   if ptr == nil {{\n\
         \x20       return nil\n\
         \x20   }}\n\
         \x20   return &{go_name}{{ptr: ptr, root: root}}\n\
         }}\n\n"
    )
}

pub(super) fn render_facade_constructor(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    constructor: &IrFunction,
    constructor_name: &str,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let constructor_params = constructor.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &constructor_params);
    let prep = render_call_prep(
        config,
        &constructor_params,
        covered_handles,
        owned_opaque_value_handles,
    );

    let mut out = format!(
        "func {constructor_name}({params}) (*{}, error) {{\n",
        class.go_name
    );
    for line in prep.setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in prep.defer_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    ptr := C.{}({})\n",
        constructor.name,
        prep.args.join(", "),
    ));
    for line in prep.post_call_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    if ptr == nil {{\n        return nil, errors.New(\"wrapper returned nil facade handle\")\n    }}\n    return newOwned{}(ptr), nil\n}}\n",
        class.go_name
    ));
    out
}

pub(super) fn render_facade_close(class: &AnalyzedFacadeClass<'_>) -> String {
    let receiver = receiver_name(&class.go_name);
    format!(
        "func ({} *{}) Close() {{\n    if {} == nil || {}.ptr == nil {{\n        return\n    }}\n    if !{}.owned {{\n        return\n    }}\n    if {}.root != nil {{\n        *{}.root = true\n    }}\n    C.{}({}.ptr)\n    {}.ptr = nil\n}}\n",
        receiver,
        class.go_name,
        receiver,
        receiver,
        receiver,
        receiver,
        receiver,
        class.destructor.name,
        receiver,
        receiver,
    )
}

pub(super) fn render_handle_helpers(class: &AnalyzedFacadeClass<'_>) -> String {
    let go_name = &class.go_name;
    let handle = &class.handle_name;
    format!(
        "func newOwned{go_name}(ptr *C.{handle}) *{go_name} {{\n\
         \x20   if ptr == nil {{\n\
         \x20       return nil\n\
         \x20   }}\n\
         \x20   root := new(bool)\n\
         \x20   return &{go_name}{{ptr: ptr, owned: true, root: root}}\n\
         }}\n\
         \n\
         func newBorrowed{go_name}(ptr *C.{handle}, root *bool) *{go_name} {{\n\
         \x20   if ptr == nil {{\n\
         \x20       return nil\n\
         \x20   }}\n\
         \x20   return &{go_name}{{ptr: ptr, root: root}}\n\
         }}\n"
    )
}

pub(super) fn render_general_api_method(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    if let Some(rendered) = render_special_field_method(
        config,
        class,
        function,
        covered_handles,
        owned_opaque_value_handles,
    ) {
        return rendered;
    }
    if has_callback_param(function.params.iter().skip(1)) {
        return render_callback_method(
            config,
            class,
            function,
            covered_handles,
            owned_opaque_value_handles,
        );
    }
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = render_param_list(config, &method_params);
    let prep = render_call_prep(
        config,
        &method_params,
        covered_handles,
        owned_opaque_value_handles,
    );
    let call = format!(
        "C.{}({})",
        function.name,
        std::iter::once(format!("{receiver}.ptr"))
            .chain(prep.args)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!(
        "func ({receiver} *{}) {}({}){sig_part} {{\n",
        class.go_name,
        go_method_export_name(function),
        params
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        {}\n    }}\n",
        go_nil_return_stmt(&function.returns)
    ));
    out.push_str(&format!(
        "    if {receiver}.root != nil && *{receiver}.root {{\n        panic(\"{} handle is closed\")\n    }}\n",
        class.go_name
    ));
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
        Some(format!("{receiver}.root")),
        covered_handles,
        owned_opaque_value_handles,
    ));
    out.push_str("}\n");
    out
}

fn render_special_field_method(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> Option<String> {
    let accessor = function.field_accessor.as_ref()?;
    if accessor.access == FieldAccessKind::Get
        && function.returns.kind == IrTypeKind::FixedModelArray
    {
        return Some(render_fixed_model_array_getter_wrapper(
            config, class, function,
        ));
    }
    if accessor.access == FieldAccessKind::Set
        && function
            .params
            .get(1)
            .is_some_and(|param| param.ty.kind == IrTypeKind::FixedModelArray)
    {
        return Some(render_fixed_model_array_setter_wrapper(class, function));
    }
    if accessor.access == FieldAccessKind::GetAt {
        return Some(render_fixed_model_array_getter_at(config, class, function));
    }
    if accessor.access == FieldAccessKind::SetAt {
        return Some(render_fixed_model_array_setter_at(
            config,
            class,
            function,
            covered_handles,
            owned_opaque_value_handles,
        ));
    }
    None
}

fn render_fixed_model_array_getter_wrapper(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let go_name = go_model_return_type(config, &function.returns);
    let n = ir_norm::fixed_array_length(&function.returns.cpp_type).unwrap_or(0);
    let at_method = go_export_name(&format!("{}At", method_name(function)));
    let mut out = format!(
        "func ({receiver} *{}) {}() ([]*{go_name}, error) {{\n",
        class.go_name,
        go_method_export_name(function)
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        return nil, errors.New(\"facade receiver is nil\")\n    }}\n"
    ));
    out.push_str(&format!(
        "    if {receiver}.root != nil && *{receiver}.root {{\n        panic(\"{} handle is closed\")\n    }}\n",
        class.go_name
    ));
    out.push_str(&format!(
        "    result := make([]*{go_name}, {n})\n    for i := range result {{\n        result[i] = {receiver}.{at_method}(i)\n        if result[i] == nil {{\n            return nil, errors.New(\"wrapper returned nil model array element\")\n        }}\n    }}\n    return result, nil\n"
    ));
    out.push_str("}\n");
    out
}

fn render_fixed_model_array_setter_wrapper(
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let params = render_param_list_dummy(function);
    let n = function
        .params
        .get(1)
        .and_then(|param| ir_norm::fixed_array_length(&param.ty.cpp_type))
        .unwrap_or(0);
    let at_method = go_export_name(&format!("{}At", method_name(function)));
    let value_name = function
        .params
        .get(1)
        .map(|param| param.name.as_str())
        .unwrap_or("value");
    let mut out = format!(
        "func ({receiver} *{}) {}({params}) {{\n",
        class.go_name,
        go_method_export_name(function)
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        return\n    }}\n"
    ));
    out.push_str(&format!(
        "    if {receiver}.root != nil && *{receiver}.root {{\n        panic(\"{} handle is closed\")\n    }}\n",
        class.go_name
    ));
    out.push_str(&format!(
        "    if len({value_name}) != {n} {{\n        panic(\"{} {} requires {n} elements\")\n    }}\n",
        class.go_name,
        go_method_export_name(function)
    ));
    out.push_str(&format!(
        "    for i := range {value_name} {{\n        {receiver}.{at_method}(i, {value_name}[i])\n    }}\n"
    ));
    out.push_str("}\n");
    out
}

fn render_fixed_model_array_getter_at(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let go_name = go_model_return_type(config, &function.returns);
    let ptr_expr = cast_raw_to_projection_handle(config, &function.returns, "raw");
    let wrap = if config
        .known_model_projection(&function.returns.cpp_type)
        .is_some()
    {
        format!("newBorrowed{go_name}({ptr_expr}, {receiver}.root)")
    } else {
        format!("&{go_name}{{ptr: {ptr_expr}}}")
    };
    let mut out = format!(
        "func ({receiver} *{}) {}(index int) *{go_name} {{\n",
        class.go_name,
        go_method_export_name(function)
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        return nil\n    }}\n"
    ));
    out.push_str(&format!(
        "    if {receiver}.root != nil && *{receiver}.root {{\n        panic(\"{} handle is closed\")\n    }}\n",
        class.go_name
    ));
    out.push_str(&format!(
        "    raw := C.{}({receiver}.ptr, C.int(index))\n    if raw == nil {{\n        return nil\n    }}\n    return {wrap}\n",
        function.name
    ));
    out.push_str("}\n");
    out
}

fn render_fixed_model_array_setter_at(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let value_param = function.params.get(2).expect("indexed setter has value");
    let go_name =
        go_param_type(config, &value_param.ty).unwrap_or_else(|| "*unsafe.Pointer".to_string());
    let mut out = format!(
        "func ({receiver} *{}) {}(index int, {} {}) {{\n",
        class.go_name,
        go_method_export_name(function),
        value_param.name,
        go_name
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        return\n    }}\n"
    ));
    out.push_str(&format!(
        "    if {receiver}.root != nil && *{receiver}.root {{\n        panic(\"{} handle is closed\")\n    }}\n",
        class.go_name
    ));
    for line in render_model_handle_setup(
        config,
        &value_param.ty,
        &value_param.name,
        "cArg1",
        covered_handles,
        owned_opaque_value_handles,
    ) {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    C.{}({receiver}.ptr, C.int(index), cArg1)\n",
        function.name
    ));
    out.push_str("}\n");
    out
}

fn render_param_list_dummy(function: &IrFunction) -> String {
    function
        .params
        .iter()
        .skip(1)
        .map(|param| {
            let go_ty = match param.ty.kind {
                IrTypeKind::FixedModelArray => {
                    let go_name = param
                        .ty
                        .handle
                        .as_deref()
                        .and_then(|h| h.strip_suffix("Handle"))
                        .map(go_export_name)
                        .unwrap_or_else(|| "unsafe.Pointer".to_string());
                    format!("[]*{go_name}")
                }
                _ => go_type_for_ir(&param.ty).unwrap_or("int32").to_string(),
            };
            format!("{} {}", param.name, go_ty)
        })
        .collect::<Vec<_>>()
        .join(", ")
}
