use std::collections::{BTreeMap, BTreeSet};

use crate::{
    codegen::ir_norm,
    domain::kind::IrTypeKind,
    ir::{IrFunction, IrType},
    pipeline::context::PipelineContext,
};

use super::{
    AnalyzedFacadeClass, DispatcherKey, OverloadDispatcher, RenderedCallPrep, go_model_return_type,
    go_nil_return_stmt, go_param_type, go_pointer_return_type, go_return_sig,
    is_model_wrapper_return, model_return_has_wrapper_helpers, model_return_is_owned,
    model_return_uses_inline_owned_literal, support::*,
};

#[derive(Clone, Copy)]
enum ReturnMode {
    Direct,
    Dispatcher,
}

struct ReturnHandleContext<'a> {
    covered_handles: &'a BTreeSet<String>,
    owned_opaque_value_handles: &'a BTreeSet<String>,
}

pub(super) fn render_free_function(
    config: &PipelineContext,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    if has_callback_param(function.params.iter()) {
        return render_callback_free_function(
            config,
            function,
            covered_handles,
            owned_opaque_value_handles,
        );
    }
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_call_prep(
        config,
        &params_list,
        covered_handles,
        owned_opaque_value_handles,
    );
    let call = format!("C.{}({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);
    let borrow_root = infer_borrow_root_expr(
        config,
        &params_list,
        covered_handles,
        owned_opaque_value_handles,
    );

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!("func {go_name}({params}){sig_part} {{\n");
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
        borrow_root,
        covered_handles,
        owned_opaque_value_handles,
    ));
    out.push_str("}\n");
    out
}

pub(super) fn covered_dispatcher_function_names(
    dispatchers: &[OverloadDispatcher<'_>],
) -> BTreeSet<String> {
    dispatchers
        .iter()
        .flat_map(|dispatcher| dispatcher.functions.iter())
        .map(|function| function.name.clone())
        .collect()
}

pub(super) fn collect_free_function_dispatchers<'a>(
    config: &PipelineContext,
    functions: &[&'a IrFunction],
) -> Vec<OverloadDispatcher<'a>> {
    let mut by_export = BTreeMap::<String, Vec<&'a IrFunction>>::new();
    for function in functions
        .iter()
        .copied()
        .filter(|function| has_disambiguated_raw_overload_suffix(function))
    {
        by_export
            .entry(go_facade_dispatcher_export_name(function))
            .or_default()
            .push(function);
    }

    by_export
        .into_iter()
        .filter_map(|(export_name, mut group)| {
            build_dispatcher(config, export_name, &mut group, go_facade_export_name)
        })
        .collect()
}

pub(super) fn collect_method_dispatchers<'a>(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'a>,
) -> Vec<OverloadDispatcher<'a>> {
    let mut by_export = BTreeMap::<String, Vec<&'a IrFunction>>::new();
    for function in class
        .methods
        .iter()
        .copied()
        .filter(|function| has_disambiguated_raw_overload_suffix(function))
    {
        by_export
            .entry(go_method_dispatcher_export_name(function))
            .or_default()
            .push(function);
    }

    by_export
        .into_iter()
        .filter_map(|(export_name, mut group)| {
            build_dispatcher(config, export_name, &mut group, go_method_export_name)
        })
        .collect()
}

fn build_dispatcher<'a>(
    config: &PipelineContext,
    export_name: String,
    group: &mut Vec<&'a IrFunction>,
    typed_export_name: fn(&IrFunction) -> String,
) -> Option<OverloadDispatcher<'a>> {
    if group.len() < 2 {
        return None;
    }
    if !dispatcher_group_is_safe(config, group) {
        return None;
    }
    group.sort_by_key(|function| typed_export_name(function));
    Some(OverloadDispatcher {
        export_name,
        functions: group.clone(),
    })
}

fn dispatcher_group_is_safe(config: &PipelineContext, functions: &[&IrFunction]) -> bool {
    let mut seen = BTreeSet::<DispatcherKey>::new();
    let mut return_sig: Option<String> = None;
    for function in functions {
        let Some(key) = dispatcher_key(config, function) else {
            return false;
        };
        if dispatcher_return_sig(&key.return_sig).is_none() {
            return false;
        }
        if return_sig
            .as_deref()
            .is_some_and(|sig| sig != key.return_sig)
        {
            return false;
        }
        return_sig.get_or_insert_with(|| key.return_sig.clone());
        if !seen.insert(key) {
            return false;
        }
    }
    true
}

fn dispatcher_key(config: &PipelineContext, function: &IrFunction) -> Option<DispatcherKey> {
    let params = dispatcher_params(function);
    let param_go_types = params
        .iter()
        .map(|param| go_param_type(config, &param.ty))
        .collect::<Option<Vec<_>>>()?;
    Some(DispatcherKey {
        param_go_types,
        return_sig: go_return_sig(config, &function.returns),
    })
}

fn dispatcher_params(function: &IrFunction) -> Vec<&ir_norm::IrParam> {
    if function.method_of.is_some() {
        function.params.iter().skip(1).collect()
    } else {
        function.params.iter().collect()
    }
}

fn dispatcher_return_sig(direct_return_sig: &str) -> Option<String> {
    if direct_return_sig.is_empty() {
        Some("error".to_string())
    } else if direct_return_sig.contains("error") {
        Some(direct_return_sig.to_string())
    } else if direct_return_sig.starts_with('(') {
        None
    } else {
        Some(format!("({direct_return_sig}, error)"))
    }
}

fn dispatcher_zero_return(config: &PipelineContext, function: &IrFunction) -> Option<String> {
    match function.returns.kind {
        IrTypeKind::Void => return Some(String::new()),
        IrTypeKind::String | IrTypeKind::CString => return Some("\"\", ".to_string()),
        IrTypeKind::FixedByteArray | IrTypeKind::FixedArray | IrTypeKind::FixedModelArray => {
            return Some("nil, ".to_string());
        }
        _ => {}
    }
    let sig = go_return_sig(config, &function.returns);
    if sig.starts_with('*') || sig == "unsafe.Pointer" {
        Some("nil, ".to_string())
    } else {
        go_value_type(config, &function.returns)
            .map(|go_type| format!("{}, ", zero_value_for_go_type(&go_type)))
    }
}

pub(super) fn render_free_function_dispatcher(
    config: &PipelineContext,
    dispatcher: &OverloadDispatcher<'_>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    render_dispatcher(
        config,
        dispatcher,
        None,
        covered_handles,
        owned_opaque_value_handles,
    )
}

pub(super) fn render_method_dispatcher(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    dispatcher: &OverloadDispatcher<'_>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let receiver = receiver_name(&class.go_name);
    render_dispatcher(
        config,
        dispatcher,
        Some((receiver.as_str(), class.go_name.as_str())),
        covered_handles,
        owned_opaque_value_handles,
    )
}

fn render_dispatcher(
    config: &PipelineContext,
    dispatcher: &OverloadDispatcher<'_>,
    receiver: Option<(&str, &str)>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let first = dispatcher.functions[0];
    let direct_sig = go_return_sig(config, &first.returns);
    let dispatcher_sig = dispatcher_return_sig(&direct_sig).unwrap();
    let display_name = receiver
        .map(|(_, class_name)| format!("{class_name}.{}", dispatcher.export_name))
        .unwrap_or_else(|| dispatcher.export_name.clone());
    let sig_part = if dispatcher_sig.is_empty() {
        String::new()
    } else {
        format!(" {dispatcher_sig}")
    };
    let mut out = if let Some((receiver_name, class_name)) = receiver {
        format!(
            "func ({receiver_name} *{class_name}) {}(args ...any){sig_part} {{\n",
            dispatcher.export_name
        )
    } else {
        format!(
            "func {}(args ...any){sig_part} {{\n",
            dispatcher.export_name
        )
    };

    if let Some((receiver_name, class_name)) = receiver {
        let error_return =
            dispatcher_error_return(config, first, &format!("{class_name} receiver is nil"));
        out.push_str(&format!(
            "    if {receiver_name} == nil || {receiver_name}.ptr == nil {{\n        {error_return}\n    }}\n"
        ));
        out.push_str(&format!(
            "    if {receiver_name}.root != nil && *{receiver_name}.root {{\n        panic(\"{class_name} handle is closed\")\n    }}\n"
        ));
    }

    let mut by_arity = BTreeMap::<usize, Vec<&IrFunction>>::new();
    for function in &dispatcher.functions {
        by_arity
            .entry(dispatcher_params(function).len())
            .or_default()
            .push(*function);
    }

    out.push_str("    switch len(args) {\n");
    for (arity, functions) in by_arity {
        out.push_str(&format!("    case {arity}:\n"));
        for function in functions {
            out.push_str(&render_dispatcher_candidate(
                config,
                function,
                receiver,
                covered_handles,
                owned_opaque_value_handles,
            ));
        }
    }
    out.push_str("    }\n");
    let error_return = dispatcher_error_return(
        config,
        first,
        &format!("no matching overload for {display_name}"),
    );
    out.push_str(&format!("    {error_return}\n"));
    out.push_str("}\n");
    out
}

fn render_dispatcher_candidate(
    config: &PipelineContext,
    function: &IrFunction,
    receiver: Option<(&str, &str)>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let params = dispatcher_params(function);
    let mut out = String::new();
    out.push_str("        {\n");
    let mut ok_names = Vec::new();
    for (index, param) in params.iter().enumerate() {
        let arg_name = format!("arg{index}");
        let ok_name = format!("ok{index}");
        let go_type = go_param_type(config, &param.ty).unwrap();
        out.push_str(&format!(
            "            {arg_name}, {ok_name} := args[{index}].({go_type})\n"
        ));
        ok_names.push(ok_name);
    }
    let condition = if ok_names.is_empty() {
        "true".to_string()
    } else {
        ok_names.join(" && ")
    };
    out.push_str(&format!("            if {condition} {{\n"));
    out.push_str(&indent_lines(
        &render_dispatcher_inline_return(
            config,
            function,
            receiver,
            covered_handles,
            owned_opaque_value_handles,
        ),
        12,
    ));
    out.push_str("            }\n");
    out.push_str("        }\n");
    out
}

fn render_dispatcher_inline_return(
    config: &PipelineContext,
    function: &IrFunction,
    receiver: Option<(&str, &str)>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let params = dispatcher_params(function)
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            let mut rebound = param.clone();
            rebound.name = format!("arg{index}");
            rebound
        })
        .collect::<Vec<_>>();
    let param_refs = params.iter().collect::<Vec<_>>();
    let has_callback = has_callback_param(param_refs.iter().copied());
    let prep = if has_callback {
        let param_offset = if receiver.is_some() { 1 } else { 0 };
        render_callback_call_prep(
            config,
            function,
            &param_refs,
            param_offset,
            covered_handles,
            owned_opaque_value_handles,
        )
    } else {
        render_call_prep(
            config,
            &param_refs,
            covered_handles,
            owned_opaque_value_handles,
        )
    };
    let call_name = if has_callback {
        format!("C.{}_bridge", function.name)
    } else {
        format!("C.{}", function.name)
    };
    let call_args = if let Some((receiver_name, _)) = receiver {
        std::iter::once(format!("{receiver_name}.ptr"))
            .chain(prep.args)
            .collect::<Vec<_>>()
    } else {
        prep.args
    };
    let call = format!("{call_name}({})", call_args.join(", "));
    let borrow_root = receiver
        .map(|(receiver_name, _)| format!("{receiver_name}.root"))
        .or_else(|| {
            infer_borrow_root_expr(
                config,
                &param_refs,
                covered_handles,
                owned_opaque_value_handles,
            )
        });

    let mut out = String::new();
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return_with_mode(
        config,
        function,
        &call,
        &prep.post_call_lines,
        borrow_root,
        ReturnHandleContext {
            covered_handles,
            owned_opaque_value_handles,
        },
        ReturnMode::Dispatcher,
    ));
    out
}

fn dispatcher_error_return(
    config: &PipelineContext,
    function: &IrFunction,
    message: &str,
) -> String {
    let zero = dispatcher_zero_return(config, function).unwrap_or_default();
    format!("return {zero}fmt.Errorf(\"{message}\")")
}

pub(super) fn render_callback_method(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = render_param_list(config, &method_params);
    let prep = render_callback_call_prep(
        config,
        function,
        &method_params,
        1,
        covered_handles,
        owned_opaque_value_handles,
    );
    let call = format!(
        "C.{}_bridge({})",
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

fn render_callback_free_function(
    config: &PipelineContext,
    function: &IrFunction,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_callback_call_prep(
        config,
        function,
        &params_list,
        0,
        covered_handles,
        owned_opaque_value_handles,
    );
    let call = format!("C.{}_bridge({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);
    let borrow_root = infer_borrow_root_expr(
        config,
        &params_list,
        covered_handles,
        owned_opaque_value_handles,
    );

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!("func {go_name}({params}){sig_part} {{\n");
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
        borrow_root,
        covered_handles,
        owned_opaque_value_handles,
    ));
    out.push_str("}\n");
    out
}

fn render_callback_call_prep(
    config: &PipelineContext,
    function: &IrFunction,
    params: &[&ir_norm::IrParam],
    param_offset: usize,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        if param.ty.kind == IrTypeKind::Callback {
            let state = callback_state_name_from_function(function, index + param_offset);
            prep.setup_lines.push(format!("{state}.mu.Lock()"));
            prep.setup_lines
                .push(format!("{state}.fn = {}", param.name));
            prep.setup_lines.push(format!("{state}.mu.Unlock()"));
            prep.args.push(format!("C.bool({} != nil)", param.name));
            continue;
        }

        match param.ty.kind {
            IrTypeKind::String | IrTypeKind::CString => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedByteArray => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .extend(render_fixed_length_guard(&param.name, &param.ty));
                prep.setup_lines.push(format!(
                    "{c_name} := (*C.uint8_t)(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedArray => {
                let c_name = format!("cArg{index}");
                let c_elem = fixed_array_cgo_elem_type(&param.ty);
                prep.setup_lines
                    .extend(render_fixed_length_guard(&param.name, &param.ty));
                prep.setup_lines.push(format!(
                    "{c_name} := (*{c_elem})(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedModelArray => {
                render_fixed_model_array_arg(
                    config,
                    &mut prep,
                    &param.ty,
                    &param.name,
                    index,
                    covered_handles,
                    owned_opaque_value_handles,
                );
            }
            IrTypeKind::Reference => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::Pointer => render_pointer_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::ExternStructReference => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, true)
            }
            IrTypeKind::ExternStructPointer => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, false)
            }
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
                render_model_arg(
                    config,
                    &mut prep,
                    &param.ty,
                    &param.name,
                    index,
                    covered_handles,
                    owned_opaque_value_handles,
                )
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

pub(super) fn render_param_list(config: &PipelineContext, params: &[&ir_norm::IrParam]) -> String {
    params
        .iter()
        .map(|param| {
            format!(
                "{} {}",
                param.name,
                go_param_type(config, &param.ty).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn render_call_prep(
    config: &PipelineContext,
    params: &[&ir_norm::IrParam],
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        match param.ty.kind {
            IrTypeKind::String | IrTypeKind::CString => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedByteArray => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .extend(render_fixed_length_guard(&param.name, &param.ty));
                prep.setup_lines.push(format!(
                    "{c_name} := (*C.uint8_t)(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedArray => {
                let c_name = format!("cArg{index}");
                let c_elem = fixed_array_cgo_elem_type(&param.ty);
                prep.setup_lines
                    .extend(render_fixed_length_guard(&param.name, &param.ty));
                prep.setup_lines.push(format!(
                    "{c_name} := (*{c_elem})(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedModelArray => {
                render_fixed_model_array_arg(
                    config,
                    &mut prep,
                    &param.ty,
                    &param.name,
                    index,
                    covered_handles,
                    owned_opaque_value_handles,
                );
            }
            IrTypeKind::Reference => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::Pointer => render_pointer_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::ExternStructReference => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, true)
            }
            IrTypeKind::ExternStructPointer => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, false)
            }
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
                render_model_arg(
                    config,
                    &mut prep,
                    &param.ty,
                    &param.name,
                    index,
                    covered_handles,
                    owned_opaque_value_handles,
                )
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

fn render_fixed_length_guard(name: &str, ty: &IrType) -> Vec<String> {
    let Some(n) = ir_norm::fixed_array_length(&ty.cpp_type) else {
        return Vec::new();
    };
    vec![
        format!("if len({name}) != {n} {{"),
        format!("    panic(\"{name} requires {n} elements\")"),
        "}".to_string(),
    ]
}

/// Returns an expression for `raw` cast to the projection's handle type,
/// inserting an unsafe.Pointer cast when the C return type's handle differs
/// from the projection's stored handle type.
pub(super) fn cast_raw_to_projection_handle(
    config: &PipelineContext,
    returns: &IrType,
    raw_expr: &str,
) -> String {
    if let Some(projection) = config.known_model_projection(&returns.cpp_type)
        && let Some(expected_handle) = &returns.handle
        && *expected_handle != projection.handle_name
    {
        return format!(
            "(*C.{})unsafe.Pointer({}))",
            projection.handle_name, raw_expr
        );
    }
    raw_expr.to_string()
}

fn render_pointer_arg(prep: &mut RenderedCallPrep, ty: &IrType, name: &str, index: usize) {
    let c_name = format!("cArg{index}");
    let base_cpp = ty.cpp_type.trim_end_matches('*').trim();
    let c_type = primitive_cgo_cast_type(base_cpp)
        .or_else(|| primitive_cgo_cast_type(ty.c_type.trim_end_matches('*').trim()))
        .unwrap_or("C.int");
    prep.setup_lines
        .push(format!("{c_name} := (*{c_type})(unsafe.Pointer({name}))"));
    prep.args.push(c_name);
}

fn render_extern_struct_arg(
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
    require_non_nil: bool,
) {
    let c_name = format!("cArg{index}");
    let go_type = extern_struct_go_type(ty).expect("external struct params must be prefiltered");
    if require_non_nil {
        prep.setup_lines.push(format!("if {name} == nil {{"));
        prep.setup_lines
            .push(format!("    panic(\"{name} reference is nil\")"));
        prep.setup_lines.push("}".to_string());
    }
    prep.setup_lines
        .push(format!("{c_name} := ({go_type})(unsafe.Pointer({name}))"));
    prep.args.push(c_name);
}

fn render_reference_arg(prep: &mut RenderedCallPrep, ty: &IrType, name: &str, index: usize) {
    let go_type =
        go_type_for_reference(ty).expect("primitive references must be filtered before rendering");
    let c_name = format!("cArg{index}");
    prep.setup_lines.push(format!("if {name} == nil {{"));
    prep.setup_lines
        .push(format!("    panic(\"{name} reference is nil\")"));
    prep.setup_lines.push("}".to_string());
    prep.setup_lines
        .push(format!("{c_name} := {}(*{})", cgo_cast_type(ty), name));
    prep.post_call_lines
        .push(format!("*{} = {}({})", name, go_type, c_name));
    prep.args.push(format!("&{c_name}"));
}

fn render_fixed_model_array_arg(
    config: &PipelineContext,
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) {
    let c_handle = ty.handle.as_deref().unwrap_or("");
    let elem_cpp = ir_norm::fixed_array_elem_type(&ty.cpp_type).unwrap_or("");
    let go_name = go_model_return_type(config, ty);
    let handles_name = format!("cHandles{index}");
    let c_name = format!("cArg{index}");
    let elem_expr = render_fixed_model_array_elem_ptr_expr(config, elem_cpp, c_handle, "_v.ptr");
    let has_root = model_handle_has_root(
        config,
        elem_cpp,
        ty.handle.as_ref(),
        covered_handles,
        owned_opaque_value_handles,
    );
    prep.setup_lines.extend(render_fixed_length_guard(name, ty));
    prep.setup_lines.push(format!(
        "{handles_name} := make([]*C.{c_handle}, len({name}))"
    ));
    prep.setup_lines
        .push(format!("for _i, _v := range {name} {{"));
    prep.setup_lines
        .push("    if _v == nil || _v.ptr == nil {".to_string());
    prep.setup_lines.push(format!(
        "        panic(\"{go_name} handle is required but nil\")"
    ));
    prep.setup_lines.push("    }".to_string());
    if has_root {
        prep.setup_lines
            .push("    if _v.root != nil && *_v.root {".to_string());
        prep.setup_lines
            .push(format!("        panic(\"{go_name} handle is closed\")"));
        prep.setup_lines.push("    }".to_string());
    }
    prep.setup_lines
        .push(format!("    {handles_name}[_i] = {elem_expr}"));
    prep.setup_lines.push("}".to_string());
    prep.setup_lines.push(format!(
        "{c_name} := (**C.{c_handle})(unsafe.Pointer(&{handles_name}[0]))"
    ));
    prep.args.push(c_name);
}

fn render_fixed_model_array_elem_ptr_expr(
    config: &PipelineContext,
    elem_cpp: &str,
    expected_handle: &str,
    raw_expr: &str,
) -> String {
    if let Some(projection) = config.known_model_projection(elem_cpp)
        && expected_handle != projection.handle_name
    {
        return format!("(*C.{expected_handle})(unsafe.Pointer({raw_expr}))");
    }
    raw_expr.to_string()
}

fn render_c_arg(ty: &IrType, name: &str) -> String {
    format!("{}({})", cgo_cast_type(ty), name)
}

pub(super) fn indented_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    lines
        .iter()
        .map(|line| format!("    {line}\n"))
        .collect::<String>()
}

fn indent_lines(lines: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    lines
        .lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("{prefix}{line}\n")
            }
        })
        .collect()
}

pub(super) fn has_string_params<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| matches!(param.ty.kind, IrTypeKind::String | IrTypeKind::CString))
}

pub(super) fn has_pointer_params<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| {
        matches!(
            param.ty.kind,
            IrTypeKind::Pointer
                | IrTypeKind::ExternStructPointer
                | IrTypeKind::ExternStructReference
        )
    })
}

pub(super) fn has_byte_array_params<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| param.ty.kind == IrTypeKind::FixedByteArray)
}

pub(super) fn has_fixed_array_params<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| {
        matches!(
            param.ty.kind,
            IrTypeKind::FixedArray | IrTypeKind::FixedModelArray
        )
    })
}

pub(super) fn has_model_handle_cast_params<'a>(
    config: &PipelineContext,
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| model_param_needs_handle_cast(config, &param.ty))
}

fn model_param_needs_handle_cast(config: &PipelineContext, ty: &IrType) -> bool {
    if matches!(
        ty.kind,
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
    ) {
        return config
            .known_model_projection(&ty.cpp_type)
            .zip(ty.handle.as_ref())
            .is_some_and(|(projection, expected_handle)| {
                *expected_handle != projection.handle_name
            });
    }
    if ty.kind == IrTypeKind::FixedModelArray {
        let Some(elem_cpp) = ir_norm::fixed_array_elem_type(&ty.cpp_type) else {
            return false;
        };
        return config
            .known_model_projection(elem_cpp)
            .zip(ty.handle.as_ref())
            .is_some_and(|(projection, expected_handle)| {
                *expected_handle != projection.handle_name
            });
    }
    false
}

pub(super) fn has_void_model_params<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| {
        matches!(
            param.ty.kind,
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
        ) && base_model_cpp_type(&param.ty.cpp_type) == "void"
    })
}

fn render_model_arg(
    config: &PipelineContext,
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) {
    // void model params: the Go type is unsafe.Pointer, which has no .ptr field.
    // Cast directly to *C.<handle> instead.
    if base_model_cpp_type(&ty.cpp_type) == "void" {
        let handle = ty.handle.as_deref().unwrap_or("void");
        let c_name = format!("cArg{index}");
        prep.setup_lines.push(format!("var {c_name} *C.{handle}"));
        prep.setup_lines.push(format!("if {name} != nil {{"));
        prep.setup_lines
            .push(format!("    {c_name} = (*C.{handle})({name})"));
        prep.setup_lines.push("}".to_string());
        prep.args.push(c_name);
        return;
    }
    let c_name = format!("cArg{index}");
    prep.setup_lines.extend(render_model_handle_setup(
        config,
        ty,
        name,
        &c_name,
        covered_handles,
        owned_opaque_value_handles,
    ));
    prep.args.push(c_name);
}

pub(super) fn render_model_handle_setup(
    config: &PipelineContext,
    ty: &IrType,
    name: &str,
    c_name: &str,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> Vec<String> {
    let handle = ty.handle.as_deref().unwrap_or("void");
    let go_name = go_model_return_type(config, ty);
    let ptr_expr = render_model_ptr_expr(config, ty, &format!("{name}.ptr"));
    let has_root = model_arg_has_root(config, ty, covered_handles, owned_opaque_value_handles);
    let mut lines = vec![format!("var {c_name} *C.{handle}")];
    if ty.kind == IrTypeKind::ModelPointer {
        lines.push(format!("if {name} != nil {{"));
        if has_root {
            lines.push(format!("    if {name}.root != nil && *{name}.root {{"));
            lines.push(format!("        panic(\"{go_name} handle is closed\")"));
            lines.push("    }".to_string());
        }
        lines.push(format!("    {c_name} = {ptr_expr}"));
        lines.push("}".to_string());
    } else {
        lines.push(format!("if {name} == nil || {name}.ptr == nil {{"));
        lines.push(format!(
            "    panic(\"{go_name} handle is required but nil\")"
        ));
        lines.push("}".to_string());
        if has_root {
            lines.push(format!("if {name}.root != nil && *{name}.root {{"));
            lines.push(format!("    panic(\"{go_name} handle is closed\")"));
            lines.push("}".to_string());
        }
        lines.push(format!("{c_name} = {ptr_expr}"));
    }
    lines
}

fn model_arg_has_root(
    config: &PipelineContext,
    ty: &IrType,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> bool {
    model_handle_has_root(
        config,
        &ty.cpp_type,
        ty.handle.as_ref(),
        covered_handles,
        owned_opaque_value_handles,
    )
}

fn model_handle_has_root(
    config: &PipelineContext,
    cpp_type: &str,
    handle: Option<&String>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> bool {
    config.known_model_projection(cpp_type).is_some()
        || handle.is_some_and(|handle| {
            covered_handles.contains(handle) || owned_opaque_value_handles.contains(handle)
        })
}

fn render_model_ptr_expr(config: &PipelineContext, ty: &IrType, raw_expr: &str) -> String {
    // When the C function's expected handle type differs from the model projection's
    // handle type (e.g., UCIDHandle* vs _UCIDHandle*), cast via unsafe.Pointer.
    if let Some(projection) = config.known_model_projection(&ty.cpp_type)
        && let Some(expected_handle) = &ty.handle
        && *expected_handle != projection.handle_name
    {
        return format!("(*C.{expected_handle})(unsafe.Pointer({raw_expr}))");
    }
    raw_expr.to_string()
}

pub(super) fn has_callback_param<'a>(
    mut params: impl Iterator<Item = &'a ir_norm::IrParam>,
) -> bool {
    params.any(|param| param.ty.kind == IrTypeKind::Callback)
}

fn infer_borrow_root_expr(
    config: &PipelineContext,
    params: &[&ir_norm::IrParam],
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> Option<String> {
    let model_params = params
        .iter()
        .filter(|param| {
            matches!(
                param.ty.kind,
                IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
            ) && base_model_cpp_type(&param.ty.cpp_type) != "void"
                && model_arg_has_root(
                    config,
                    &param.ty,
                    covered_handles,
                    owned_opaque_value_handles,
                )
        })
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>();
    (model_params.len() == 1).then(|| format!("{}.root", model_params[0]))
}

/// Renders the function body from the C call onwards (call, post_call, nil-check, return).
/// Does NOT include setup/defer lines or the closing `}`.
pub(super) fn render_go_call_return(
    config: &PipelineContext,
    function: &IrFunction,
    call: &str,
    post_call_lines: &[String],
    borrow_root: Option<String>,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    render_go_call_return_with_mode(
        config,
        function,
        call,
        post_call_lines,
        borrow_root,
        ReturnHandleContext {
            covered_handles,
            owned_opaque_value_handles,
        },
        ReturnMode::Direct,
    )
}

fn render_go_call_return_with_mode(
    config: &PipelineContext,
    function: &IrFunction,
    call: &str,
    post_call_lines: &[String],
    borrow_root: Option<String>,
    handles: ReturnHandleContext<'_>,
    mode: ReturnMode,
) -> String {
    let ty = &function.returns;
    match ty.kind {
        IrTypeKind::Void => {
            let mut out = format!("    {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            if matches!(mode, ReturnMode::Dispatcher) {
                out.push_str("    return nil\n");
            }
            out
        }
        IrTypeKind::String => {
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
                crate::config::WRAPPER_PREFIX
            ));
            out
        }
        IrTypeKind::CString => {
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n",
            );
            out
        }
        IrTypeKind::FixedByteArray => {
            let n = ir_norm::byte_array_length(&ty.cpp_type).unwrap_or(0);
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil byte array\")\n    }}\n    defer C.{prefix}_byte_array_free(raw)\n    return C.GoBytes(unsafe.Pointer(raw), C.int({n})), nil\n",
                prefix = crate::config::WRAPPER_PREFIX
            ));
            out
        }
        IrTypeKind::FixedArray => {
            let n = ir_norm::fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let go_elem = fixed_array_go_elem_type(ty);
            let c_elem = fixed_array_cgo_elem_type(ty);
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil array\")\n    }}\n    defer C.{prefix}_array_free(unsafe.Pointer(raw))\n    cSlice := (*[{n}]{c_elem})(unsafe.Pointer(raw))\n    result := make([]{go_elem}, {n})\n    for i := range result {{\n        result[i] = {go_elem}(cSlice[i])\n    }}\n    return result, nil\n",
                prefix = crate::config::WRAPPER_PREFIX
            ));
            out
        }
        IrTypeKind::FixedModelArray => {
            let n = ir_norm::fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let go_name = go_model_return_type(config, ty);
            let c_handle = ty.handle.as_deref().unwrap_or("");
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            if config.known_model_projection(&ty.cpp_type).is_some() {
                out.push_str(&format!(
                    "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil model array\")\n    }}\n    defer C.free(unsafe.Pointer(raw))\n    cSlice := (*[{n}]*C.{c_handle})(unsafe.Pointer(raw))\n    result := make([]*{go_name}, {n})\n    for i := range result {{\n        result[i] = newOwned{go_name}(cSlice[i])\n    }}\n    return result, nil\n"
                ));
            } else {
                out.push_str(&format!(
                    "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil model array\")\n    }}\n    defer C.free(unsafe.Pointer(raw))\n    cSlice := (*[{n}]*C.{c_handle})(unsafe.Pointer(raw))\n    result := make([]*{go_name}, {n})\n    for i := range result {{\n        result[i] = &{go_name}{{ptr: cSlice[i]}}\n    }}\n    return result, nil\n"
                ));
            }
            out
        }
        IrTypeKind::Pointer => {
            let go_type = go_pointer_return_type(ty).unwrap();
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            let suffix = dispatcher_success_suffix(mode);
            out.push_str(&format!(
                "    return ({go_type})(unsafe.Pointer(raw)){suffix}\n"
            ));
            out
        }
        _ if is_model_wrapper_return(ty) => {
            let go_name = go_model_return_type(config, ty);
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            let suffix = dispatcher_success_suffix(mode);
            if go_name == "unsafe.Pointer" {
                out.push_str(&format!("    return unsafe.Pointer(raw){suffix}\n"));
            } else {
                let ptr_expr = cast_raw_to_projection_handle(config, ty, "raw");
                if model_return_has_wrapper_helpers(
                    config,
                    ty,
                    handles.covered_handles,
                    handles.owned_opaque_value_handles,
                ) {
                    let helper = if model_return_uses_inline_owned_literal(config, function, ty) {
                        format!("&{go_name}{{ptr: {ptr_expr}, owned: true, root: new(bool)}}")
                    } else if model_return_is_owned(config, function, ty) {
                        format!("newOwned{go_name}({ptr_expr})")
                    } else {
                        let root_expr = borrow_root.unwrap_or_else(|| "nil".to_string());
                        format!("newBorrowed{go_name}({ptr_expr}, {root_expr})")
                    };
                    out.push_str(&format!(
                        "    if raw == nil {{\n        return nil{suffix}\n    }}\n    return {helper}{suffix}\n"
                    ));
                } else {
                    out.push_str(&format!(
                        "    if raw == nil {{\n        return nil{suffix}\n    }}\n    return &{go_name}{{ptr: {ptr_expr}}}{suffix}\n"
                    ));
                }
            }
            out
        }
        _ => {
            let go_type = go_value_type(config, ty).unwrap();
            let mut out = String::new();
            if go_type == "bool" {
                let suffix = dispatcher_success_suffix(mode);
                out.push_str(&format!("    result := {call}\n"));
                out.push_str(&indented_lines(post_call_lines));
                out.push_str(&format!("    return bool(result){suffix}\n"));
            } else if post_call_lines.is_empty() && matches!(mode, ReturnMode::Direct) {
                out.push_str(&format!("    return {go_type}({call})\n"));
            } else {
                let suffix = dispatcher_success_suffix(mode);
                out.push_str(&format!("    result := {call}\n"));
                out.push_str(&indented_lines(post_call_lines));
                out.push_str(&format!("    return {go_type}(result){suffix}\n"));
            }
            out
        }
    }
}

fn dispatcher_success_suffix(mode: ReturnMode) -> &'static str {
    match mode {
        ReturnMode::Direct => "",
        ReturnMode::Dispatcher => ", nil",
    }
}
