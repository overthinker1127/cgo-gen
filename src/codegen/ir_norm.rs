use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::Serialize;

pub use crate::domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind, RecordKind};

use crate::{
    config::{Config, WRAPPER_PREFIX},
    parser::{
        CppCallbackTypedef, CppConstructor, CppEnum, CppField, CppFunction, CppMacroConstant,
        CppMethod, CppParam, CppRecord, ParsedApi,
    },
    pipeline::context::PipelineContext,
};

#[derive(Debug, Clone, Serialize)]
pub struct IrModule {
    pub version: u32,
    pub module: String,
    pub source_headers: Vec<String>,
    pub records: Vec<IrRecord>,
    pub opaque_types: Vec<OpaqueType>,
    pub functions: Vec<IrFunction>,
    pub enums: Vec<IrEnum>,
    pub constants: Vec<IrMacroConstant>,
    pub callbacks: Vec<IrCallback>,
    pub support: SupportMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpaqueType {
    pub name: String,
    pub cpp_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrRecord {
    pub cpp_type: String,
    pub handle_name: String,
    pub kind: RecordKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFunction {
    pub name: String,
    pub kind: IrFunctionKind,
    pub cpp_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_cpp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_const: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_accessor: Option<IrFieldAccessor>,
    pub returns: IrType,
    pub params: Vec<IrParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFieldAccessor {
    pub field_name: String,
    pub access: FieldAccessKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrParam {
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrType {
    pub kind: IrTypeKind,
    pub cpp_type: String,
    pub c_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrCallback {
    pub name: String,
    pub cpp_name: String,
    pub returns: IrType,
    pub params: Vec<IrParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEnum {
    pub name: String,
    pub cpp_name: String,
    #[serde(skip)]
    pub is_anonymous: bool,
    pub variants: Vec<IrEnumVariant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEnumVariant {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMacroConstant {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportMetadata {
    pub parser_backend: String,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_declarations: Vec<SkippedDeclaration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedDeclaration {
    pub cpp_name: String,
    pub reason: String,
}

pub fn normalize(ctx: &PipelineContext, api: &ParsedApi) -> Result<IrModule> {
    let config = &ctx.config;
    let module = WRAPPER_PREFIX.to_string();
    let mut opaque_types = Vec::new();
    let mut functions = Vec::new();
    let mut enums = Vec::new();
    let mut constants = Vec::new();
    let mut callbacks = Vec::new();
    let mut skipped_declarations = Vec::new();
    let callback_names = callback_name_set(api);
    let known_enum_types = if ctx.known_enum_types.is_empty() {
        collect_known_enum_types(api)
    } else {
        ctx.known_enum_types.iter().cloned().collect()
    };
    let abstract_types = api
        .records
        .iter()
        .filter(|record| record.is_abstract)
        .map(|record| cpp_qualified(&record.namespace, &record.name))
        .collect::<BTreeSet<_>>();
    let preferred_model_aliases = if ctx.preferred_model_aliases.is_empty() {
        collect_preferred_model_aliases(api)
    } else {
        ctx.preferred_model_aliases.clone()
    };
    let records = collect_ir_records(api, &preferred_model_aliases);

    for record in &records {
        opaque_types.push(OpaqueType {
            name: record.handle_name.clone(),
            cpp_type: record.cpp_type.clone(),
        });
    }

    for record in &api.records {
        let qualified = cpp_qualified(&record.namespace, &record.name);
        let handle_name = records
            .iter()
            .find(|item| item.cpp_type == qualified)
            .map(|item| item.handle_name.as_str())
            .unwrap_or("");
        functions.extend(normalize_record(
            config,
            record,
            handle_name,
            &abstract_types,
            &callback_names,
            &known_enum_types,
            &records,
            &mut skipped_declarations,
        )?);
    }

    let free_signature_groups = collect_function_signature_groups(&api.functions);
    let mut emitted_free_signatures = BTreeMap::<String, BTreeSet<Vec<String>>>::new();
    for function in &api.functions {
        let group_key = cpp_qualified(&function.namespace, &function.name);
        let existing_signatures = free_signature_groups
            .get(&group_key)
            .cloned()
            .unwrap_or_default();
        let emitted_signatures = emitted_free_signatures.entry(group_key).or_default();
        for params in default_argument_param_variants(
            &function.params,
            &existing_signatures,
            emitted_signatures,
        ) {
            if let Some(function) = normalize_function(
                config,
                function,
                params,
                &abstract_types,
                &callback_names,
                &known_enum_types,
                &records,
                &mut skipped_declarations,
            )? {
                functions.push(function);
            }
        }
    }

    for item in &api.enums {
        enums.push(normalize_enum(item));
    }
    for item in &api.macros {
        constants.push(normalize_macro_constant(item));
    }
    for callback in &api.callbacks {
        callbacks.push(normalize_callback(
            config,
            callback,
            &callback_names,
            &known_enum_types,
            &records,
        )?);
    }

    collect_referenced_opaque_types(&mut opaque_types, &functions);

    assign_unique_function_symbols(&mut functions);
    ensure_unique_function_symbols(&functions)?;

    Ok(IrModule {
        version: config.version.unwrap_or(1),
        module,
        source_headers: api.headers.clone(),
        records,
        opaque_types,
        functions,
        enums,
        constants,
        callbacks,
        support: SupportMetadata {
            parser_backend: "libclang".to_string(),
            notes: {
                let mut notes = vec![
                    "Parsed with clang AST and normalized into a conservative C ABI IR."
                        .to_string(),
                    "v1 intentionally rejects unsupported C++ constructs during type normalization."
                        .to_string(),
                ];
                if !skipped_declarations.is_empty() {
                    notes.push(
                        "Skipped declarations are recorded in support.skipped_declarations when v1 cannot safely express them in raw output.".to_string(),
                    );
                }
                notes
            },
            skipped_declarations,
        },
    })
}

fn collect_referenced_opaque_types(opaque_types: &mut Vec<OpaqueType>, functions: &[IrFunction]) {
    let mut known = opaque_types
        .iter()
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();

    for function in functions {
        for ty in
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        {
            let Some(handle) = &ty.handle else {
                continue;
            };
            if known.contains(handle) {
                continue;
            }
            if !matches!(
                ty.kind,
                IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
            ) {
                continue;
            }

            opaque_types.push(OpaqueType {
                name: handle.clone(),
                cpp_type: base_model_cpp_type(&ty.cpp_type),
            });
            known.insert(handle.clone());
        }
    }
}

fn collect_ir_records(
    api: &ParsedApi,
    preferred_model_aliases: &BTreeMap<String, String>,
) -> Vec<IrRecord> {
    api.records
        .iter()
        .map(|record| {
            let cpp_type = cpp_qualified(&record.namespace, &record.name);
            let handle_name = stable_class_handle_name(&cpp_type, preferred_model_aliases);
            IrRecord {
                cpp_type,
                handle_name,
                kind: record.kind,
            }
        })
        .collect()
}

pub fn collect_preferred_model_aliases(api: &ParsedApi) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();

    for record in &api.records {
        for field in &record.fields {
            record_preferred_model_alias(&mut aliases, &field.ty, &field.canonical_ty);
        }
        for constructor in &record.constructors {
            for param in &constructor.params {
                record_preferred_model_alias(&mut aliases, &param.ty, &param.canonical_ty);
            }
        }
        for method in &record.methods {
            record_preferred_model_alias(
                &mut aliases,
                &method.return_type,
                &method.return_canonical_type,
            );
            for param in &method.params {
                record_preferred_model_alias(&mut aliases, &param.ty, &param.canonical_ty);
            }
        }
    }

    for function in &api.functions {
        record_preferred_model_alias(
            &mut aliases,
            &function.return_type,
            &function.return_canonical_type,
        );
        for param in &function.params {
            record_preferred_model_alias(&mut aliases, &param.ty, &param.canonical_ty);
        }
    }

    for callback in &api.callbacks {
        record_preferred_model_alias(
            &mut aliases,
            &callback.return_type,
            &callback.return_canonical_type,
        );
        for param in &callback.params {
            record_preferred_model_alias(&mut aliases, &param.ty, &param.canonical_ty);
        }
    }

    aliases
}

fn collect_known_enum_types(api: &ParsedApi) -> BTreeSet<String> {
    api.enums
        .iter()
        .filter(|item| !item.is_anonymous)
        .map(|item| cpp_qualified(&item.namespace, &item.name))
        .collect()
}

fn record_preferred_model_alias(
    aliases: &mut BTreeMap<String, String>,
    display_type: &str,
    canonical_type: &str,
) {
    let Some(display_base) = model_alias_base_type(display_type) else {
        return;
    };
    let Some(canonical_base) = model_alias_base_type(canonical_type) else {
        return;
    };
    if display_base == canonical_base {
        return;
    }

    let preferred = preferred_model_base_name(&display_base, &canonical_base);
    if preferred == canonical_base {
        return;
    }

    aliases.entry(canonical_base).or_insert(preferred);
}

fn stable_class_handle_name(
    qualified_cpp_type: &str,
    aliases: &BTreeMap<String, String>,
) -> String {
    let preferred = aliases
        .get(qualified_cpp_type)
        .cloned()
        .unwrap_or_else(|| qualified_cpp_type.to_string());
    stable_model_handle_name(&preferred)
}

fn preferred_model_base_name(display_base: &str, canonical_base: &str) -> String {
    if !display_base.starts_with('_') {
        display_base.to_string()
    } else if !canonical_base.starts_with('_') {
        canonical_base.to_string()
    } else {
        display_base.to_string()
    }
}

fn stable_model_handle_name(base_cpp_type: &str) -> String {
    format!("{}Handle", flatten_qualified_cpp_name(base_cpp_type))
}

fn model_alias_base_type(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_start_matches("const ").trim();
    if let Some((elem, _)) = parse_array_type(trimmed) {
        return model_alias_base_type(elem);
    }

    let base = base_model_cpp_type(trimmed);
    if base.is_empty()
        || base == "void"
        || base.contains('<')
        || base.starts_with("std::")
        || base.starts_with("struct ")
        || base.contains('[')
        || base.contains(']')
        || base.contains('(')
        || base.contains(')')
        || is_supported_primitive(&base)
    {
        return None;
    }

    Some(base)
}

fn assign_unique_function_symbols(functions: &mut [IrFunction]) {
    let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, function) in functions.iter().enumerate() {
        by_symbol
            .entry(function.name.clone())
            .or_default()
            .push(index);
    }

    for (base_name, indexes) in by_symbol {
        if indexes.len() < 2 {
            continue;
        }

        let mut assigned: BTreeMap<String, usize> = BTreeMap::new();
        for index in indexes {
            let suffix = overload_suffix(&functions[index]);
            let candidate = format!("{base_name}__{suffix}");
            let occurrence = assigned.entry(candidate.clone()).or_insert(0);
            *occurrence += 1;
            if *occurrence == 1 {
                functions[index].name = candidate;
            } else {
                functions[index].name = format!("{candidate}_{}", occurrence);
            }
        }
    }
}

fn ensure_unique_function_symbols(functions: &[IrFunction]) -> Result<()> {
    let mut by_symbol: BTreeMap<&str, Vec<&IrFunction>> = BTreeMap::new();
    for function in functions {
        by_symbol
            .entry(function.name.as_str())
            .or_default()
            .push(function);
    }

    let duplicates = by_symbol
        .into_iter()
        .filter(|(_, items)| items.len() > 1)
        .collect::<Vec<_>>();

    if duplicates.is_empty() {
        return Ok(());
    }

    let message = duplicates
        .into_iter()
        .map(|(symbol, items)| {
            let origins = items
                .into_iter()
                .map(|item| item.cpp_name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("wrapper symbol `{symbol}` collides for C++ declarations: {origins}")
        })
        .collect::<Vec<_>>()
        .join("; ");

    bail!("overload collision detected after suffix assignment: {message}")
}

fn collect_function_signature_groups(
    functions: &[CppFunction],
) -> BTreeMap<String, BTreeSet<Vec<String>>> {
    let mut out = BTreeMap::new();
    for function in functions {
        out.entry(cpp_qualified(&function.namespace, &function.name))
            .or_insert_with(BTreeSet::new)
            .insert(cpp_param_signature(&function.params));
    }
    out
}

fn collect_constructor_signature_set(constructors: &[CppConstructor]) -> BTreeSet<Vec<String>> {
    constructors
        .iter()
        .map(|constructor| cpp_param_signature(&constructor.params))
        .collect()
}

fn collect_method_signature_groups(
    methods: &[CppMethod],
) -> BTreeMap<(String, bool), BTreeSet<Vec<String>>> {
    let mut out = BTreeMap::new();
    for method in methods {
        out.entry((method.name.clone(), method.is_const))
            .or_insert_with(BTreeSet::new)
            .insert(cpp_param_signature(&method.params));
    }
    out
}

fn default_argument_param_variants<'a>(
    params: &'a [CppParam],
    existing_signatures: &BTreeSet<Vec<String>>,
    emitted_signatures: &mut BTreeSet<Vec<String>>,
) -> Vec<&'a [CppParam]> {
    let mut lengths = Vec::new();
    lengths.push(params.len());

    let mut len = params.len();
    while len > 0 && params[len - 1].has_default {
        len -= 1;
        lengths.push(len);
    }

    let mut out = Vec::new();
    for len in lengths {
        let variant = &params[..len];
        let signature = cpp_param_signature(variant);
        if len < params.len() && existing_signatures.contains(&signature) {
            continue;
        }
        if emitted_signatures.insert(signature) {
            out.push(variant);
        }
    }
    out
}

fn cpp_param_signature(params: &[CppParam]) -> Vec<String> {
    params
        .iter()
        .map(|param| param.canonical_ty.clone())
        .collect()
}

fn normalize_record(
    config: &Config,
    record: &CppRecord,
    handle_name: &str,
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Vec<IrFunction>> {
    let mut functions = Vec::new();
    let qualified = cpp_qualified(&record.namespace, &record.name);

    if record.is_abstract {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason: "abstract class: has pure virtual methods; constructor wrapper omitted"
                .to_string(),
        });
    } else if record.constructors.is_empty() {
        functions.push(IrFunction {
            name: symbol_name(config, &record.namespace, &record.name, "new"),
            kind: IrFunctionKind::Constructor,
            cpp_name: qualified.clone(),
            method_of: Some(handle_name.to_string()),
            owner_cpp_type: Some(qualified.clone()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: qualified.clone(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name.to_string()),
            },
            params: Vec::new(),
        });
    } else {
        let initial_len = functions.len();
        let existing_signatures = collect_constructor_signature_set(&record.constructors);
        let mut emitted_signatures = BTreeSet::new();
        for constructor in &record.constructors {
            for params in default_argument_param_variants(
                &constructor.params,
                &existing_signatures,
                &mut emitted_signatures,
            ) {
                if let Some(function) = normalize_constructor(
                    config,
                    record,
                    handle_name,
                    params,
                    callback_names,
                    known_enum_types,
                    known_records,
                    skipped_declarations,
                )? {
                    functions.push(function);
                }
            }
        }
        if functions.len() == initial_len {
            bail!(
                "class {qualified} declares constructors, but none were eligible for wrapper generation; refusing to synthesize a default constructor"
            );
        }
    }

    functions.push(IrFunction {
        name: symbol_name(config, &record.namespace, &record.name, "delete"),
        kind: IrFunctionKind::Destructor,
        cpp_name: if record.has_destructor {
            format!("~{}", qualified)
        } else {
            qualified.clone()
        },
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.clone()),
        is_const: None,
        field_accessor: None,
        returns: primitive_type("void"),
        params: vec![IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: format!("{}*", qualified),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name.to_string()),
            },
        }],
    });

    let method_signature_groups = collect_method_signature_groups(&record.methods);
    let mut emitted_method_signatures = BTreeMap::<(String, bool), BTreeSet<Vec<String>>>::new();
    for method in &record.methods {
        let group_key = (method.name.clone(), method.is_const);
        let existing_signatures = method_signature_groups
            .get(&group_key)
            .cloned()
            .unwrap_or_default();
        let emitted_signatures = emitted_method_signatures.entry(group_key).or_default();
        for params in default_argument_param_variants(
            &method.params,
            &existing_signatures,
            emitted_signatures,
        ) {
            if let Some(function) = normalize_method(
                config,
                record,
                handle_name,
                method,
                params,
                abstract_types,
                callback_names,
                known_enum_types,
                known_records,
                skipped_declarations,
            )? {
                functions.push(function);
            }
        }
    }

    if record.kind == RecordKind::Struct {
        functions.extend(normalize_struct_fields(
            config,
            record,
            handle_name,
            callback_names,
            known_enum_types,
            known_records,
        )?);
    }

    Ok(functions)
}

fn normalize_struct_fields(
    config: &Config,
    record: &CppRecord,
    handle_name: &str,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Result<Vec<IrFunction>> {
    let qualified = cpp_qualified(&record.namespace, &record.name);
    let existing_methods = record
        .methods
        .iter()
        .map(|method| method.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut functions = Vec::new();

    for field in &record.fields {
        if field.is_function_pointer {
            continue;
        }

        let suffix = struct_field_accessor_suffix(&field.name);
        let getter_name = format!("Get{suffix}");
        if existing_methods.contains(getter_name.as_str()) {
            continue;
        }

        let Ok(field_ty) = normalize_type_with_canonical(
            config,
            &field.ty,
            &field.canonical_ty,
            callback_names,
            known_enum_types,
            known_records,
        ) else {
            continue;
        };
        if field_ty.kind != IrTypeKind::Primitive
            && field_ty.kind != IrTypeKind::Enum
            && field_ty.kind != IrTypeKind::ModelValue
            && field_ty.kind != IrTypeKind::CString
            && field_ty.kind != IrTypeKind::FixedByteArray
            && field_ty.kind != IrTypeKind::FixedArray
            && field_ty.kind != IrTypeKind::FixedModelArray
        {
            continue;
        }

        let is_fixed_model_array = field_ty.kind == IrTypeKind::FixedModelArray;

        functions.push(make_struct_field_getter(
            config,
            &record.namespace,
            &record.name,
            &qualified,
            handle_name,
            field,
            field_ty.clone(),
        ));

        if is_fixed_model_array {
            functions.push(make_struct_field_indexed_getter(
                config,
                &record.namespace,
                &record.name,
                &qualified,
                handle_name,
                field,
                &field_ty,
            ));
        }

        let setter_name = format!("Set{suffix}");
        if existing_methods.contains(setter_name.as_str()) || field_is_read_only(field) {
            continue;
        }

        let setter_field_ty = field_ty.clone();
        functions.push(make_struct_field_setter(
            config,
            &record.namespace,
            &record.name,
            &qualified,
            handle_name,
            field,
            setter_field_ty,
        ));

        if is_fixed_model_array {
            functions.push(make_struct_field_indexed_setter(
                config,
                &record.namespace,
                &record.name,
                &qualified,
                handle_name,
                field,
                &field_ty,
            ));
        }
    }

    Ok(functions)
}

fn make_struct_field_getter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    returns: IrType,
) -> IrFunction {
    let method_name = format!("Get{}", struct_field_accessor_suffix(&field.name));
    let returns = if returns.kind == IrTypeKind::ModelValue {
        IrType {
            kind: IrTypeKind::ModelPointer,
            cpp_type: format!("{}*", base_model_cpp_type(&returns.cpp_type)),
            ..returns
        }
    } else {
        returns
    };
    let getter_self_ty = if returns.kind == IrTypeKind::ModelPointer {
        IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: format!("{}*", qualified),
            c_type: format!("{handle_name}*"),
            handle: Some(handle_name.to_string()),
        }
    } else {
        IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: format!("const {}*", qualified),
            c_type: format!("const {handle_name}*"),
            handle: Some(handle_name.to_string()),
        }
    };
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(returns.kind != IrTypeKind::ModelPointer),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::Get,
            array_len: None,
        }),
        returns,
        params: vec![IrParam {
            name: "self".to_string(),
            ty: getter_self_ty,
        }],
    }
}

fn make_struct_field_setter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    field_ty: IrType,
) -> IrFunction {
    let method_name = format!("Set{}", struct_field_accessor_suffix(&field.name));
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(false),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::Set,
            array_len: None,
        }),
        returns: primitive_type("void"),
        params: vec![
            IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: format!("{}*", qualified),
                    c_type: format!("{handle_name}*"),
                    handle: Some(handle_name.to_string()),
                },
            },
            IrParam {
                name: "value".to_string(),
                ty: field_ty,
            },
        ],
    }
}

fn make_struct_field_indexed_getter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    field_ty: &IrType,
) -> IrFunction {
    let method_name = format!("Get{}At", struct_field_accessor_suffix(&field.name));
    let elem_cpp = fixed_array_elem_type(&field_ty.cpp_type).unwrap_or("void");
    let elem_handle = field_ty.handle.clone().unwrap_or_default();
    let array_len = fixed_array_length(&field_ty.cpp_type).unwrap_or(0);
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(true),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::GetAt,
            array_len: Some(array_len),
        }),
        returns: IrType {
            kind: IrTypeKind::ModelPointer,
            cpp_type: format!("{elem_cpp}*"),
            c_type: format!("{elem_handle}*"),
            handle: Some(elem_handle),
        },
        params: vec![
            IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: format!("const {}*", qualified),
                    c_type: format!("const {handle_name}*"),
                    handle: Some(handle_name.to_string()),
                },
            },
            IrParam {
                name: "index".to_string(),
                ty: primitive_type("int"),
            },
        ],
    }
}

fn make_struct_field_indexed_setter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    field_ty: &IrType,
) -> IrFunction {
    let method_name = format!("Set{}At", struct_field_accessor_suffix(&field.name));
    let elem_cpp = fixed_array_elem_type(&field_ty.cpp_type).unwrap_or("void");
    let elem_handle = field_ty.handle.clone().unwrap_or_default();
    let array_len = fixed_array_length(&field_ty.cpp_type).unwrap_or(0);
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(false),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::SetAt,
            array_len: Some(array_len),
        }),
        returns: primitive_type("void"),
        params: vec![
            IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: format!("{}*", qualified),
                    c_type: format!("{handle_name}*"),
                    handle: Some(handle_name.to_string()),
                },
            },
            IrParam {
                name: "index".to_string(),
                ty: primitive_type("int"),
            },
            IrParam {
                name: "value".to_string(),
                ty: IrType {
                    kind: IrTypeKind::ModelPointer,
                    cpp_type: format!("{elem_cpp}*"),
                    c_type: format!("{elem_handle}*"),
                    handle: Some(elem_handle),
                },
            },
        ],
    }
}

fn field_is_read_only(field: &CppField) -> bool {
    let ty = field.ty.trim();
    let canonical = field.canonical_ty.trim();
    ty.starts_with("const ") || canonical.starts_with("const ")
}

fn struct_field_accessor_suffix(field_name: &str) -> String {
    field_name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = first.to_uppercase().collect::<String>();
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn normalize_constructor(
    config: &Config,
    record: &CppRecord,
    handle_name: &str,
    cpp_params: &[CppParam],
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&record.namespace, &record.name);
    if let Some(reason) = function_pointer_reason(None, cpp_params, callback_names) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason,
        });
        return Ok(None);
    }
    if let Some(reason) = double_pointer_reason(None, cpp_params) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason,
        });
        return Ok(None);
    }
    Ok(Some(IrFunction {
        name: symbol_name(config, &record.namespace, &record.name, "new"),
        kind: IrFunctionKind::Constructor,
        cpp_name: qualified.clone(),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.clone()),
        is_const: None,
        field_accessor: None,
        returns: IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: qualified.clone(),
            c_type: format!("{handle_name}*"),
            handle: Some(handle_name.to_string()),
        },
        params: cpp_params
            .iter()
            .map(|param| {
                normalize_param(
                    config,
                    param,
                    callback_names,
                    known_enum_types,
                    known_records,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    }))
}

fn normalize_method(
    config: &Config,
    record: &CppRecord,
    handle_name: &str,
    method: &CppMethod,
    cpp_params: &[CppParam],
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&record.namespace, &record.name);
    let cpp_name = format!("{}::{}", qualified, method.name);
    if is_operator_name(&method.name) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name,
            reason: "operator declarations are unsupported in v1".to_string(),
        });
        return Ok(None);
    }
    if let Some(reason) = function_pointer_reason(
        Some((
            &method.return_type,
            &method.return_canonical_type,
            method.return_is_function_pointer,
        )),
        cpp_params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = double_pointer_reason(
        Some((&method.return_type, &method.return_canonical_type)),
        cpp_params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&method.return_type, &method.return_canonical_type)),
        cpp_params,
        callback_names,
        known_enum_types,
        known_records,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    let mut params = Vec::new();
    params.push(IrParam {
        name: "self".to_string(),
        ty: IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: if method.is_const {
                format!("const {}*", qualified)
            } else {
                format!("{}*", qualified)
            },
            c_type: if method.is_const {
                format!("const {handle_name}*")
            } else {
                format!("{handle_name}*")
            },
            handle: Some(handle_name.to_string()),
        },
    });
    params.extend(
        cpp_params
            .iter()
            .map(|param| {
                normalize_param(
                    config,
                    param,
                    callback_names,
                    known_enum_types,
                    known_records,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(Some(IrFunction {
        name: symbol_name(config, &record.namespace, &record.name, &method.name),
        kind: IrFunctionKind::Method,
        cpp_name,
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified),
        is_const: Some(method.is_const),
        field_accessor: None,
        returns: normalize_return_type_with_canonical(
            config,
            &method.return_type,
            &method.return_canonical_type,
            abstract_types,
            callback_names,
            known_enum_types,
            known_records,
        )?,
        params,
    }))
}

fn normalize_function(
    config: &Config,
    function: &CppFunction,
    cpp_params: &[CppParam],
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let cpp_name = cpp_qualified(&function.namespace, &function.name);
    if is_operator_name(&function.name) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name,
            reason: "operator declarations are unsupported in v1".to_string(),
        });
        return Ok(None);
    }
    if let Some(reason) = function_pointer_reason(
        Some((
            &function.return_type,
            &function.return_canonical_type,
            function.return_is_function_pointer,
        )),
        cpp_params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = double_pointer_reason(
        Some((&function.return_type, &function.return_canonical_type)),
        cpp_params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&function.return_type, &function.return_canonical_type)),
        cpp_params,
        callback_names,
        known_enum_types,
        known_records,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    Ok(Some(IrFunction {
        name: symbol_name(config, &function.namespace, "", &function.name),
        kind: IrFunctionKind::Function,
        cpp_name,
        method_of: None,
        owner_cpp_type: None,
        is_const: None,
        field_accessor: None,
        returns: normalize_return_type_with_canonical(
            config,
            &function.return_type,
            &function.return_canonical_type,
            abstract_types,
            callback_names,
            known_enum_types,
            known_records,
        )?,
        params: cpp_params
            .iter()
            .map(|param| {
                normalize_param(
                    config,
                    param,
                    callback_names,
                    known_enum_types,
                    known_records,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    }))
}

fn normalize_enum(item: &CppEnum) -> IrEnum {
    IrEnum {
        name: flatten_cpp_name(&item.namespace, &item.name),
        cpp_name: cpp_qualified(&item.namespace, &item.name),
        is_anonymous: item.is_anonymous,
        variants: item
            .variants
            .iter()
            .map(|variant| IrEnumVariant {
                name: variant.name.clone(),
                value: variant.value.clone(),
            })
            .collect(),
    }
}

fn normalize_macro_constant(item: &CppMacroConstant) -> IrMacroConstant {
    IrMacroConstant {
        name: item.name.clone(),
        value: item.value.clone(),
    }
}

fn normalize_callback(
    config: &Config,
    callback: &CppCallbackTypedef,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Result<IrCallback> {
    Ok(IrCallback {
        name: callback.name.clone(),
        cpp_name: cpp_qualified(&callback.namespace, &callback.name),
        returns: normalize_type_with_canonical(
            config,
            &callback.return_type,
            &callback.return_canonical_type,
            callback_names,
            known_enum_types,
            known_records,
        )?,
        params: callback
            .params
            .iter()
            .map(|param| {
                normalize_param(
                    config,
                    param,
                    callback_names,
                    known_enum_types,
                    known_records,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    })
}

fn sanitize_go_param_name(name: &str) -> String {
    const GO_KEYWORDS: &[&str] = &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "type",
        "var",
    ];
    if GO_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

fn normalize_param(
    config: &Config,
    param: &CppParam,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Result<IrParam> {
    Ok(IrParam {
        name: sanitize_go_param_name(&param.name),
        ty: normalize_type_with_canonical(
            config,
            &param.ty,
            &param.canonical_ty,
            callback_names,
            known_enum_types,
            known_records,
        )?,
    })
}

fn normalize_return_type_with_canonical(
    config: &Config,
    cpp_type: &str,
    canonical_type: &str,
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Result<IrType> {
    let ty = normalize_type_with_canonical(
        config,
        cpp_type,
        canonical_type,
        callback_names,
        known_enum_types,
        known_records,
    )?;
    if matches!(
        ty.kind,
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer
    ) && is_abstract_model_type(&ty.cpp_type, abstract_types)
    {
        return Ok(ty);
    }
    Ok(ty)
}

fn is_abstract_model_type(cpp_type: &str, abstract_types: &BTreeSet<String>) -> bool {
    let base = base_model_cpp_type(cpp_type);
    !base.is_empty() && abstract_types.contains(&base)
}

fn function_pointer_reason(
    return_type: Option<(&str, &str, bool)>,
    params: &[CppParam],
    callback_names: &BTreeSet<String>,
) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical, is_function_pointer)) = return_type
        && is_function_pointer
    {
        issues.push(format!(
            "return type `{}` uses a function pointer",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if param.is_function_pointer && !is_named_callback_param(param, callback_names) {
            issues.push(format!(
                "parameter `{}` type `{}` uses a function pointer",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn double_pointer_reason(return_type: Option<(&str, &str)>, params: &[CppParam]) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical)) = return_type
        && (is_unsupported_double_pointer_type(display)
            || (!canonical.trim().is_empty() && is_unsupported_double_pointer_type(canonical)))
    {
        issues.push(format!(
            "return type `{}` uses an unsupported double-pointer type",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if is_unsupported_double_pointer_type(&param.ty)
            || (!param.canonical_ty.trim().is_empty()
                && is_unsupported_double_pointer_type(&param.canonical_ty))
        {
            issues.push(format!(
                "parameter `{}` type `{}` uses an unsupported double-pointer type",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn raw_unsafe_by_value_reason(
    return_type: Option<(&str, &str)>,
    params: &[CppParam],
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical)) = return_type
        && is_raw_unsafe_by_value_return_type(
            display,
            canonical,
            callback_names,
            known_enum_types,
            known_records,
        )
    {
        issues.push(format!(
            "return type `{}` uses a raw-unsafe by-value object type",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if is_raw_unsafe_by_value_param_type(
            &param.ty,
            &param.canonical_ty,
            callback_names,
            known_enum_types,
            known_records,
        ) {
            issues.push(format!(
                "parameter `{}` type `{}` uses a raw-unsafe by-value object type",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn is_raw_unsafe_by_value_return_type(
    display: &str,
    canonical: &str,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> bool {
    if normalize_type_with_canonical(
        &Config::default(),
        display,
        canonical,
        callback_names,
        known_enum_types,
        known_records,
    )
    .is_ok()
    {
        return false;
    }
    if normalize_type(display, callback_names).is_ok() {
        return false;
    }
    if !canonical.trim().is_empty()
        && canonical.trim() != display.trim()
        && normalize_type(canonical, callback_names).is_ok()
    {
        return false;
    }

    is_raw_unsafe_by_value_object_candidate(display)
        || (!canonical.trim().is_empty()
            && canonical.trim() != display.trim()
            && is_raw_unsafe_by_value_object_candidate(canonical))
}

fn is_raw_unsafe_by_value_param_type(
    display: &str,
    canonical: &str,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> bool {
    let display = display.trim();
    let canonical = canonical.trim();

    if let Ok(ty) = normalize_type_with_canonical(
        &Config::default(),
        display,
        canonical,
        callback_names,
        known_enum_types,
        known_records,
    ) {
        let _ = ty;
        return false;
    }

    [display, canonical]
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .any(is_raw_unsafe_by_value_object_candidate)
}

fn is_raw_unsafe_by_value_object_candidate(cpp_type: &str) -> bool {
    let trimmed = cpp_type.trim();
    if trimmed.is_empty() || trimmed == "void" || trimmed.ends_with('&') || trimmed.ends_with('*') {
        return false;
    }

    let base = base_model_cpp_type(trimmed);
    !base.is_empty()
        && !base.contains('<')
        && !base.starts_with("std::")
        && !is_supported_primitive(&base)
}

fn format_type_for_reason(display: &str, canonical: &str) -> String {
    if canonical.is_empty() || canonical == display {
        display.to_string()
    } else {
        format!("{display} ({canonical})")
    }
}

fn is_unsupported_double_pointer_type(cpp_type: &str) -> bool {
    let trimmed = cpp_type.trim();
    if trimmed.is_empty() || parse_array_type(trimmed).is_some() {
        return false;
    }
    trailing_pointer_depth(trimmed) >= 2
}

fn trailing_pointer_depth(value: &str) -> usize {
    let mut depth = 0;
    for ch in value.trim().chars().rev() {
        if ch == '*' {
            depth += 1;
        } else if ch.is_whitespace() {
            continue;
        } else {
            break;
        }
    }
    depth
}

fn resolved_enum_cpp_type(
    display: &str,
    canonical: &str,
    known_enum_types: &BTreeSet<String>,
) -> Option<String> {
    if raw_type_shape(display) != "value" {
        return None;
    }

    if known_enum_type_name(display, known_enum_types).is_some() {
        return Some(display.trim().to_string());
    }

    if known_enum_type_name(canonical, known_enum_types).is_some() {
        return Some(canonical.trim().to_string());
    }

    None
}

fn known_enum_type_name(value: &str, known_enum_types: &BTreeSet<String>) -> Option<String> {
    let base = enum_base_cpp_type(value);
    known_enum_types.iter().find_map(|candidate| {
        let normalized = enum_base_cpp_type(candidate);
        (normalized == base
            || normalized.rsplit("::").next().unwrap_or(&normalized) == base
            || base.rsplit("::").next().unwrap_or(&base) == normalized)
            .then(|| candidate.clone())
    })
}

fn enum_base_cpp_type(value: &str) -> String {
    let base = base_model_cpp_type(value);
    base.strip_prefix("enum ")
        .unwrap_or(&base)
        .trim()
        .to_string()
}

fn enum_value_type(cpp_type: &str) -> IrType {
    IrType {
        kind: IrTypeKind::Enum,
        cpp_type: cpp_type.to_string(),
        c_type: "int64_t".to_string(),
        handle: None,
    }
}

fn normalize_type_with_canonical(
    _config: &Config,
    cpp_type: &str,
    canonical_type: &str,
    callback_names: &BTreeSet<String>,
    known_enum_types: &BTreeSet<String>,
    known_records: &[IrRecord],
) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    let canonical_trimmed = canonical_type.trim();
    if is_unsupported_double_pointer_type(trimmed)
        || (!canonical_trimmed.is_empty() && is_unsupported_double_pointer_type(canonical_trimmed))
    {
        bail!(
            "unsupported double-pointer type in v1: {}",
            format_type_for_reason(trimmed, canonical_trimmed)
        );
    }
    if let Some(enum_cpp_type) =
        resolved_enum_cpp_type(trimmed, canonical_trimmed, known_enum_types)
    {
        return Ok(enum_value_type(&enum_cpp_type));
    }
    if let Some(ty) = normalize_known_record_type(trimmed, known_records) {
        return Ok(ty);
    }
    if canonical_trimmed != trimmed
        && raw_type_shape(trimmed) == raw_type_shape(canonical_trimmed)
        && let Some(mut ty) = normalize_known_record_type(canonical_trimmed, known_records)
    {
        ty.cpp_type =
            canonicalized_known_record_cpp_type(trimmed, &base_model_cpp_type(&ty.cpp_type));
        return Ok(ty);
    }
    if let Ok(ty) = normalize_type(trimmed, callback_names) {
        if matches!(
            ty.kind,
            IrTypeKind::ModelReference
                | IrTypeKind::ModelPointer
                | IrTypeKind::ModelValue
                | IrTypeKind::FixedModelArray
        ) && canonical_trimmed != trimmed
        {
            if let Ok(mut canonical_ty) = normalize_type(canonical_trimmed, callback_names) {
                if matches!(
                    canonical_ty.kind,
                    IrTypeKind::ExternStructReference
                        | IrTypeKind::ExternStructPointer
                        | IrTypeKind::Primitive
                        | IrTypeKind::Reference
                        | IrTypeKind::Pointer
                        | IrTypeKind::String
                        | IrTypeKind::CString
                        | IrTypeKind::FixedByteArray
                        | IrTypeKind::FixedArray
                ) {
                    canonical_ty.cpp_type = trimmed.to_string();
                    return Ok(canonical_ty);
                }
                // When both original and canonical resolve to a Model kind, use the
                // canonical type name for C++ code generation (e.g. `iKey_t` instead
                // of `iKey`) while keeping the original handle/c_type for the C API.
                if matches!(
                    canonical_ty.kind,
                    IrTypeKind::ModelValue | IrTypeKind::ModelReference | IrTypeKind::ModelPointer
                ) {
                    return Ok(IrType {
                        cpp_type: canonical_trimmed.to_string(),
                        ..ty
                    });
                }
            }
        }
        return Ok(ty);
    }

    if canonical_trimmed != trimmed {
        if raw_type_shape(trimmed) == raw_type_shape(canonical_trimmed)
            && let Ok(mut ty) = normalize_type(canonical_trimmed, callback_names)
        {
            ty.cpp_type = trimmed.to_string();
            return Ok(ty);
        }
        bail!("unsupported C++ type in v1: {trimmed} (canonical: {canonical_trimmed})");
    }

    bail!("unsupported C++ type in v1: {trimmed}");
}

fn normalize_known_record_type(cpp_type: &str, known_records: &[IrRecord]) -> Option<IrType> {
    let trimmed = cpp_type.trim();

    if let Some((elem, _)) = parse_array_type(trimmed) {
        let record = known_record(elem, known_records)?;
        return Some(IrType {
            kind: IrTypeKind::FixedModelArray,
            cpp_type: canonicalized_known_record_array_type(trimmed, &record.cpp_type),
            c_type: format!("{}**", record.handle_name),
            handle: Some(record.handle_name.clone()),
        });
    }

    let record = known_record(trimmed, known_records)?;
    let shape = raw_type_shape(trimmed);
    let cpp_type = canonicalized_known_record_cpp_type(trimmed, &record.cpp_type);
    let c_type = if is_const_qualified_model_type(trimmed) {
        format!("const {}*", record.handle_name)
    } else {
        format!("{}*", record.handle_name)
    };
    match shape {
        "pointer" => Some(IrType {
            kind: IrTypeKind::ModelPointer,
            cpp_type,
            c_type,
            handle: Some(record.handle_name.clone()),
        }),
        "reference" => Some(IrType {
            kind: IrTypeKind::ModelReference,
            cpp_type,
            c_type,
            handle: Some(record.handle_name.clone()),
        }),
        _ => Some(IrType {
            kind: IrTypeKind::ModelValue,
            cpp_type,
            c_type,
            handle: Some(record.handle_name.clone()),
        }),
    }
}

fn is_const_qualified_model_type(value: &str) -> bool {
    let trimmed = value.trim();
    let base = trimmed.trim_end_matches('&').trim_end_matches('*').trim();
    base.starts_with("const ") || base.ends_with(" const")
}

fn known_record<'a>(value: &str, known_records: &'a [IrRecord]) -> Option<&'a IrRecord> {
    let base = record_base_cpp_type(value);
    known_records.iter().find(|record| {
        let normalized = record_base_cpp_type(&record.cpp_type);
        normalized == base
            || normalized.rsplit("::").next().unwrap_or(&normalized) == base
            || base.rsplit("::").next().unwrap_or(&base) == normalized
    })
}

fn canonicalized_known_record_cpp_type(original: &str, record_cpp_type: &str) -> String {
    let is_const = original.trim().starts_with("const ");
    let base = if is_const {
        format!("const {record_cpp_type}")
    } else {
        record_cpp_type.to_string()
    };
    match raw_type_shape(original) {
        "pointer" => format!("{base}*"),
        "reference" => format!("{base}&"),
        _ => base,
    }
}

fn canonicalized_known_record_array_type(original: &str, record_cpp_type: &str) -> String {
    let len = fixed_array_length(original).unwrap_or(0);
    format!("{record_cpp_type}[{len}]")
}

fn raw_type_shape(cpp_type: &str) -> &'static str {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    if trimmed.ends_with('*') {
        "pointer"
    } else if trimmed.ends_with('&') {
        "reference"
    } else {
        "value"
    }
}

fn normalize_type(cpp_type: &str, callback_names: &BTreeSet<String>) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    if is_unsupported_double_pointer_type(trimmed) {
        bail!("unsupported double-pointer type in v1: {trimmed}");
    }
    if callback_names.contains(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::Callback,
            cpp_type: trimmed.to_string(),
            c_type: trimmed.to_string(),
            handle: None,
        });
    }

    // Strip leading "const " for value types and retry.
    // Only applies to non-pointer/reference types to preserve const semantics
    // on pointer targets (e.g. "const char*" is handled separately above).
    if let Some(stripped) = trimmed.strip_prefix("const ") {
        let stripped = stripped.trim();
        if !stripped.ends_with('*')
            && !stripped.ends_with('&')
            && let Ok(ty) = normalize_type(stripped, callback_names)
        {
            return Ok(IrType {
                cpp_type: trimmed.to_string(),
                c_type: ty.c_type.clone(),
                ..ty
            });
        }
    }

    if is_char_array_type(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        });
    }

    if is_unsigned_char_array_type(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::FixedByteArray,
            cpp_type: trimmed.to_string(),
            c_type: "uint8_t*".to_string(),
            handle: None,
        });
    }

    if let Some((elem, _)) = parse_array_type(trimmed) {
        if is_supported_primitive(elem) {
            let c_elem = canonical_primitive_c_type(elem);
            return Ok(IrType {
                kind: IrTypeKind::FixedArray,
                cpp_type: trimmed.to_string(),
                c_type: format!("{c_elem}*"),
                handle: None,
            });
        }
        if let Some(handle_name) = raw_safe_model_handle_name(elem) {
            return Ok(IrType {
                kind: IrTypeKind::FixedModelArray,
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}**"),
                handle: Some(handle_name),
            });
        }
    }

    match trimmed {
        "void" => Ok(primitive_type(trimmed)),
        "bool" | "int" | "short" | "long" | "long long" | "float" | "double" | "size_t"
        | "char" | "const char" | "unsigned" | "unsigned int" | "unsigned short"
        | "unsigned long" | "unsigned long long" | "signed char" | "unsigned char" => {
            Ok(primitive_type(trimmed))
        }
        "uint8" => Ok(alias_primitive_type(trimmed, "uint8_t")),
        "uint16" => Ok(alias_primitive_type(trimmed, "uint16_t")),
        "uint32" => Ok(alias_primitive_type(trimmed, "uint32_t")),
        "uint64" => Ok(alias_primitive_type(trimmed, "uint64_t")),
        "int8" => Ok(alias_primitive_type(trimmed, "int8_t")),
        "int16" => Ok(alias_primitive_type(trimmed, "int16_t")),
        "int32" => Ok(alias_primitive_type(trimmed, "int32_t")),
        "int64" => Ok(alias_primitive_type(trimmed, "int64_t")),
        "const char *" | "const char*" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "char *" | "char*" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
            handle: None,
        }),
        "NPCSTR" | "NPSTRC" | "NPCSTRC" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "NPSTR" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
            handle: None,
        }),
        "NPVOID" | "void *" | "void*" => Ok(IrType {
            kind: IrTypeKind::ModelPointer,
            cpp_type: "void".to_string(),
            c_type: "NPVOIDHandle*".to_string(),
            handle: Some("NPVOIDHandle".to_string()),
        }),
        "std::string" | "const std::string &" | "const std::string&" | "std::string_view" => {
            Ok(IrType {
                kind: IrTypeKind::String,
                cpp_type: trimmed.to_string(),
                c_type: "char*".to_string(),
                handle: None,
            })
        }
        _ if trimmed.ends_with('*')
            && is_supported_primitive(trimmed.trim_end_matches('*').trim()) =>
        {
            let base = trimmed.trim_end_matches('*').trim();
            Ok(IrType {
                kind: IrTypeKind::Pointer,
                cpp_type: trimmed.to_string(),
                c_type: format!("{}*", canonical_primitive_c_type(base)),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&')
            && is_supported_primitive(trimmed.trim_end_matches('&').trim()) =>
        {
            let base = trimmed.trim_end_matches('&').trim();
            Ok(IrType {
                kind: IrTypeKind::Reference,
                cpp_type: trimmed.to_string(),
                c_type: format!("{}*", canonical_primitive_c_type(base)),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&') && extern_c_struct_base_type(trimmed).is_some() => {
            let base = extern_c_struct_base_type(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ExternStructReference,
                cpp_type: trimmed.to_string(),
                c_type: format!("{base}*"),
                handle: None,
            })
        }
        _ if trimmed.ends_with('*') && extern_c_struct_base_type(trimmed).is_some() => {
            let base = extern_c_struct_base_type(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ExternStructPointer,
                cpp_type: trimmed.to_string(),
                c_type: format!("{base}*"),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelReference,
                cpp_type: trimmed.to_string(),
                c_type: if is_const_qualified_model_type(trimmed) {
                    format!("const {handle_name}*")
                } else {
                    format!("{handle_name}*")
                },
                handle: Some(handle_name),
            })
        }
        _ if trimmed.ends_with('*') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelPointer,
                cpp_type: trimmed.to_string(),
                c_type: if is_const_qualified_model_type(trimmed) {
                    format!("const {handle_name}*")
                } else {
                    format!("{handle_name}*")
                },
                handle: Some(handle_name),
            })
        }
        _ if raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelValue,
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name),
            })
        }
        _ => bail!("unsupported C++ type in v1: {trimmed}"),
    }
}

fn primitive_type(name: &str) -> IrType {
    IrType {
        kind: if name == "void" {
            IrTypeKind::Void
        } else {
            IrTypeKind::Primitive
        },
        cpp_type: name.to_string(),
        c_type: name.to_string(),
        handle: None,
    }
}

fn alias_primitive_type(cpp_name: &str, c_name: &str) -> IrType {
    IrType {
        kind: IrTypeKind::Primitive,
        cpp_type: cpp_name.to_string(),
        c_type: c_name.to_string(),
        handle: None,
    }
}

fn canonical_primitive_c_type(name: &str) -> &str {
    match name {
        "uint8" => "uint8_t",
        "uint16" => "uint16_t",
        "uint32" => "uint32_t",
        "uint64" => "uint64_t",
        "int8" => "int8_t",
        "int16" => "int16_t",
        "int32" => "int32_t",
        "int64" => "int64_t",
        other => other,
    }
}

fn callback_name_set(api: &ParsedApi) -> BTreeSet<String> {
    api.callbacks
        .iter()
        .flat_map(|callback| {
            let qualified = cpp_qualified(&callback.namespace, &callback.name);
            [callback.name.clone(), qualified]
        })
        .collect()
}

fn is_named_callback_param(param: &CppParam, callback_names: &BTreeSet<String>) -> bool {
    param
        .callback_typedef
        .as_deref()
        .is_some_and(|name| callback_names.contains(name))
}

fn is_operator_name(name: &str) -> bool {
    name.trim().starts_with("operator")
}

fn is_supported_primitive(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "int"
            | "int8_t"
            | "int8"
            | "int16_t"
            | "int16"
            | "int32_t"
            | "int32"
            | "int64_t"
            | "int64"
            | "short"
            | "long"
            | "long long"
            | "float"
            | "double"
            | "size_t"
            | "uint8_t"
            | "uint8"
            | "uint16_t"
            | "uint16"
            | "uint32_t"
            | "uint32"
            | "uint64_t"
            | "uint64"
            | "char"
            | "const char"
            | "unsigned"
            | "unsigned int"
            | "unsigned short"
            | "unsigned long"
            | "unsigned long long"
            | "signed char"
            | "unsigned char"
    )
}

fn symbol_name(_config: &Config, namespace: &[String], owner: &str, tail: &str) -> String {
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

fn sanitize_symbol_token(value: &str) -> String {
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

fn cpp_qualified(namespace: &[String], leaf: &str) -> String {
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

fn record_base_cpp_type(value: &str) -> String {
    let base = base_model_cpp_type(value);
    base.strip_prefix("struct ")
        .unwrap_or(&base)
        .trim()
        .to_string()
}

fn extern_c_struct_base_type(cpp_type: &str) -> Option<String> {
    let base = base_model_cpp_type(cpp_type);
    if let Some(tag) = base.strip_prefix("struct ") {
        return (!tag.trim().is_empty()).then(|| format!("struct {}", tag.trim()));
    }
    match base.as_str() {
        "timeval" => Some("struct timeval".to_string()),
        _ => None,
    }
}

fn raw_safe_model_handle_name(cpp_type: &str) -> Option<String> {
    let base = base_model_cpp_type(cpp_type);
    if base.is_empty()
        || base == "void"
        || base.contains('<')
        || base.starts_with("std::")
        || base.starts_with("struct ")
        || base.contains('[')
        || base.contains(']')
        || base.contains('(')
        || base.contains(')')
        || is_supported_primitive(&base)
    {
        return None;
    }

    Some(format!("{}Handle", flatten_qualified_cpp_name(&base)))
}

/// Extracts `(elem_type_str, size)` from a `T[N]` pattern after removing a
/// leading `const` prefix.
fn parse_array_type(value: &str) -> Option<(&str, usize)> {
    let trimmed = value.trim().trim_start_matches("const ").trim();
    let bracket = trimmed.rfind('[')?;
    let elem = trimmed[..bracket].trim();
    let rest = trimmed[bracket + 1..].strip_suffix(']')?;
    let n: usize = rest.trim().parse().ok()?;
    if elem.is_empty() || n == 0 {
        return None;
    }
    Some((elem, n))
}

/// Extracts the array length from `cpp_type` for fixed arrays.
pub fn fixed_array_length(cpp_type: &str) -> Option<usize> {
    parse_array_type(cpp_type).map(|(_, n)| n)
}

/// Extracts the element type string from `cpp_type` for fixed arrays.
pub fn fixed_array_elem_type(cpp_type: &str) -> Option<&str> {
    parse_array_type(cpp_type).map(|(t, _)| t)
}

fn is_char_array_type(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix("const ") {
        return is_char_array_type(inner);
    }

    let Some(prefix) = trimmed.strip_prefix("char[") else {
        return false;
    };
    let Some(length) = prefix.strip_suffix(']') else {
        return false;
    };
    !length.is_empty() && length.chars().all(|ch| ch.is_ascii_digit())
}

fn is_unsigned_char_array_type(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix("const ") {
        return is_unsigned_char_array_type(inner);
    }
    let Some(prefix) = trimmed.strip_prefix("unsigned char[") else {
        return false;
    };
    let Some(length) = prefix.strip_suffix(']') else {
        return false;
    };
    !length.is_empty() && length.chars().all(|ch| ch.is_ascii_digit())
}

pub fn byte_array_length(cpp_type: &str) -> Option<usize> {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    let prefix = trimmed.strip_prefix("unsigned char[")?;
    let len = prefix.strip_suffix(']')?;
    len.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::parser::{CppFunction, CppParam, ParsedApi};

    #[test]
    fn normalizes_struct_timeval_pointer_and_reference_as_external_structs() {
        let callback_names = BTreeSet::new();

        let pointer = normalize_type("struct timeval*", &callback_names).unwrap();
        assert_eq!(pointer.kind, IrTypeKind::ExternStructPointer);
        assert_eq!(pointer.c_type, "struct timeval*");
        assert_eq!(pointer.handle, None);

        let reference = normalize_type("struct timeval&", &callback_names).unwrap();
        assert_eq!(reference.kind, IrTypeKind::ExternStructReference);
        assert_eq!(reference.c_type, "struct timeval*");
        assert_eq!(reference.handle, None);
    }

    #[test]
    fn normalizes_timeval_alias_from_canonical_type() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type_with_canonical(
            &Config::default(),
            "timeval*",
            "struct timeval*",
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ExternStructPointer);
        assert_eq!(ty.cpp_type, "timeval*");
        assert_eq!(ty.c_type, "struct timeval*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_parsed_struct_pointer_as_model_handle_type() {
        let callback_names = BTreeSet::new();
        let known_records = vec![IrRecord {
            cpp_type: "Counter".to_string(),
            handle_name: "CounterHandle".to_string(),
            kind: RecordKind::Struct,
        }];
        let ty = normalize_type_with_canonical(
            &Config::default(),
            "struct Counter*",
            "Counter*",
            &callback_names,
            &BTreeSet::new(),
            &known_records,
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ModelPointer);
        assert_eq!(ty.cpp_type, "Counter*");
        assert_eq!(ty.c_type, "CounterHandle*");
        assert_eq!(ty.handle, Some("CounterHandle".to_string()));
    }

    #[test]
    fn rejects_by_value_type_when_only_canonical_form_is_pointer() {
        let callback_names = BTreeSet::new();
        let result = normalize_type_with_canonical(
            &Config::default(),
            "MTime",
            "MTime*",
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();

        assert_eq!(result.kind, IrTypeKind::ModelValue);
        assert_eq!(result.c_type, "MTimeHandle*");
    }

    #[test]
    fn rejects_by_value_type_when_only_canonical_form_is_reference() {
        let callback_names = BTreeSet::new();
        let result = normalize_type_with_canonical(
            &Config::default(),
            "TD_IE_CALL",
            "TD_IE_CALL&",
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();

        assert_eq!(result.kind, IrTypeKind::ModelValue);
        assert_eq!(result.c_type, "TD_IE_CALLHandle*");
    }

    #[test]
    fn by_value_model_params_are_supported() {
        let callback_names = BTreeSet::new();
        assert!(!is_raw_unsafe_by_value_param_type(
            "MTime",
            "MTime",
            &callback_names,
            &BTreeSet::new(),
            &[],
        ));
    }

    #[test]
    fn preserves_model_pointer_returns_as_model_pointer() {
        let callback_names = BTreeSet::new();
        let ty = normalize_return_type_with_canonical(
            &Config::default(),
            "ThingModel*",
            "ThingModel*",
            &BTreeSet::new(),
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ModelPointer);
        assert_eq!(ty.c_type, "ThingModelHandle*");
    }

    #[test]
    fn collects_opaque_handles_for_model_value_returns() {
        let mut opaque_types = Vec::new();
        let functions = vec![IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("ApiHandle".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(true),
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::ModelValue,
                cpp_type: "ThingModel*".to_string(),
                c_type: "ThingModelHandle*".to_string(),
                handle: Some("ThingModelHandle".to_string()),
            },
            params: vec![IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: "const Api*".to_string(),
                    c_type: "const ApiHandle*".to_string(),
                    handle: Some("ApiHandle".to_string()),
                },
            }],
        }];

        collect_referenced_opaque_types(&mut opaque_types, &functions);

        assert_eq!(opaque_types.len(), 1);
        assert_eq!(opaque_types[0].name, "ThingModelHandle");
        assert_eq!(opaque_types[0].cpp_type, "ThingModel");
    }

    #[test]
    fn collects_unknown_return_only_handles_for_model_value_returns() {
        let mut opaque_types = Vec::new();
        let functions = vec![IrFunction {
            name: "cgowrap_SessionManager_GetSharedRegion".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "SessionManager::GetSharedRegion".to_string(),
            method_of: Some("SessionManagerHandle".to_string()),
            owner_cpp_type: Some("SessionManager".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::ModelValue,
                cpp_type: "SharedRegion*".to_string(),
                c_type: "SharedRegionHandle*".to_string(),
                handle: Some("SharedRegionHandle".to_string()),
            },
            params: vec![IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: "SessionManager*".to_string(),
                    c_type: "SessionManagerHandle*".to_string(),
                    handle: Some("SessionManagerHandle".to_string()),
                },
            }],
        }];

        collect_referenced_opaque_types(&mut opaque_types, &functions);

        assert_eq!(opaque_types.len(), 1);
        assert_eq!(opaque_types[0].name, "SharedRegionHandle");
        assert_eq!(opaque_types[0].cpp_type, "SharedRegion");
    }

    #[test]
    fn keeps_abstract_model_pointer_returns_as_model_pointer() {
        let callback_names = BTreeSet::new();
        let abstract_types = BTreeSet::from([String::from("DBHandler")]);
        let ty = normalize_return_type_with_canonical(
            &Config::default(),
            "DBHandler*",
            "DBHandler*",
            &abstract_types,
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ModelPointer);
        assert_eq!(ty.c_type, "DBHandlerHandle*");
    }

    #[test]
    fn normalizes_char_array_as_c_string() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("char[33]", &callback_names).unwrap();

        assert_eq!(ty.kind, IrTypeKind::CString);
        assert_eq!(ty.cpp_type, "char[33]");
        assert_eq!(ty.c_type, "const char*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn array_types_are_not_promoted_to_model_handles() {
        assert_eq!(raw_safe_model_handle_name("char[33]"), None);
        assert_eq!(raw_safe_model_handle_name("uint32[8]"), None);
    }

    #[test]
    fn normalizes_unsigned_char_array_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("unsigned char[16]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "unsigned char[16]");
        assert_eq!(ty.c_type, "uint8_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_const_unsigned_char_array_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("const unsigned char[32]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "const unsigned char[32]");
        assert_eq!(ty.c_type, "uint8_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_uuid_t_alias_via_canonical_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type_with_canonical(
            &Config::default(),
            "uuid_t",
            "unsigned char[16]",
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "uuid_t");
        assert_eq!(ty.c_type, "uint8_t*");
    }

    #[test]
    fn byte_array_length_extracts_size() {
        assert_eq!(byte_array_length("unsigned char[16]"), Some(16));
        assert_eq!(byte_array_length("const unsigned char[32]"), Some(32));
        assert_eq!(byte_array_length("unsigned char[1]"), Some(1));
        assert_eq!(byte_array_length("char[16]"), None);
        assert_eq!(byte_array_length("unsigned char*"), None);
        assert_eq!(byte_array_length("unsigned char"), None);
    }

    #[test]
    fn serializes_kind_enums_with_legacy_string_values() {
        let ty = IrType {
            kind: IrTypeKind::ModelValue,
            cpp_type: "ThingModel".to_string(),
            c_type: "ThingModelHandle*".to_string(),
            handle: Some("ThingModelHandle".to_string()),
        };
        let function = IrFunction {
            name: "cgowrap_ThingModel_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "ThingModel".to_string(),
            method_of: None,
            owner_cpp_type: Some("ThingModel".to_string()),
            is_const: None,
            field_accessor: None,
            returns: ty.clone(),
            params: vec![],
        };

        let serialized_ty = serde_yaml::to_string(&ty).unwrap();
        let serialized_function = serde_yaml::to_string(&function).unwrap();
        let serialized_record_kind = serde_yaml::to_string(&RecordKind::Struct).unwrap();

        assert!(serialized_ty.contains("kind: model_value"));
        assert!(serialized_function.contains("kind: constructor"));
        assert_eq!(serialized_record_kind.trim(), "struct");
    }

    #[test]
    fn normalizes_int_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("int[4]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "int[4]");
        assert_eq!(ty.c_type, "int*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_bool_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("bool[8]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "bool[8]");
        assert_eq!(ty.c_type, "bool*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_float_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("float[3]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "float[3]");
        assert_eq!(ty.c_type, "float*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_uint32_t_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("uint32_t[2]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "uint32_t[2]");
        assert_eq!(ty.c_type, "uint32_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn rejects_model_double_pointer_type() {
        let callback_names = BTreeSet::new();
        let err = normalize_type("ThingModel**", &callback_names).unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported double-pointer type in v1: ThingModel**")
        );
    }

    #[test]
    fn rejects_char_double_pointer_type() {
        let callback_names = BTreeSet::new();
        let err = normalize_type("char **", &callback_names).unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported double-pointer type in v1: char **")
        );
    }

    #[test]
    fn rejects_const_model_double_pointer_with_canonical_type() {
        let callback_names = BTreeSet::new();
        let err = normalize_type_with_canonical(
            &Config::default(),
            "const ThingModel **",
            "ThingModel const **",
            &callback_names,
            &BTreeSet::new(),
            &[],
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported double-pointer type in v1")
        );
        assert!(err.to_string().contains("const ThingModel **"));
    }

    #[test]
    fn normalizes_model_array_as_fixed_model_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("FooModel[3]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedModelArray);
        assert_eq!(ty.cpp_type, "FooModel[3]");
        assert_eq!(ty.c_type, "FooModelHandle**");
        assert_eq!(ty.handle, Some("FooModelHandle".to_string()));
    }

    #[test]
    fn fixed_array_length_extracts_size() {
        assert_eq!(fixed_array_length("int[4]"), Some(4));
        assert_eq!(fixed_array_length("bool[8]"), Some(8));
        assert_eq!(fixed_array_length("float[3]"), Some(3));
        assert_eq!(fixed_array_length("FooModel[3]"), Some(3));
        assert_eq!(fixed_array_length("int"), None);
        assert_eq!(fixed_array_length("int*"), None);
    }

    #[test]
    fn fixed_array_elem_type_extracts_elem() {
        assert_eq!(fixed_array_elem_type("int[4]"), Some("int"));
        assert_eq!(fixed_array_elem_type("bool[8]"), Some("bool"));
        assert_eq!(fixed_array_elem_type("float[3]"), Some("float"));
        assert_eq!(fixed_array_elem_type("int"), None);
    }

    #[test]
    fn collects_public_aliases_for_underscore_backed_models() {
        let api = ParsedApi {
            headers: vec![],
            records: vec![],
            enums: vec![],
            macros: vec![],
            callbacks: vec![],
            functions: vec![CppFunction {
                source_header: PathBuf::from("DcsHistory.h"),
                namespace: vec![],
                name: "GetLowData".to_string(),
                return_type: "DCSHISTORY*".to_string(),
                return_canonical_type: "_DCSHISTORY*".to_string(),
                return_is_function_pointer: false,
                params: vec![CppParam {
                    name: "item".to_string(),
                    ty: "DCS_HIST_ITEM*".to_string(),
                    canonical_ty: "_DCS_HIST_ITEM*".to_string(),
                    is_function_pointer: false,
                    callback_typedef: None,
                    has_default: false,
                }],
            }],
        };

        let aliases = collect_preferred_model_aliases(&api);
        assert_eq!(aliases.get("_DCSHISTORY"), Some(&"DCSHISTORY".to_string()));
        assert_eq!(
            aliases.get("_DCS_HIST_ITEM"),
            Some(&"DCS_HIST_ITEM".to_string())
        );
    }

    #[test]
    fn stable_class_handle_name_prefers_public_alias_when_available() {
        let aliases = BTreeMap::from([("_DCSHISTORY".to_string(), "DCSHISTORY".to_string())]);

        assert_eq!(
            stable_class_handle_name("_DCSHISTORY", &aliases),
            "DCSHISTORYHandle"
        );
        assert_eq!(
            stable_class_handle_name("_DCS_HIST_ITEM", &aliases),
            "_DCS_HIST_ITEMHandle"
        );
    }
}
