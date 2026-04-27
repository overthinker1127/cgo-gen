#![allow(non_upper_case_globals)]
use std::{
    collections::BTreeSet,
    ffi::{CStr, CString},
    os::raw::{c_int, c_uint, c_void},
    path::{Path, PathBuf},
    ptr,
};

use anyhow::{Result, anyhow, bail};
use clang_sys::*;
use serde::Serialize;

use crate::{domain::kind::RecordKind, parsing::compiler, pipeline::context::PipelineContext};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParsedApi {
    pub headers: Vec<String>,
    pub functions: Vec<CppFunction>,
    pub records: Vec<CppRecord>,
    pub enums: Vec<CppEnum>,
    pub macros: Vec<CppMacroConstant>,
    pub callbacks: Vec<CppCallbackTypedef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppRecord {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub kind: RecordKind,
    pub fields: Vec<CppField>,
    pub methods: Vec<CppMethod>,
    pub constructors: Vec<CppConstructor>,
    pub has_destructor: bool,
    pub has_declared_constructor: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppFunction {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppMethod {
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppConstructor {
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppField {
    pub name: String,
    pub ty: String,
    pub canonical_ty: String,
    pub is_function_pointer: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppParam {
    pub name: String,
    pub ty: String,
    pub canonical_ty: String,
    pub is_function_pointer: bool,
    pub callback_typedef: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_default: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppCallbackTypedef {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnum {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    #[serde(skip)]
    pub is_anonymous: bool,
    pub variants: Vec<CppEnumVariant>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnumVariant {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppMacroConstant {
    pub source_header: PathBuf,
    pub name: String,
    pub value: String,
}

impl ParsedApi {
    pub fn filter_to_header(&self, header: &Path) -> Self {
        Self {
            headers: vec![header.display().to_string()],
            functions: self
                .functions
                .iter()
                .filter(|function| same_path(&function.source_header, header))
                .cloned()
                .collect(),
            records: self
                .records
                .iter()
                .filter(|record| same_path(&record.source_header, header))
                .cloned()
                .collect(),
            enums: self
                .enums
                .iter()
                .filter(|item| same_path(&item.source_header, header))
                .cloned()
                .collect(),
            macros: self
                .macros
                .iter()
                .filter(|item| same_path(&item.source_header, header))
                .cloned()
                .collect(),
            callbacks: self
                .callbacks
                .iter()
                .filter(|item| same_path(&item.source_header, header))
                .cloned()
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.records.is_empty()
            && self.enums.is_empty()
            && self.macros.is_empty()
            && self.callbacks.is_empty()
    }
}

pub fn parse(ctx: &PipelineContext) -> Result<ParsedApi> {
    let mut api = ParsedApi::default();
    let filter = ParseFilter::from_context(&ctx);
    let translation_units = compiler::collect_translation_units(&ctx.config)?;
    let mut discovered_headers = BTreeSet::new();
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            bail!("failed to create libclang index");
        }

        parse_translation_units(
            index,
            ctx,
            &filter,
            &translation_units,
            &mut discovered_headers,
            &mut api,
        )?;

        if let Some(dir) = &ctx.input.dir {
            let all_headers = ctx.config.discovered_headers()?;
            let supplemental_headers = all_headers
                .into_iter()
                .filter(|path| path.starts_with(dir))
                .filter(|path| {
                    !discovered_headers.iter().any(|seen| {
                        Path::new(seen)
                            .canonicalize()
                            .map(|candidate| candidate == *path)
                            .unwrap_or(false)
                    })
                })
                .filter(|path| {
                    ctx.target_header
                        .as_ref()
                        .map(|target| same_path(path, target))
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();

            if !supplemental_headers.is_empty() {
                parse_translation_units(
                    index,
                    ctx,
                    &filter,
                    &supplemental_headers,
                    &mut discovered_headers,
                    &mut api,
                )?;
            }
        }

        clang_disposeIndex(index);
    }

    api.headers = discovered_headers.into_iter().collect();
    dedupe_api(&mut api);
    Ok(api)
}

unsafe fn parse_translation_units(
    index: CXIndex,
    ctx: &PipelineContext,
    filter: &ParseFilter,
    translation_units: &[PathBuf],
    discovered_headers: &mut BTreeSet<String>,
    api: &mut ParsedApi,
) -> Result<()> {
    for translation_unit_path in translation_units {
        compiler::ensure_header_exists(translation_unit_path)?;
        let args = compiler::collect_clang_args(&ctx.config, translation_unit_path)?;
        let c_header = CString::new(translation_unit_path.to_string_lossy().to_string())?;
        let c_args = args
            .iter()
            .map(|arg| CString::new(arg.as_str()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut arg_ptrs = c_args.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();

        let flags = (CXTranslationUnit_DetailedPreprocessingRecord
            | CXTranslationUnit_SkipFunctionBodies) as c_int;
        let mut translation_unit = ptr::null_mut();
        let error = unsafe {
            clang_parseTranslationUnit2(
                index,
                c_header.as_ptr(),
                arg_ptrs.as_mut_ptr(),
                arg_ptrs.len() as c_int,
                ptr::null_mut(),
                0,
                flags,
                &mut translation_unit,
            )
        };

        if error != CXError_Success || translation_unit.is_null() {
            bail!(
                "failed to parse {} with libclang (error code {})",
                translation_unit_path.display(),
                error
            );
        }

        let root = unsafe { clang_getTranslationUnitCursor(translation_unit) };
        for child in direct_children(root) {
            collect_entity(child, &[], filter, discovered_headers, api)?;
        }

        let diagnostics = collect_diagnostics(translation_unit);
        if !diagnostics.is_empty() {
            unsafe { clang_disposeTranslationUnit(translation_unit) };
            bail!(
                "libclang reported diagnostics while parsing {}:\n{}",
                translation_unit_path.display(),
                diagnostics.join("\n")
            );
        }

        unsafe { clang_disposeTranslationUnit(translation_unit) };
    }

    Ok(())
}

fn dedupe_api(api: &mut ParsedApi) {
    api.functions = api
        .functions
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.records = api
        .records
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.enums = api
        .enums
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.macros = api
        .macros
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.callbacks = api
        .callbacks
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
}

#[derive(Debug, Clone)]
struct ParseFilter {
    main_file_only: bool,
    owned_dir: Option<PathBuf>,
    target_header: Option<PathBuf>,
}

impl ParseFilter {
    fn from_context(ctx: &PipelineContext) -> Self {
        Self {
            main_file_only: false,
            owned_dir: ctx.input.dir.clone(),
            target_header: ctx.target_header.clone(),
        }
    }
}

fn collect_entity(
    cursor: CXCursor,
    namespace: &[String],
    filter: &ParseFilter,
    discovered_headers: &mut BTreeSet<String>,
    api: &mut ParsedApi,
) -> Result<()> {
    if !should_collect_cursor(cursor, filter) {
        return Ok(());
    }
    record_header_path(cursor, discovered_headers);

    match unsafe { clang_getCursorKind(cursor) } {
        CXCursor_Namespace => {
            let Some(name) = cursor_spelling(cursor) else {
                return Ok(());
            };
            let mut next_namespace = namespace.to_vec();
            next_namespace.push(name);
            for child in direct_children(cursor) {
                collect_entity(child, &next_namespace, filter, discovered_headers, api)?;
            }
        }
        CXCursor_ClassDecl | CXCursor_StructDecl => {
            if unsafe { clang_isCursorDefinition(cursor) } == 0 {
                return Ok(());
            }
            if cursor_spelling(cursor).is_some() {
                let parsed = parse_record(cursor, namespace.to_vec(), filter, discovered_headers)?;
                if parsed.has_declared_constructor
                    || parsed.has_destructor
                    || !parsed.fields.is_empty()
                    || !parsed.methods.is_empty()
                {
                    api.records.push(parsed);
                }
            }
        }
        CXCursor_FunctionDecl => {
            if cursor_spelling(cursor).is_some() {
                api.functions
                    .push(parse_function(cursor, namespace.to_vec())?);
            }
        }
        CXCursor_TypedefDecl => {
            let Some(name) = cursor_spelling(cursor) else {
                return Ok(());
            };
            if let Some(callback) =
                parse_callback_typedef(cursor, namespace.to_vec(), name.clone())?
            {
                api.callbacks.push(callback);
                return Ok(());
            }
            let Some(enum_cursor) = direct_children(cursor)
                .into_iter()
                .find(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumDecl)
            else {
                return Ok(());
            };
            if enum_decl_name(enum_cursor).is_none() {
                api.enums
                    .push(parse_enum_with_name(enum_cursor, namespace.to_vec(), name));
            }
        }
        CXCursor_EnumDecl => {
            if let Some(name) = enum_decl_name(cursor) {
                api.enums
                    .push(parse_enum_with_name(cursor, namespace.to_vec(), name));
            } else {
                api.enums
                    .push(parse_anonymous_enum(cursor, namespace.to_vec()));
            }
        }
        CXCursor_MacroDefinition => {
            if let Some(item) = parse_macro_definition(cursor) {
                api.macros.push(item);
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_record(
    cursor: CXCursor,
    namespace: Vec<String>,
    filter: &ParseFilter,
    discovered_headers: &mut BTreeSet<String>,
) -> Result<CppRecord> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("anonymous classes are unsupported in v1"))?;
    let source_header = normalized_cursor_file_path(cursor)
        .ok_or_else(|| anyhow!("failed to determine source header for class `{name}`"))?;
    let kind = if unsafe { clang_getCursorKind(cursor) == CXCursor_StructDecl } {
        RecordKind::Struct
    } else {
        RecordKind::Class
    };
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut constructors = Vec::new();
    let mut has_destructor = false;
    let mut has_declared_constructor = false;
    let mut is_abstract = false;

    for child in direct_children(cursor) {
        if !should_collect_cursor(child, filter) {
            continue;
        }
        record_header_path(child, discovered_headers);
        let accessible = matches!(unsafe { clang_getCXXAccessSpecifier(child) }, CX_CXXPublic)
            || (kind == RecordKind::Struct
                && unsafe { clang_getCXXAccessSpecifier(child) } == CX_CXXInvalidAccessSpecifier);

        match unsafe { clang_getCursorKind(child) } {
            CXCursor_CXXMethod if unsafe { clang_CXXMethod_isPureVirtual(child) != 0 } => {
                is_abstract = true;
            }
            _ => {}
        }

        if !accessible {
            continue;
        }

        match unsafe { clang_getCursorKind(child) } {
            CXCursor_Constructor => {
                has_declared_constructor = true;
                constructors.push(CppConstructor {
                    params: parse_params(child),
                });
            }
            CXCursor_Destructor => has_destructor = true,
            CXCursor_FieldDecl => {
                if let Some(name) = cursor_spelling(child) {
                    fields.push(CppField {
                        name,
                        ty: canonicalize_type_name(&cursor_type_spelling(child)),
                        canonical_ty: canonicalize_type_name(&cursor_canonical_type_spelling(
                            child,
                        )),
                        is_function_pointer: cursor_is_function_pointer(child),
                    });
                }
            }
            CXCursor_CXXMethod => {
                methods.push(CppMethod {
                    name: cursor_spelling(child).unwrap_or_default(),
                    return_type: result_type_name(child),
                    return_canonical_type: result_canonical_type_name(child),
                    return_is_function_pointer: result_is_function_pointer(child),
                    params: parse_params(child),
                    is_const: unsafe { clang_CXXMethod_isConst(child) != 0 },
                });
            }
            _ => {}
        }
    }

    Ok(CppRecord {
        source_header,
        namespace,
        name,
        kind,
        fields,
        methods,
        constructors,
        has_destructor,
        has_declared_constructor,
        is_abstract,
    })
}

fn parse_function(cursor: CXCursor, namespace: Vec<String>) -> Result<CppFunction> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("encountered unnamed function declaration"))?;
    let source_header = normalized_cursor_file_path(cursor)
        .ok_or_else(|| anyhow!("failed to determine source header for function `{name}`"))?;
    Ok(CppFunction {
        source_header,
        namespace,
        name,
        return_type: result_type_name(cursor),
        return_canonical_type: result_canonical_type_name(cursor),
        return_is_function_pointer: result_is_function_pointer(cursor),
        params: parse_params(cursor),
    })
}

fn parse_enum_with_name(cursor: CXCursor, namespace: Vec<String>, name: String) -> CppEnum {
    let source_header = normalized_cursor_file_path(cursor).unwrap_or_default();
    let variants = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumConstantDecl)
        .map(|child| CppEnumVariant {
            name: cursor_spelling(child).unwrap_or_default(),
            value: Some(unsafe { clang_getEnumConstantDeclValue(child) }.to_string()),
        })
        .collect();

    CppEnum {
        source_header,
        namespace,
        name,
        is_anonymous: false,
        variants,
    }
}

fn parse_anonymous_enum(cursor: CXCursor, namespace: Vec<String>) -> CppEnum {
    let source_header = normalized_cursor_file_path(cursor).unwrap_or_default();
    let (line, column) = cursor_line_column(cursor);
    let variants = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumConstantDecl)
        .map(|child| CppEnumVariant {
            name: cursor_spelling(child).unwrap_or_default(),
            value: Some(unsafe { clang_getEnumConstantDeclValue(child) }.to_string()),
        })
        .collect();

    CppEnum {
        source_header,
        namespace,
        name: format!("__anonymous_enum_{line}_{column}"),
        is_anonymous: true,
        variants,
    }
}

fn parse_macro_definition(cursor: CXCursor) -> Option<CppMacroConstant> {
    if unsafe { clang_Cursor_isMacroFunctionLike(cursor) } != 0 {
        return None;
    }

    let name = cursor_spelling(cursor)?;
    let source_header = normalized_cursor_file_path(cursor)?;
    let tokens = cursor_token_spellings(cursor);
    let name_index = tokens.iter().position(|token| token == &name)?;
    let replacement = tokens[name_index + 1..].join("");
    let value = normalize_macro_integer_literal(&replacement)?;

    Some(CppMacroConstant {
        source_header,
        name,
        value,
    })
}

fn parse_callback_typedef(
    cursor: CXCursor,
    namespace: Vec<String>,
    name: String,
) -> Result<Option<CppCallbackTypedef>> {
    let underlying = unsafe { clang_getTypedefDeclUnderlyingType(cursor) };
    let function_type = callback_function_type(underlying);
    if function_type.kind == CXType_Invalid {
        return Ok(None);
    }

    let source_header = normalized_cursor_file_path(cursor).ok_or_else(|| {
        anyhow!("failed to determine source header for callback typedef `{name}`")
    })?;
    let return_type =
        canonicalize_type_name(&unsafe { type_spelling(clang_getResultType(function_type)) });
    let return_canonical_type = canonicalize_type_name(&unsafe {
        type_spelling(clang_getCanonicalType(clang_getResultType(function_type)))
    });

    let child_params = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_ParmDecl)
        .map(|arg| CppParam {
            name: cursor_spelling(arg).unwrap_or_else(|| "arg".to_string()),
            ty: canonicalize_type_name(&cursor_type_spelling(arg)),
            canonical_ty: canonicalize_type_name(&cursor_canonical_type_spelling(arg)),
            is_function_pointer: cursor_is_function_pointer(arg),
            callback_typedef: callback_typedef_name_from_type(unsafe { clang_getCursorType(arg) }),
            has_default: param_has_default(arg),
        })
        .enumerate()
        .map(|(index, mut param)| {
            if param.name.is_empty() || param.name == "arg" {
                param.name = format!("arg{index}");
            }
            param
        })
        .collect::<Vec<_>>();

    let params = if !child_params.is_empty() {
        child_params
    } else {
        parse_callback_params_from_type(function_type)
    };

    Ok(Some(CppCallbackTypedef {
        source_header,
        namespace,
        name,
        return_type,
        return_canonical_type,
        params,
    }))
}

fn enum_decl_name(cursor: CXCursor) -> Option<String> {
    cursor_spelling(cursor).filter(|name| !is_unnamed_enum_spelling(name))
}

fn cursor_line_column(cursor: CXCursor) -> (u32, u32) {
    let location = unsafe { clang_getCursorLocation(cursor) };
    let mut line = 0;
    let mut column = 0;
    unsafe {
        clang_getSpellingLocation(
            location,
            std::ptr::null_mut(),
            &mut line,
            &mut column,
            std::ptr::null_mut(),
        );
    }
    (line, column)
}

fn is_unnamed_enum_spelling(name: &str) -> bool {
    name.starts_with("(unnamed enum at ")
}

fn parse_params(cursor: CXCursor) -> Vec<CppParam> {
    let count = unsafe { clang_Cursor_getNumArguments(cursor) };
    if count < 0 {
        return Vec::new();
    }

    (0..count)
        .map(|index| unsafe { clang_Cursor_getArgument(cursor, index as c_uint) })
        .map(|arg| CppParam {
            name: cursor_spelling(arg).unwrap_or_else(|| "arg".to_string()),
            ty: canonicalize_type_name(&cursor_type_spelling(arg)),
            canonical_ty: canonicalize_type_name(&cursor_canonical_type_spelling(arg)),
            is_function_pointer: cursor_is_function_pointer(arg),
            callback_typedef: callback_typedef_name_from_type(unsafe { clang_getCursorType(arg) }),
            has_default: param_has_default(arg),
        })
        .enumerate()
        .map(|(index, mut param)| {
            if param.name.is_empty() || param.name == "arg" {
                param.name = format!("arg{index}");
            }
            param
        })
        .collect()
}

fn parse_callback_params_from_type(function_type: CXType) -> Vec<CppParam> {
    let count = unsafe { clang_getNumArgTypes(function_type) };
    if count < 0 {
        return Vec::new();
    }

    (0..count)
        .map(|index| unsafe { clang_getArgType(function_type, index as c_uint) })
        .enumerate()
        .map(|(index, ty)| CppParam {
            name: format!("arg{index}"),
            ty: canonicalize_type_name(&unsafe { type_spelling(ty) }),
            canonical_ty: canonicalize_type_name(&unsafe {
                type_spelling(clang_getCanonicalType(ty))
            }),
            is_function_pointer: is_function_pointer_type(ty),
            callback_typedef: callback_typedef_name_from_type(ty),
            has_default: false,
        })
        .collect()
}

fn param_has_default(cursor: CXCursor) -> bool {
    cursor_token_spellings(cursor)
        .iter()
        .any(|token| token == "=")
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn result_type_name(cursor: CXCursor) -> String {
    canonicalize_type_name(&unsafe { type_spelling(clang_getCursorResultType(cursor)) })
}

fn result_canonical_type_name(cursor: CXCursor) -> String {
    canonicalize_type_name(&unsafe {
        type_spelling(clang_getCanonicalType(clang_getCursorResultType(cursor)))
    })
}

fn result_is_function_pointer(cursor: CXCursor) -> bool {
    is_function_pointer_type(unsafe { clang_getCursorResultType(cursor) })
}

fn cursor_type_spelling(cursor: CXCursor) -> String {
    unsafe { type_spelling(clang_getCursorType(cursor)) }
}

fn cursor_canonical_type_spelling(cursor: CXCursor) -> String {
    unsafe { type_spelling(clang_getCanonicalType(clang_getCursorType(cursor))) }
}

fn cursor_is_function_pointer(cursor: CXCursor) -> bool {
    is_function_pointer_type(unsafe { clang_getCursorType(cursor) })
}

unsafe fn type_spelling(ty: CXType) -> String {
    unsafe { cxstring_to_string(clang_getTypeSpelling(ty)) }
}

fn is_function_pointer_type(ty: CXType) -> bool {
    let canonical = unsafe { clang_getCanonicalType(ty) };
    match canonical.kind {
        CXType_FunctionProto | CXType_FunctionNoProto => true,
        CXType_Pointer => {
            let pointee = unsafe { clang_getPointeeType(canonical) };
            matches!(pointee.kind, CXType_FunctionProto | CXType_FunctionNoProto)
        }
        _ => false,
    }
}

fn callback_function_type(ty: CXType) -> CXType {
    let canonical = unsafe { clang_getCanonicalType(ty) };
    match canonical.kind {
        CXType_FunctionProto | CXType_FunctionNoProto => canonical,
        CXType_Pointer => {
            let pointee = unsafe { clang_getPointeeType(canonical) };
            if matches!(pointee.kind, CXType_FunctionProto | CXType_FunctionNoProto) {
                pointee
            } else {
                invalid_type()
            }
        }
        _ => invalid_type(),
    }
}

fn invalid_type() -> CXType {
    CXType {
        kind: CXType_Invalid,
        data: [ptr::null_mut(); 2],
    }
}

fn callback_typedef_name_from_type(ty: CXType) -> Option<String> {
    if !is_function_pointer_type(ty) {
        return None;
    }

    let declaration = unsafe { clang_getTypeDeclaration(ty) };
    if unsafe { clang_equalCursors(declaration, clang_getNullCursor()) } != 0 {
        return None;
    }
    if unsafe { clang_getCursorKind(declaration) } != CXCursor_TypedefDecl {
        return None;
    }

    cursor_spelling(declaration)
}

fn direct_children(cursor: CXCursor) -> Vec<CXCursor> {
    let mut children = Vec::new();
    unsafe {
        clang_visitChildren(
            cursor,
            collect_child,
            &mut children as *mut Vec<CXCursor> as *mut c_void,
        );
    }
    children
}

extern "C" fn collect_child(
    cursor: CXCursor,
    _parent: CXCursor,
    data: CXClientData,
) -> CXChildVisitResult {
    let children = unsafe { &mut *(data as *mut Vec<CXCursor>) };
    children.push(cursor);
    CXChildVisit_Continue
}

fn collect_diagnostics(translation_unit: CXTranslationUnit) -> Vec<String> {
    let count = unsafe { clang_getNumDiagnostics(translation_unit) };
    let mut diagnostics = Vec::new();
    for index in 0..count {
        unsafe {
            let diagnostic = clang_getDiagnostic(translation_unit, index);
            let severity = clang_getDiagnosticSeverity(diagnostic);
            if severity >= CXDiagnostic_Error {
                let message = cxstring_to_string(clang_formatDiagnostic(
                    diagnostic,
                    clang_defaultDiagnosticDisplayOptions(),
                ));
                diagnostics.push(message);
            }
            clang_disposeDiagnostic(diagnostic);
        }
    }
    diagnostics
}

fn should_collect_cursor(cursor: CXCursor, filter: &ParseFilter) -> bool {
    if is_system_header(cursor) {
        return false;
    }
    if filter.main_file_only {
        if !is_main_file(cursor) {
            return false;
        }
        return matches_target_header(cursor, filter.target_header.as_ref());
    }
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    let Some(dir) = &filter.owned_dir else {
        return false;
    };
    path_is_within(&path, dir)
        && is_header_path(&path)
        && match filter.target_header.as_ref() {
            Some(target) => same_path(&path, target),
            None => true,
        }
}

fn matches_target_header(cursor: CXCursor, target_header: Option<&PathBuf>) -> bool {
    let Some(target_header) = target_header else {
        return true;
    };
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    same_path(&path, target_header)
}

fn same_path(path: &Path, target: &Path) -> bool {
    if path == target {
        return true;
    }
    match (path.canonicalize(), target.canonicalize()) {
        (Ok(path), Ok(target)) => path == target,
        _ => false,
    }
}

fn normalized_cursor_file_path(cursor: CXCursor) -> Option<PathBuf> {
    let path = cursor_file_path(cursor)?;
    Some(path.canonicalize().unwrap_or(path))
}

fn is_main_file(cursor: CXCursor) -> bool {
    unsafe { clang_Location_isFromMainFile(clang_getCursorLocation(cursor)) != 0 }
}

fn is_system_header(cursor: CXCursor) -> bool {
    unsafe { clang_Location_isInSystemHeader(clang_getCursorLocation(cursor)) != 0 }
}

fn cursor_spelling(cursor: CXCursor) -> Option<String> {
    let spelling = unsafe { cxstring_to_string(clang_getCursorSpelling(cursor)) };
    if spelling.is_empty() {
        None
    } else {
        Some(spelling)
    }
}

fn cursor_token_spellings(cursor: CXCursor) -> Vec<String> {
    unsafe {
        let translation_unit = clang_Cursor_getTranslationUnit(cursor);
        if translation_unit.is_null() {
            return Vec::new();
        }

        let mut tokens = ptr::null_mut();
        let mut token_count = 0;
        clang_tokenize(
            translation_unit,
            clang_getCursorExtent(cursor),
            &mut tokens,
            &mut token_count,
        );
        if tokens.is_null() || token_count == 0 {
            return Vec::new();
        }

        let slice = std::slice::from_raw_parts(tokens, token_count as usize);
        let out = slice
            .iter()
            .map(|token| cxstring_to_string(clang_getTokenSpelling(translation_unit, *token)))
            .collect::<Vec<_>>();
        clang_disposeTokens(translation_unit, tokens, token_count);
        out
    }
}

fn normalize_macro_integer_literal(value: &str) -> Option<String> {
    let mut normalized = value.trim();
    if normalized.is_empty() {
        return None;
    }

    while let Some(inner) = strip_wrapping_parentheses(normalized) {
        normalized = inner.trim();
    }

    if is_supported_integer_literal(normalized) {
        Some(normalized.to_string())
    } else {
        None
    }
}

fn strip_wrapping_parentheses(value: &str) -> Option<&str> {
    if !value.starts_with('(') || !value.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 && index != value.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }

    (depth == 0).then(|| &value[1..value.len() - 1])
}

fn is_supported_integer_literal(value: &str) -> bool {
    let value = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    if value.is_empty() {
        return false;
    }

    let digits_end = value
        .char_indices()
        .rfind(|(_, ch)| !matches!(ch, 'u' | 'U' | 'l' | 'L'))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let (digits, suffix) = value.split_at(digits_end);
    if digits.is_empty()
        || suffix
            .chars()
            .any(|ch| !matches!(ch, 'u' | 'U' | 'l' | 'L'))
    {
        return false;
    }

    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
    } else {
        digits.chars().all(|ch| ch.is_ascii_digit())
    }
}

fn record_header_path(cursor: CXCursor, discovered_headers: &mut BTreeSet<String>) {
    let Some(path) = cursor_file_path(cursor) else {
        return;
    };
    if !is_header_path(&path) {
        return;
    }
    discovered_headers.insert(path.display().to_string());
}

fn cursor_file_path(cursor: CXCursor) -> Option<PathBuf> {
    unsafe {
        let location = clang_getCursorLocation(cursor);
        if clang_equalLocations(location, clang_getNullLocation()) != 0 {
            return None;
        }

        let mut file = ptr::null_mut();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;
        clang_getExpansionLocation(location, &mut file, &mut line, &mut column, &mut offset);
        if file.is_null() {
            return None;
        }
        let raw = cxstring_to_string(clang_getFileName(file));
        if raw.is_empty() {
            None
        } else {
            Some(PathBuf::from(raw))
        }
    }
}

fn path_is_within(path: &Path, dir: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    path.starts_with(dir)
}

fn is_header_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

unsafe fn cxstring_to_string(raw: CXString) -> String {
    let value = unsafe { clang_getCString(raw) };
    let owned = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    unsafe { clang_disposeString(raw) };
    owned
}

fn canonicalize_type_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" &", "&")
        .replace("* ", "*")
        .replace(" *", "*")
        .replace("< ", "<")
        .replace(" >", ">")
        .trim()
        .to_string()
}
