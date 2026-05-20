use std::collections::BTreeMap;

use crate::{
    domain::kind::IrTypeKind,
    ir::{IrCallback, IrFunction, IrModule},
};

use super::{AnalyzedFacadeClass, CallbackUsage, support::*};

pub(super) fn collect_callback_usages<'a>(
    functions: &[&'a IrFunction],
    classes: &[AnalyzedFacadeClass<'a>],
    ir: &'a IrModule,
) -> Vec<CallbackUsage<'a>> {
    let callbacks = ir
        .callbacks
        .iter()
        .map(|callback| (callback.name.as_str(), callback))
        .collect::<BTreeMap<_, _>>();
    let mut usages = Vec::new();

    for function in functions {
        usages.extend(callback_usages_for_function(function, &callbacks));
    }
    for class in classes {
        for function in &class.methods {
            usages.extend(callback_usages_for_function(function, &callbacks));
        }
    }

    usages
}

fn callback_usages_for_function<'a>(
    function: &'a IrFunction,
    callbacks: &BTreeMap<&str, &'a IrCallback>,
) -> Vec<CallbackUsage<'a>> {
    function
        .params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| {
            (param.ty.kind == IrTypeKind::Callback).then(|| {
                callbacks
                    .get(param.ty.cpp_type.as_str())
                    .map(|callback| CallbackUsage {
                        callback,
                        function,
                        param_index: index,
                    })
            })?
        })
        .collect()
}

pub(super) fn used_callbacks<'a>(usages: &'a [CallbackUsage<'a>]) -> Vec<&'a IrCallback> {
    let mut seen = BTreeMap::<String, &'a IrCallback>::new();
    for usage in usages {
        seen.entry(usage.callback.name.clone())
            .or_insert(usage.callback);
    }
    seen.into_values().collect()
}

pub(super) fn render_callback_type(callback: &IrCallback) -> String {
    let params = callback
        .params
        .iter()
        .map(|param| format!("{} {}", param.name, callback_go_type(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let returns = if callback.returns.kind == IrTypeKind::Void {
        String::new()
    } else {
        format!(" {}", callback_go_type(&callback.returns))
    };
    format!("type {} func({}){}\n", callback.name, params, returns)
}

pub(super) fn render_callback_registry(usage: &CallbackUsage<'_>) -> String {
    format!(
        "var {} struct {{\n    mu sync.RWMutex\n    fn {}\n}}\n",
        callback_state_name(usage),
        usage.callback.name
    )
}

pub(super) fn render_callback_export(usage: &CallbackUsage<'_>) -> String {
    let params = usage
        .callback
        .params
        .iter()
        .map(|param| format!("{} {}", param.name, callback_cgo_param_type(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str(&format!("//export {}\n", callback_go_export_name(usage)));
    out.push_str(&format!(
        "func {}({})",
        callback_go_export_name(usage),
        params
    ));
    if usage.callback.returns.kind != IrTypeKind::Void {
        out.push_str(&format!(
            " {}",
            callback_cgo_return_type(&usage.callback.returns)
        ));
    }
    out.push_str(" {\n");
    out.push_str(&format!(
        "    {}.mu.RLock()\n    fn := {}.fn\n    {}.mu.RUnlock()\n    if fn == nil {{\n",
        callback_state_name(usage),
        callback_state_name(usage),
        callback_state_name(usage)
    ));
    if usage.callback.returns.kind == IrTypeKind::Void {
        out.push_str("        return\n");
    } else {
        out.push_str(&format!(
            "        return {}\n",
            zero_value_for_go_type(go_type_for_ir(&usage.callback.returns).unwrap_or("int"))
        ));
    }
    out.push_str("    }\n");
    let args = usage
        .callback
        .params
        .iter()
        .map(|param| render_callback_go_arg(&param.ty, &param.name))
        .collect::<Vec<_>>()
        .join(", ");
    if usage.callback.returns.kind == IrTypeKind::Void {
        out.push_str(&format!("    fn({})\n", args));
    } else {
        out.push_str(&format!(
            "    return {}(fn({}))\n",
            callback_cgo_return_type(&usage.callback.returns),
            args
        ));
    }
    out.push_str("}\n");
    out
}
