use cgo_gen::{
    config::Config,
    domain::kind::IrTypeKind,
    generator::{render_go_structs, render_header, render_source},
    ir, parser,
    pipeline::context::PipelineContext,
};

fn write_fixture(name: &str, header: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("c_go_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(root.join("include/Api.hpp"), header).unwrap();
    std::fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();
    root
}

#[test]
fn normalizes_timeval_pointer_and_reference_params() {
    let root = write_fixture(
        "timeval_types",
        r#"
        struct timeval;
        typedef struct timeval timeval;

        bool TakeByPtr(struct timeval* value);
        bool TakeByRef(struct timeval& value);
        bool TakeAlias(timeval* value);
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let by_ptr = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeByPtr")
        .unwrap();
    assert_eq!(by_ptr.params[0].ty.kind, IrTypeKind::ExternStructPointer);
    assert_eq!(by_ptr.params[0].ty.c_type, "struct timeval*");
    assert_eq!(by_ptr.params[0].ty.handle, None);

    let by_ref = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeByRef")
        .unwrap();
    assert_eq!(by_ref.params[0].ty.kind, IrTypeKind::ExternStructReference);
    assert_eq!(by_ref.params[0].ty.c_type, "struct timeval*");
    assert_eq!(by_ref.params[0].ty.handle, None);

    let alias = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAlias")
        .unwrap();
    assert_eq!(alias.params[0].ty.kind, IrTypeKind::ExternStructPointer);
    assert_eq!(alias.params[0].ty.cpp_type, "timeval*");
    assert_eq!(alias.params[0].ty.c_type, "struct timeval*");
    assert_eq!(alias.params[0].ty.handle, None);
}

#[test]
fn renders_go_facade_and_cpp_wrapper_for_timeval_params() {
    let root = write_fixture(
        "timeval_render",
        r#"
        struct timeval;
        typedef struct timeval timeval;

        bool TakeByPtr(struct timeval* value);
        bool TakeByRef(struct timeval& value);
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let header = render_header(&ctx, &ir);
    assert!(header.contains("#include <sys/time.h>"));
    assert!(header.contains("bool cgowrap_TakeByPtr(struct timeval* value);"));
    assert!(header.contains("bool cgowrap_TakeByRef(struct timeval* value);"));

    let go = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("#include <sys/time.h>"));
    assert!(
        go[0]
            .contents
            .contains("func TakeByPtr(value *C.struct_timeval) bool {")
    );
    assert!(
        go[0]
            .contents
            .contains("func TakeByRef(value *C.struct_timeval) bool {")
    );
    assert!(
        go[0]
            .contents
            .contains("cArg0 := (*C.struct_timeval)(unsafe.Pointer(value))")
    );
    assert!(go[0].contents.contains("panic(\"value reference is nil\")"));

    let source = render_source(&ctx, &ir);
    assert!(source.contains("return TakeByPtr(value);"));
    assert!(source.contains("return TakeByRef(*value);"));
}
