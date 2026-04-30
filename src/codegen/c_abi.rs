use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    analysis::model_analysis,
    codegen::{go_facade as facade, ir_norm as ir},
    config::WRAPPER_PREFIX,
    domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind},
    ir::{IrCallback, IrFunction, IrModule, IrParam, IrType},
    parsing::{compiler, parser},
    pipeline::context::PipelineContext,
};

#[derive(Debug, Clone)]
struct SyntheticOpaqueDelete {
    handle_name: String,
    cpp_type: String,
    symbol_name: String,
}

pub fn generate_all(ctx: &PipelineContext, write_ir: bool) -> Result<()> {
    let (ctx, parsed) = prepare_with_parsed(&ctx)?;
    let generation_headers = generation_headers(&ctx);

    if generation_headers.len() > 1 && !ctx.uses_default_output_names() {
        bail!(
            "multi-header generation does not support explicit output.header/source/ir overrides; leave them as defaults to emit one wrapper set per header"
        );
    }

    if generation_headers.len() <= 1 {
        let scoped = generation_headers
            .first()
            .cloned()
            .map(|header| ctx.scoped_to_header(header))
            .unwrap_or_else(|| ctx.clone());
        let header_api = scoped
            .target_header
            .as_deref()
            .map(|header| parsed.filter_to_header(header))
            .unwrap_or_else(|| parsed.clone());
        let normalized_ir = ir::normalize(&scoped, &header_api)?;
        let class_handles = class_handles_with_methods(&normalized_ir);
        return generate(&scoped, &normalized_ir, write_ir, &class_handles);
    }

    // Pass 1: normalize all headers up front so we can do global deduplication in pass 2.
    let mut all_normalized: Vec<(PipelineContext, IrModule)> = Vec::new();
    for header in &generation_headers {
        let scoped = ctx.scoped_to_header(header.clone());
        let header_api = parsed.filter_to_header(header);
        if header_api.is_empty() {
            continue;
        }
        let normalized_ir = ir::normalize(&scoped, &header_api)?;
        all_normalized.push((scoped, normalized_ir));
    }

    // Compute the set of handles for classes that will get primary Go wrapper structs.
    // These are classes with at least one method AND a destructor in any normalized IR.
    // Using the normalized IR (instead of parsed.records.has_destructor) correctly
    // includes classes with implicit C++ destructors (has_destructor=false in parser).
    let global_class_handles: BTreeSet<String> = all_normalized
        .iter()
        .flat_map(|(_, ir)| class_handles_with_methods(ir))
        .collect();
    let global_owned_opaque_value_handles = all_normalized
        .iter()
        .flat_map(|(scoped, ir)| {
            opaque_model_value_handles_needing_go_ownership(scoped, ir, &global_class_handles)
        })
        .collect::<BTreeSet<_>>();
    let mut owned_opaque_owner = BTreeSet::<String>::new();

    // Pass 2: generate each file, tracking every opaque handle that has already been
    // emitted so that non-class opaque types shared across headers are declared only once.
    let mut globally_emitted_opaques = global_class_handles.clone();
    globally_emitted_opaques.extend(global_owned_opaque_value_handles.iter().cloned());
    for (scoped, normalized_ir) in &all_normalized {
        let local_owned_opaque_value_handles = opaque_model_value_handles_needing_go_ownership(
            scoped,
            normalized_ir,
            &global_class_handles,
        )
        .into_iter()
        .filter(|handle| owned_opaque_owner.insert(handle.clone()))
        .collect::<BTreeSet<_>>();
        generate_with_opaque_ownership(
            scoped,
            normalized_ir,
            write_ir,
            &global_class_handles,
            &globally_emitted_opaques,
            &global_owned_opaque_value_handles,
            &local_owned_opaque_value_handles,
        )?;
        for ot in &normalized_ir.opaque_types {
            globally_emitted_opaques.insert(ot.name.clone());
        }
    }

    Ok(())
}

/// Returns the set of C handle names for classes that will generate primary Go wrapper structs.
/// A class gets a wrapper if it has at least one method AND a synthesized destructor in the
/// normalized IR. Using the IR (not parser's has_destructor) correctly covers classes with
/// implicit C++ destructors.
fn class_handles_with_methods(ir: &IrModule) -> BTreeSet<String> {
    let handles_with_methods: BTreeSet<&str> = ir
        .functions
        .iter()
        .filter(|f| f.kind == IrFunctionKind::Method)
        .filter_map(|f| f.method_of.as_deref())
        .collect();
    ir.functions
        .iter()
        .filter(|f| f.kind == IrFunctionKind::Destructor)
        .filter_map(|f| f.method_of.as_deref())
        .filter(|h| handles_with_methods.contains(*h))
        .map(|s| s.to_string())
        .collect()
}

fn generation_headers(ctx: &PipelineContext) -> Vec<PathBuf> {
    ctx.input
        .dir
        .as_ref()
        .and_then(|dir| scan_generation_headers(dir).ok())
        .unwrap_or_default()
}

fn scan_generation_headers(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut headers = BTreeSet::new();
    scan_generation_headers_recursive(dir, &mut headers)?;
    Ok(headers.into_iter().collect())
}

fn scan_generation_headers_recursive(dir: &Path, headers: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read generation directory: {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            scan_generation_headers_recursive(&path, headers)?;
            continue;
        }
        if path.is_file()
            && matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("h" | "hh" | "hpp" | "hxx")
            )
        {
            headers.insert(path.canonicalize().unwrap_or(path));
        }
    }
    Ok(())
}

pub fn prepare_context(ctx: &PipelineContext) -> Result<PipelineContext> {
    Ok(prepare_with_parsed(ctx)?.0)
}

pub fn prepare_config(ctx: &PipelineContext) -> Result<PipelineContext> {
    prepare_context(ctx)
}

pub fn prepare_with_parsed(ctx: &PipelineContext) -> Result<(PipelineContext, parser::ParsedApi)> {
    let parsed = parser::parse(ctx)?;
    let ctx = build_pipeline_context(&ctx, &parsed)?;
    Ok((ctx, parsed))
}

fn build_pipeline_context(
    ctx: &PipelineContext,
    parsed: &parser::ParsedApi,
) -> Result<PipelineContext> {
    let known_model_types = collect_known_model_types(parsed);
    let known_enum_types = collect_known_enum_types(parsed);
    let preferred_model_aliases = ir::collect_preferred_model_aliases(parsed);
    let scoped = ctx
        .clone()
        .with_known_model_types(known_model_types)
        .with_known_enum_types(known_enum_types)
        .with_preferred_model_aliases(preferred_model_aliases);
    let ir = ir::normalize(&scoped, parsed)?;
    let known_model_projections = model_analysis::collect_known_model_projections(&scoped, &ir)?;
    Ok(scoped.with_known_model_projections(known_model_projections))
}

fn collect_known_model_types(parsed: &parser::ParsedApi) -> Vec<String> {
    parsed
        .records
        .iter()
        .map(|record| {
            if record.namespace.is_empty() {
                record.name.clone()
            } else {
                format!("{}::{}", record.namespace.join("::"), record.name)
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_known_enum_types(parsed: &parser::ParsedApi) -> Vec<String> {
    parsed
        .enums
        .iter()
        .filter(|item| !item.is_anonymous)
        .map(|item| {
            if item.namespace.is_empty() {
                item.name.clone()
            } else {
                format!("{}::{}", item.namespace.join("::"), item.name)
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn generate(
    ctx: &PipelineContext,
    ir: &IrModule,
    write_ir: bool,
    global_class_handles: &BTreeSet<String>,
) -> Result<()> {
    let local_owned_opaque_value_handles =
        opaque_model_value_handles_needing_go_ownership(ctx, ir, global_class_handles);
    generate_with_opaque_ownership(
        ctx,
        ir,
        write_ir,
        global_class_handles,
        global_class_handles,
        &local_owned_opaque_value_handles,
        &local_owned_opaque_value_handles,
    )
}

fn generate_with_opaque_ownership(
    ctx: &PipelineContext,
    ir: &IrModule,
    write_ir: bool,
    native_covered_handles: &BTreeSet<String>,
    globally_emitted_opaques: &BTreeSet<String>,
    global_owned_opaque_value_handles: &BTreeSet<String>,
    local_owned_opaque_value_handles: &BTreeSet<String>,
) -> Result<()> {
    fs::create_dir_all(ctx.output_dir()).with_context(|| {
        format!(
            "failed to create output dir: {}",
            ctx.output_dir().display()
        )
    })?;

    let header_path = ctx.output_dir().join(&ctx.output.header);
    let source_path = ctx.output_dir().join(&ctx.output.source);
    let ir_path = ctx.output_dir().join(&ctx.output.ir);
    fs::write(
        &header_path,
        trim_trailing_blank_lines(render_header_with_owned_opaque_handles(
            &ctx,
            ir,
            native_covered_handles,
            local_owned_opaque_value_handles,
        )),
    )
    .with_context(|| format!("failed to write header: {}", header_path.display()))?;
    fs::write(
        &source_path,
        trim_trailing_blank_lines(render_source_with_owned_opaque_handles(
            &ctx,
            ir,
            native_covered_handles,
            local_owned_opaque_value_handles,
        )),
    )
    .with_context(|| format!("failed to write source: {}", source_path.display()))?;
    for go_file in facade::render_go_facade_with_owned_opaques(
        &ctx,
        ir,
        globally_emitted_opaques,
        global_owned_opaque_value_handles,
        local_owned_opaque_value_handles,
    )? {
        fs::create_dir_all(ctx.output_dir()).with_context(|| {
            format!(
                "failed to create go output dir: {}",
                ctx.output_dir().display()
            )
        })?;
        let go_path = ctx.output_dir().join(&go_file.filename);
        fs::write(&go_path, trim_trailing_blank_lines(go_file.contents))
            .with_context(|| format!("failed to write Go wrapper: {}", go_path.display()))?;
    }
    write_go_package_metadata(&ctx)?;
    if write_ir {
        let serialized = serde_yaml::to_string(ir)?;
        fs::write(&ir_path, serialized)
            .with_context(|| format!("failed to write ir dump: {}", ir_path.display()))?;
    }
    Ok(())
}

fn trim_trailing_blank_lines(mut contents: String) -> String {
    while contents.ends_with("\n\n") {
        contents.pop();
    }
    contents
}

pub fn write_ir(path: &Path, ir: &IrModule) -> Result<()> {
    let serialized = serde_yaml::to_string(ir)?;
    fs::write(path, serialized)
        .with_context(|| format!("failed to write ir dump: {}", path.display()))?;
    Ok(())
}

fn write_go_package_metadata(ctx: &PipelineContext) -> Result<()> {
    let Some(go_module) = ctx.go_module.as_deref() else {
        return Ok(());
    };

    let go_mod_path = ctx.output_dir().join("go.mod");
    fs::write(
        &go_mod_path,
        render_go_mod(go_module, &ctx.output.go_version),
    )
    .with_context(|| format!("failed to write go.mod: {}", go_mod_path.display()))?;

    let build_flags_path = ctx.output_dir().join("build_flags.go");
    fs::write(&build_flags_path, render_build_flags(ctx)).with_context(|| {
        format!(
            "failed to write build_flags.go: {}",
            build_flags_path.display()
        )
    })?;

    Ok(())
}

fn render_go_mod(go_module: &str, go_version: &str) -> String {
    format!("module {go_module}\n\ngo {go_version}\n")
}

fn render_build_flags(ctx: &PipelineContext) -> String {
    let package_name = go_package_name(&ctx.output.dir);
    let cxxflags = exported_cxxflags(ctx);
    let cxxflags_line = cxxflags.join(" ");
    let ldflags = &ctx.input.ldflags;
    if ldflags.is_empty() {
        format!(
            "package {package_name}\n\n/*\n#cgo CFLAGS: -I${{SRCDIR}}\n#cgo CXXFLAGS: {cxxflags_line}\n*/\nimport \"C\"\n"
        )
    } else {
        let ldflags_line = ldflags.join(" ");
        format!(
            "package {package_name}\n\n/*\n#cgo CFLAGS: -I${{SRCDIR}}\n#cgo CXXFLAGS: {cxxflags_line}\n#cgo LDFLAGS: {ldflags_line}\n*/\nimport \"C\"\n"
        )
    }
}

fn exported_cxxflags(ctx: &PipelineContext) -> Vec<String> {
    let mut flags = vec!["-I${SRCDIR}".to_string()];
    let mut index = 0;
    let raw = &ctx.input.clang_args;

    while index < raw.len() {
        let arg = &raw[index];

        if arg == "-I" || arg == "-D" {
            if let Some(value) = raw.get(index + 1) {
                flags.push(arg.clone());
                flags.push(value.clone());
            }
            index += 2;
            continue;
        }

        if arg == "-isystem" {
            index += 2;
            continue;
        }

        if (arg.starts_with("-I") && arg.len() > 2)
            || (arg.starts_with("-D") && arg.len() > 2)
            || arg.starts_with("-std=")
        {
            flags.push(arg.clone());
        }

        index += 1;
    }

    flags
}

fn go_package_name(path: &Path) -> String {
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

pub fn render_header(ctx: &PipelineContext, ir: &IrModule) -> String {
    render_header_with_covered_handles(ctx, ir, &BTreeSet::new())
}

fn render_header_with_covered_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
) -> String {
    render_header_with_optional_owned_opaque_handles(ctx, ir, covered_handles, None)
}

fn render_header_with_owned_opaque_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    render_header_with_optional_owned_opaque_handles(
        ctx,
        ir,
        covered_handles,
        Some(owned_opaque_value_handles),
    )
}

fn render_header_with_optional_owned_opaque_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: Option<&BTreeSet<String>>,
) -> String {
    let guard = format!(
        "{}_{}",
        WRAPPER_PREFIX.to_uppercase(),
        ctx.output.header.replace('.', "_").to_uppercase()
    );
    let mut out = String::new();
    out.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));
    out.push_str(
        "#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdlib.h>\n\n",
    );
    if ir_uses_struct_timeval(ir) {
        out.push_str("#include <sys/time.h>\n\n");
    }
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

    for opaque in &ir.opaque_types {
        out.push_str(&format!(
            "typedef struct {} {};\n",
            opaque.name, opaque.name
        ));
    }
    if !ir.opaque_types.is_empty() {
        out.push('\n');
    }

    for callback in &ir.callbacks {
        render_callback_decl(&mut out, callback);
    }

    for function in &ir.functions {
        out.push_str(&render_function_decl(function));
        out.push('\n');
    }
    for delete in
        synthetic_opaque_model_value_deletes(ctx, ir, covered_handles, owned_opaque_value_handles)
    {
        out.push_str(&format!(
            "void {}({}* self);\n",
            delete.symbol_name, delete.handle_name
        ));
    }
    for function in callback_bridge_functions(ir) {
        out.push_str(&render_function_decl(&function));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == IrTypeKind::String)
    {
        out.push_str(&format!(
            "void {}_string_free(char* value);\n\n",
            WRAPPER_PREFIX
        ));
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == IrTypeKind::FixedByteArray)
    {
        out.push_str(&format!(
            "static inline void {}_byte_array_free(uint8_t* value) {{ free(value); }}\n\n",
            WRAPPER_PREFIX
        ));
    }

    let needs_array_free = ir.functions.iter().any(|f| {
        matches!(
            f.returns.kind,
            IrTypeKind::FixedArray | IrTypeKind::FixedModelArray
        )
    });
    if needs_array_free {
        out.push_str(&format!(
            "static inline void {}_array_free(void* value) {{ free(value); }}\n\n",
            WRAPPER_PREFIX
        ));
    }

    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    out.push_str(&format!("#endif /* {guard} */\n"));
    out
}

pub fn render_source(ctx: &PipelineContext, ir: &IrModule) -> String {
    render_source_with_covered_handles(ctx, ir, &BTreeSet::new())
}

fn render_source_with_covered_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
) -> String {
    render_source_with_optional_owned_opaque_handles(ctx, ir, covered_handles, None)
}

fn render_source_with_owned_opaque_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: &BTreeSet<String>,
) -> String {
    render_source_with_optional_owned_opaque_handles(
        ctx,
        ir,
        covered_handles,
        Some(owned_opaque_value_handles),
    )
}

fn render_source_with_optional_owned_opaque_handles(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: Option<&BTreeSet<String>>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("#include \"{}\"\n", ctx.output.header));
    out.push_str("#include <cstdlib>\n#include <cstring>\n#include <new>\n#include <string>\n\n");
    for include_name in source_include_prelude(ctx, ir) {
        out.push_str(&format!("#include \"{}\"\n", include_name));
    }
    out.push('\n');

    for function in &ir.functions {
        out.push_str(&render_function_def(function));
        out.push('\n');
    }
    for delete in
        synthetic_opaque_model_value_deletes(ctx, ir, covered_handles, owned_opaque_value_handles)
    {
        out.push_str(&render_synthetic_opaque_delete_def(&delete));
        out.push('\n');
    }
    let callback_map = callback_map(ir);
    let extern_c_block = render_go_callback_extern_c_block(&ir.functions, &callback_map);
    if !extern_c_block.is_empty() {
        out.push_str(&extern_c_block);
    }
    for function in ir.functions.iter().filter(|function| {
        function
            .params
            .iter()
            .any(|param| param.ty.kind == IrTypeKind::Callback)
    }) {
        out.push_str(&render_callback_bridge_def(function, &callback_map));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == IrTypeKind::String)
    {
        out.push_str(&render_string_free(&ctx));
    }

    // array_free helpers are defined as static inline in the header

    out
}

fn source_include_prelude(ctx: &PipelineContext, ir: &IrModule) -> Vec<String> {
    let mut ordered_paths = Vec::new();
    let mut emitted = BTreeSet::new();
    for header in &ir.source_headers {
        for include_path in translation_unit_support_include_paths(ctx, Path::new(header)) {
            let normalized = fs::canonicalize(&include_path).unwrap_or(include_path);
            if emitted.insert(normalized.clone()) {
                ordered_paths.push(normalized);
            }
        }
        for include_path in immediate_local_include_paths(Path::new(header)) {
            let normalized = fs::canonicalize(&include_path).unwrap_or(include_path);
            if emitted.insert(normalized.clone()) {
                ordered_paths.push(normalized);
            }
        }
        let normalized = fs::canonicalize(header).unwrap_or_else(|_| PathBuf::from(header));
        if emitted.insert(normalized.clone()) {
            ordered_paths.push(normalized);
        }
    }

    let mut include_names = Vec::new();
    let mut seen = BTreeSet::new();
    for path in ordered_paths {
        let include_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !include_name.is_empty() && seen.insert(include_name.to_string()) {
            include_names.push(include_name.to_string());
        }
    }

    include_names
}

fn translation_unit_support_include_paths(ctx: &PipelineContext, header: &Path) -> Vec<PathBuf> {
    let target = fs::canonicalize(header).unwrap_or_else(|_| header.to_path_buf());
    let mut support_paths = Vec::new();
    let mut seen = BTreeSet::new();

    for unit in compiler::collect_translation_units(&ctx.config)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| is_source_translation_unit(path))
    {
        let normalized = fs::canonicalize(&unit).unwrap_or(unit);
        let Ok(contents) = fs::read_to_string(&normalized) else {
            continue;
        };
        let Some(target_index) = leading_local_include_paths(&normalized, &contents)
            .iter()
            .position(|include_path| same_canonical_path(include_path, &target))
        else {
            continue;
        };

        for include_path in leading_local_include_paths(&normalized, &contents)
            .into_iter()
            .take(target_index)
        {
            let normalized = fs::canonicalize(&include_path).unwrap_or(include_path);
            if seen.insert(normalized.clone()) {
                support_paths.push(normalized);
            }
        }
    }

    support_paths
}

fn immediate_local_include_paths(path: &Path) -> Vec<PathBuf> {
    let normalized = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let Ok(contents) = fs::read_to_string(&normalized) else {
        return Vec::new();
    };
    leading_local_include_paths(&normalized, &contents)
}

fn local_quoted_includes(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.strip_prefix("#include")?.trim_start();
            let inner = rest.strip_prefix('"')?;
            let end = inner.find('"')?;
            Some(inner[..end].to_string())
        })
        .collect()
}

fn leading_local_include_paths(path: &Path, contents: &str) -> Vec<PathBuf> {
    let mut includes = Vec::new();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed == "#pragma once"
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("*/")
        {
            continue;
        }
        if let Some(include) = local_quoted_includes(trimmed).into_iter().next() {
            let include_path = parent.join(include);
            if include_path.exists() {
                includes.push(include_path);
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        break;
    }
    includes
}

fn is_source_translation_unit(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "cc" | "cpp" | "cxx")
    )
}

fn same_canonical_path(left: &Path, right: &Path) -> bool {
    fs::canonicalize(left)
        .map(|path| path == right)
        .unwrap_or_else(|_| left == right)
}

pub fn render_go_structs(ctx: &PipelineContext, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    facade::render_go_facade(ctx, ir, &BTreeSet::new())
}

fn render_callback_decl(out: &mut String, callback: &IrCallback) {
    let params = if callback.params.is_empty() {
        "void".to_string()
    } else {
        callback
            .params
            .iter()
            .map(|param| format!("{} {}", param.ty.c_type, param.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!(
        "typedef {} (*{})({});\n\n",
        callback.returns.c_type, callback.name, params
    ));
}

pub use crate::codegen::go_facade::GeneratedGoFile;

fn render_function_decl(function: &IrFunction) -> String {
    let params = render_param_list(function);
    format!("{} {}({});", function.returns.c_type, function.name, params)
}

fn render_param_list(function: &IrFunction) -> String {
    if function.params.is_empty() {
        return "void".to_string();
    }
    function
        .params
        .iter()
        .map(|param| format!("{} {}", param.ty.c_type, param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_function_def(function: &IrFunction) -> String {
    let signature = format!(
        "{} {}({})",
        function.returns.c_type,
        function.name,
        render_param_list(function)
    );
    let body = match function.kind {
        IrFunctionKind::Constructor => render_constructor_body(function),
        IrFunctionKind::Destructor => render_destructor_body(function),
        IrFunctionKind::Method => render_method_body(function),
        IrFunctionKind::Function => render_free_function_body(function),
    };
    format!("{signature} {{\n{body}}}\n")
}

fn render_callback_bridge_def(
    function: &IrFunction,
    callbacks: &std::collections::BTreeMap<String, IrCallback>,
) -> String {
    let bridge = make_callback_bridge_function(function);
    let signature = format!(
        "{} {}({})",
        bridge.returns.c_type,
        bridge.name,
        render_param_list(&bridge)
    );
    let body = render_callback_bridge_body(function, callbacks);
    format!("{signature} {{\n{body}}}\n")
}

fn render_constructor_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    let call_args = call_args(function, 0);
    format!(
        "    return reinterpret_cast<{}>(new {}({}));\n",
        function.returns.c_type, owner, call_args
    )
}

fn render_destructor_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    format!("    delete reinterpret_cast<{}*>(self);\n", owner)
}

fn render_method_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    if let Some(accessor) = &function.field_accessor {
        let receiver = if function.is_const.unwrap_or(false) {
            format!("reinterpret_cast<const {}*>(self)", owner)
        } else {
            format!("reinterpret_cast<{}*>(self)", owner)
        };
        return match accessor.access {
            FieldAccessKind::Get => {
                render_field_getter_body(function, &receiver, &accessor.field_name)
            }
            FieldAccessKind::Set => {
                render_field_setter_body(function, &receiver, &accessor.field_name)
            }
            FieldAccessKind::GetAt => {
                render_indexed_field_getter_body(function, accessor, &receiver)
            }
            FieldAccessKind::SetAt => {
                render_indexed_field_setter_body(function, accessor, &receiver)
            }
        };
    }
    let receiver = if function.is_const.unwrap_or(false) {
        format!("reinterpret_cast<const {}*>(self)", owner)
    } else {
        format!("reinterpret_cast<{}*>(self)", owner)
    };
    let method_name = function
        .cpp_name
        .rsplit("::")
        .next()
        .unwrap_or(&function.cpp_name);
    render_callable_body(function, &format!("{receiver}->{method_name}"), 1)
}

fn render_free_function_body(function: &IrFunction) -> String {
    render_callable_body(function, &function.cpp_name, 0)
}

fn render_callable_body(function: &IrFunction, target: &str, arg_start: usize) -> String {
    let args = call_args(function, arg_start);
    match function.returns.kind {
        IrTypeKind::Void => format!("    {}({});\n", target, args),
        IrTypeKind::Enum => format!(
            "    return static_cast<{}>({}({}));\n",
            function.returns.c_type, target, args
        ),
        IrTypeKind::String => format!(
            "    std::string result = {}({});\n    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));\n    if (buffer == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(buffer, result.c_str(), result.size() + 1);\n    return buffer;\n",
            target, args
        ),
        IrTypeKind::FixedByteArray => format!(
            "    auto _tmp = {target}({args});\n    uint8_t* _r = static_cast<uint8_t*>(std::malloc(sizeof(_tmp)));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(_r, &_tmp, sizeof(_tmp));\n    return _r;\n"
        ),
        IrTypeKind::FixedArray => {
            let c_elem = function.returns.c_type.trim_end_matches('*').trim();
            format!(
                "    auto _tmp = {target}({args});\n    {c_elem}* _r = static_cast<{c_elem}*>(std::malloc(sizeof(_tmp)));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(_r, &_tmp, sizeof(_tmp));\n    return _r;\n"
            )
        }
        IrTypeKind::FixedModelArray => {
            let handle = function.returns.handle.as_deref().unwrap_or("");
            let base_cpp = fixed_model_array_elem_cpp_type(&function.returns.cpp_type);
            let n = ir::fixed_array_length(&function.returns.cpp_type).unwrap_or(0);
            format!(
                "    auto _tmp = {target}({args});\n    {handle}** _r = static_cast<{handle}**>(std::malloc({n} * sizeof({handle}*)));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    for (int _i = 0; _i < {n}; _i++) {{\n        _r[_i] = reinterpret_cast<{handle}*>(new {base_cpp}(_tmp[_i]));\n    }}\n    return _r;\n"
            )
        }
        IrTypeKind::ModelReference => format!(
            "    return reinterpret_cast<{}>(&{}({}));\n",
            function.returns.c_type, target, args
        ),
        IrTypeKind::ModelValue => render_model_value_return(function, target, &args),
        _ if function.returns.handle.is_some() => format!(
            "    return reinterpret_cast<{}>({}({}));\n",
            function.returns.c_type, target, args
        ),
        _ => format!("    return {}({});\n", target, args),
    }
}

fn render_field_getter_body(function: &IrFunction, receiver: &str, field_name: &str) -> String {
    match function.returns.kind {
        IrTypeKind::Enum => format!(
            "    return static_cast<{}>({receiver}->{field_name});\n",
            function.returns.c_type
        ),
        IrTypeKind::FixedByteArray => format!(
            "    uint8_t* _r = static_cast<uint8_t*>(std::malloc(sizeof({receiver}->{field_name})));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(_r, {receiver}->{field_name}, sizeof({receiver}->{field_name}));\n    return _r;\n"
        ),
        IrTypeKind::FixedArray => {
            let c_elem = function.returns.c_type.trim_end_matches('*').trim();
            format!(
                "    {c_elem}* _r = static_cast<{c_elem}*>(std::malloc(sizeof({receiver}->{field_name})));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(_r, {receiver}->{field_name}, sizeof({receiver}->{field_name}));\n    return _r;\n"
            )
        }
        IrTypeKind::FixedModelArray => {
            let handle = function.returns.handle.as_deref().unwrap_or("");
            let n = ir::fixed_array_length(&function.returns.cpp_type).unwrap_or(0);
            format!(
                "    {handle}** _r = static_cast<{handle}**>(std::malloc({n} * sizeof({handle}*)));\n    if (_r == nullptr) {{\n        return nullptr;\n    }}\n    for (int _i = 0; _i < {n}; _i++) {{\n        _r[_i] = reinterpret_cast<{handle}*>(const_cast<{}*>(&{receiver}->{field_name}[_i]));\n    }}\n    return _r;\n",
                fixed_model_array_elem_cpp_type(&function.returns.cpp_type)
            )
        }
        IrTypeKind::ModelPointer | IrTypeKind::ModelReference => format!(
            "    return reinterpret_cast<{}>(&{receiver}->{field_name});\n",
            function.returns.c_type
        ),
        IrTypeKind::ModelValue => format!(
            "    return reinterpret_cast<{}>(new {}({receiver}->{field_name}));\n",
            function.returns.c_type,
            base_model_cpp_type(&function.returns.cpp_type),
        ),
        _ => format!("    return {receiver}->{};\n", field_name),
    }
}

fn render_indexed_field_getter_body(
    function: &IrFunction,
    accessor: &ir::IrFieldAccessor,
    receiver: &str,
) -> String {
    let index_name = function
        .params
        .get(1)
        .map(|param| param.name.as_str())
        .unwrap_or("index");
    let array_len = accessor.array_len.unwrap_or(0);
    let c_type = &function.returns.c_type;
    let base_cpp = base_model_cpp_type(&function.returns.cpp_type);
    let cast_expr = if c_type.starts_with("const ") {
        format!(
            "reinterpret_cast<{c_type}>(&{receiver}->{field}[{index_name}])",
            field = accessor.field_name
        )
    } else {
        format!(
            "reinterpret_cast<{c_type}>(const_cast<{base_cpp}*>(&{receiver}->{field}[{index_name}]))",
            field = accessor.field_name
        )
    };
    format!(
        "    if (self == nullptr || {index_name} < 0 || {index_name} >= {array_len}) {{\n        return nullptr;\n    }}\n    return {cast_expr};\n"
    )
}

fn render_indexed_field_setter_body(
    function: &IrFunction,
    accessor: &ir::IrFieldAccessor,
    receiver: &str,
) -> String {
    let index_name = function
        .params
        .get(1)
        .map(|param| param.name.as_str())
        .unwrap_or("index");
    let value_name = function
        .params
        .get(2)
        .map(|param| param.name.as_str())
        .unwrap_or("value");
    let array_len = accessor.array_len.unwrap_or(0);
    let base_cpp = function
        .params
        .get(2)
        .map(|param| base_model_cpp_type(&param.ty.cpp_type))
        .unwrap_or_else(|| "void".to_string());
    format!(
        "    if (self == nullptr || {value_name} == nullptr || {index_name} < 0 || {index_name} >= {array_len}) {{\n        return;\n    }}\n    {receiver}->{field_name}[{index_name}] = *reinterpret_cast<{base_cpp}*>({value_name});\n",
        field_name = accessor.field_name
    )
}

fn render_field_setter_body(function: &IrFunction, receiver: &str, field_name: &str) -> String {
    let Some(value_param) = function.params.get(1) else {
        return format!("    {receiver}->{} = value;\n", field_name);
    };
    match value_param.ty.kind {
        IrTypeKind::Enum => format!(
            "    {receiver}->{field_name} = static_cast<{}>(value);\n",
            value_param.ty.cpp_type
        ),
        IrTypeKind::CString => {
            let Some(length) = char_array_length(&value_param.ty.cpp_type) else {
                return format!("    {receiver}->{} = value;\n", field_name);
            };
            let copy_len = length.saturating_sub(1);
            format!(
                "    if (value == nullptr) {{\n        {receiver}->{}[0] = '\\0';\n        return;\n    }}\n    std::strncpy({receiver}->{}, value, {});\n    {receiver}->{}[{}] = '\\0';\n",
                field_name, field_name, copy_len, field_name, copy_len
            )
        }
        IrTypeKind::FixedByteArray => format!(
            "    if (value == nullptr) {{\n        return;\n    }}\n    std::memcpy({receiver}->{field_name}, value, sizeof({receiver}->{field_name}));\n"
        ),
        IrTypeKind::FixedArray => format!(
            "    if (value == nullptr) {{\n        return;\n    }}\n    std::memcpy({receiver}->{field_name}, value, sizeof({receiver}->{field_name}));\n"
        ),
        IrTypeKind::FixedModelArray => {
            let base_cpp = fixed_model_array_elem_cpp_type(&value_param.ty.cpp_type);
            let n = ir::fixed_array_length(&value_param.ty.cpp_type).unwrap_or(0);
            format!(
                "    if (value == nullptr) {{\n        return;\n    }}\n    for (int _i = 0; _i < {n}; _i++) {{\n        {receiver}->{field_name}[_i] = *reinterpret_cast<{base_cpp}*>(value[_i]);\n    }}\n"
            )
        }
        IrTypeKind::ModelValue => format!(
            "    {receiver}->{field_name} = {};\n",
            render_model_deref_cast(&value_param.ty, "value")
        ),
        _ => format!("    {receiver}->{} = value;\n", field_name),
    }
}

fn render_model_value_return(function: &IrFunction, target: &str, args: &str) -> String {
    let base = base_model_cpp_type(&function.returns.cpp_type);
    if function.returns.cpp_type.trim_end().ends_with('*') {
        return format!(
            "    auto result = {}({});\n    if (result == nullptr) {{\n        return nullptr;\n    }}\n    return reinterpret_cast<{}>(new {}(*result));\n",
            target, args, function.returns.c_type, base
        );
    }

    format!(
        "    return reinterpret_cast<{}>(new {}({}({})));\n",
        function.returns.c_type, base, target, args
    )
}

fn opaque_model_value_handles_needing_go_ownership(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut unavailable_handles = covered_handles.clone();
    unavailable_handles.extend(
        ctx.known_model_projections
            .iter()
            .map(|projection| projection.handle_name.clone()),
    );
    ir.functions
        .iter()
        .filter(|function| function.returns.kind == IrTypeKind::ModelValue)
        .filter_map(|function| function.returns.handle.clone())
        .filter(|handle| !unavailable_handles.contains(handle))
        .collect()
}

fn synthetic_opaque_model_value_deletes(
    ctx: &PipelineContext,
    ir: &IrModule,
    covered_handles: &BTreeSet<String>,
    owned_opaque_value_handles: Option<&BTreeSet<String>>,
) -> Vec<SyntheticOpaqueDelete> {
    let mut unavailable_handles = existing_owned_model_handles(ctx, ir);
    unavailable_handles.extend(covered_handles.iter().cloned());
    let existing_symbols = ir
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut deletes = BTreeMap::<String, SyntheticOpaqueDelete>::new();

    for function in &ir.functions {
        if function.returns.kind != IrTypeKind::ModelValue {
            continue;
        }
        let Some(handle_name) = function.returns.handle.as_deref() else {
            continue;
        };
        if owned_opaque_value_handles.is_some_and(|handles| !handles.contains(handle_name)) {
            continue;
        }
        if unavailable_handles.contains(handle_name) {
            continue;
        }
        let symbol_name = opaque_delete_symbol(handle_name);
        if existing_symbols.contains(symbol_name.as_str()) {
            continue;
        }
        deletes
            .entry(handle_name.to_string())
            .or_insert_with(|| SyntheticOpaqueDelete {
                handle_name: handle_name.to_string(),
                cpp_type: base_model_cpp_type(&function.returns.cpp_type),
                symbol_name,
            });
    }

    deletes.into_values().collect()
}

fn existing_owned_model_handles(ctx: &PipelineContext, ir: &IrModule) -> BTreeSet<String> {
    let mut handles = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Destructor)
        .filter_map(|function| function.params.first())
        .filter_map(|param| param.ty.handle.clone())
        .collect::<BTreeSet<_>>();
    handles.extend(
        ctx.known_model_projections
            .iter()
            .map(|projection| projection.handle_name.clone()),
    );
    handles
}

fn opaque_delete_symbol(handle_name: &str) -> String {
    let base = handle_name.strip_suffix("Handle").unwrap_or(handle_name);
    format!("{WRAPPER_PREFIX}_{base}_delete")
}

fn render_synthetic_opaque_delete_def(delete: &SyntheticOpaqueDelete) -> String {
    format!(
        "void {}({}* self) {{\n    delete reinterpret_cast<{}*>(self);\n}}\n",
        delete.symbol_name, delete.handle_name, delete.cpp_type
    )
}

fn call_args(function: &IrFunction, start: usize) -> String {
    function
        .params
        .iter()
        .skip(start)
        .map(|param| render_cpp_arg(param.ty.clone(), &param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_cpp_arg(ty: IrType, name: &str) -> String {
    match ty.kind {
        IrTypeKind::Enum => format!("static_cast<{}>({name})", ty.cpp_type.trim()),
        IrTypeKind::Primitive if ty.cpp_type != ty.c_type => {
            // Use the C++ alias name only for known generator aliases (e.g. uint32, int32).
            // For unknown project-specific typedefs (e.g. iChLeg_t) that are not in scope
            // inside the generated wrapper file, fall back to the canonical C type.
            let cast_type = if generator_supported_primitive(&ty.cpp_type) {
                &ty.cpp_type
            } else {
                &ty.c_type
            };
            format!("static_cast<{cast_type}>({name})")
        }
        IrTypeKind::String => format!("std::string({name} != nullptr ? {name} : \"\")"),
        IrTypeKind::Reference => primitive_alias_cast_target(&ty)
            .map(|cpp_type| format!("*reinterpret_cast<{}*>({name})", cpp_type))
            .or_else(|| {
                // Unknown typedef: cast using canonical C type to avoid referencing
                // project-specific type aliases not in scope in the wrapper.
                let c_base = ty.c_type.trim_end_matches('*').trim();
                let cpp_base = ty.cpp_type.trim_end_matches('&').trim();
                if !generator_supported_primitive(cpp_base) && generator_supported_primitive(c_base)
                {
                    Some(format!("*reinterpret_cast<{}*>({name})", c_base))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| format!("*{name}")),
        IrTypeKind::Pointer => primitive_alias_cast_target(&ty)
            .map(|cpp_type| format!("reinterpret_cast<{}*>({name})", cpp_type))
            .unwrap_or_else(|| name.to_string()),
        IrTypeKind::ExternStructReference => format!("*{name}"),
        IrTypeKind::ModelReference => render_model_deref_cast(&ty, name),
        IrTypeKind::ModelValue => render_model_deref_cast(&ty, name),
        IrTypeKind::ModelPointer => {
            let base = qualified_model_cpp_type(&ty);
            let depth = ty.cpp_type.chars().filter(|ch| *ch == '*').count().max(1);
            let stars = "*".repeat(depth);
            format!("reinterpret_cast<{base}{stars}>({name})")
        }
        IrTypeKind::FixedByteArray => name.to_string(),
        _ => name.to_string(),
    }
}

fn primitive_alias_cast_target(ty: &IrType) -> Option<&str> {
    let cpp_base = match ty.kind {
        IrTypeKind::Reference => ty.cpp_type.trim_end_matches('&').trim(),
        IrTypeKind::Pointer => ty.cpp_type.trim_end_matches('*').trim(),
        _ => return None,
    };
    let c_base = ty.c_type.trim_end_matches('*').trim();
    if generator_supported_primitive(cpp_base) && cpp_base != c_base {
        Some(cpp_base)
    } else {
        None
    }
}

fn char_array_length(cpp_type: &str) -> Option<usize> {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    let prefix = trimmed.strip_prefix("char[")?;
    let len = prefix.strip_suffix(']')?;
    len.parse().ok()
}

fn generator_supported_primitive(name: &str) -> bool {
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

fn callback_bridge_functions(ir: &IrModule) -> Vec<IrFunction> {
    ir.functions
        .iter()
        .filter(|function| {
            function
                .params
                .iter()
                .any(|param| param.ty.kind == IrTypeKind::Callback)
        })
        .map(make_callback_bridge_function)
        .collect()
}

fn make_callback_bridge_function(function: &IrFunction) -> IrFunction {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            if param.ty.kind == IrTypeKind::Callback {
                IrParam {
                    name: format!("use_cb{index}"),
                    ty: IrType {
                        kind: IrTypeKind::Primitive,
                        cpp_type: "bool".to_string(),
                        c_type: "bool".to_string(),
                        handle: None,
                    },
                }
            } else {
                param.clone()
            }
        })
        .collect::<Vec<_>>();

    IrFunction {
        name: format!("{}_bridge", function.name),
        kind: IrFunctionKind::Function,
        cpp_name: function.cpp_name.clone(),
        method_of: function.method_of.clone(),
        owner_cpp_type: function.owner_cpp_type.clone(),
        is_const: function.is_const,
        field_accessor: None,
        returns: function.returns.clone(),
        params,
    }
}

fn callback_map(ir: &IrModule) -> std::collections::BTreeMap<String, IrCallback> {
    ir.callbacks
        .iter()
        .map(|callback| (callback.name.clone(), callback.clone()))
        .collect()
}

fn render_callback_bridge_body(
    function: &IrFunction,
    callbacks: &std::collections::BTreeMap<String, IrCallback>,
) -> String {
    let mut out = String::new();

    for (index, param) in function.params.iter().enumerate() {
        if param.ty.kind != IrTypeKind::Callback {
            continue;
        }
        let callback = callbacks
            .get(&param.ty.cpp_type)
            .expect("callback bridge requires callback typedef metadata");
        out.push_str(&render_callback_trampoline_decl(function, index, callback));
    }

    let target = function.name.clone();
    let call_args = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            if param.ty.kind == IrTypeKind::Callback {
                format!(
                    "use_cb{index} ? {} : nullptr",
                    callback_trampoline_name(function, index)
                )
            } else {
                param.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    match function.returns.kind {
        IrTypeKind::Void => out.push_str(&format!("    {}({});\n", target, call_args)),
        _ => out.push_str(&format!("    return {}({});\n", target, call_args)),
    }

    out
}

fn render_go_callback_extern_c_block(
    functions: &[IrFunction],
    callbacks: &std::collections::BTreeMap<String, IrCallback>,
) -> String {
    let mut decls = Vec::new();
    for function in functions {
        for (index, param) in function.params.iter().enumerate() {
            if param.ty.kind != IrTypeKind::Callback {
                continue;
            }
            if let Some(callback) = callbacks.get(&param.ty.cpp_type) {
                let params = if callback.params.is_empty() {
                    "void".to_string()
                } else {
                    callback
                        .params
                        .iter()
                        .map(|p| format!("{} {}", p.ty.c_type, p.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let go_symbol = callback_go_export_name(function, index);
                decls.push(format!(
                    "    {} {}({});",
                    callback.returns.c_type, go_symbol, params
                ));
            }
        }
    }
    if decls.is_empty() {
        return String::new();
    }
    format!("extern \"C\" {{\n{}\n}}\n\n", decls.join("\n"))
}

fn render_callback_trampoline_decl(
    function: &IrFunction,
    index: usize,
    callback: &IrCallback,
) -> String {
    let params = if callback.params.is_empty() {
        "void".to_string()
    } else {
        callback
            .params
            .iter()
            .map(|param| format!("{} {}", param.ty.c_type, param.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let call_args = callback
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let go_symbol = callback_go_export_name(function, index);
    let invoke = if callback.returns.kind == IrTypeKind::Void {
        format!("{}({});", go_symbol, call_args)
    } else {
        format!("return {}({});", go_symbol, call_args)
    };
    format!(
        "    {} {} = []({}) -> {} {{ {} }};\n",
        callback.name,
        callback_trampoline_name(function, index),
        params,
        callback.returns.c_type,
        invoke
    )
}

fn callback_trampoline_name(function: &IrFunction, index: usize) -> String {
    format!("{}_cb{}_trampoline", function.name, index)
}

fn callback_go_export_name(function: &IrFunction, index: usize) -> String {
    format!("go_{}_cb{}", function.name, index)
}

fn render_model_deref_cast(ty: &IrType, name: &str) -> String {
    format!(
        "*reinterpret_cast<{}*>({name})",
        qualified_model_cpp_type(ty)
    )
}

fn qualified_model_cpp_type(ty: &IrType) -> String {
    let trimmed = ty.cpp_type.trim();
    let base = trimmed.trim_end_matches('&').trim_end_matches('*').trim();
    if let Some(inner) = base.strip_suffix(" const") {
        return format!("const {}", inner.trim());
    }
    base.to_string()
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

fn fixed_model_array_elem_cpp_type(cpp_type: &str) -> String {
    ir::fixed_array_elem_type(cpp_type)
        .map(base_model_cpp_type)
        .unwrap_or_else(|| base_model_cpp_type(cpp_type))
}

fn ir_uses_struct_timeval(ir: &IrModule) -> bool {
    ir.functions
        .iter()
        .flat_map(|function| {
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        })
        .chain(ir.callbacks.iter().flat_map(|callback| {
            std::iter::once(&callback.returns).chain(callback.params.iter().map(|param| &param.ty))
        }))
        .any(|ty| {
            matches!(
                ty.kind,
                IrTypeKind::ExternStructReference | IrTypeKind::ExternStructPointer
            ) && base_model_cpp_type(&ty.c_type) == "struct timeval"
        })
}

fn render_string_free(_ctx: &PipelineContext) -> String {
    format!(
        "void {}_string_free(char* value) {{\n    free(value);\n}}\n",
        WRAPPER_PREFIX
    )
}
