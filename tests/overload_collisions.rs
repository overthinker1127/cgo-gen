use cgo_gen::{
    config::Config,
    generator::{render_go_structs, render_header, render_source},
    ir, parser,
    pipeline::context::PipelineContext,
};

#[test]
fn disambiguates_overloaded_free_functions_with_signature_suffixes() {
    let config = Config::load("tests/fixtures/overload/free_function.yaml").unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();

    assert_eq!(parsed.functions.len(), 2);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_add__int_int")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_add__double_double")
    );

    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    assert!(header.contains("int cgowrap_clash_add__int_int(int lhs, int rhs);"));
    assert!(header.contains("double cgowrap_clash_add__double_double(double lhs, double rhs);"));
    assert!(source.contains("int cgowrap_clash_add__int_int(int lhs, int rhs)"));
    assert!(source.contains("double cgowrap_clash_add__double_double(double lhs, double rhs)"));
}

#[test]
fn disambiguates_overloaded_methods_with_signature_suffixes() {
    let config = Config::load("tests/fixtures/overload/method.yaml").unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();

    assert_eq!(parsed.records.len(), 1);
    assert_eq!(parsed.records[0].methods.len(), 2);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_Widget_set__int_mut")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_Widget_set__double_mut")
    );

    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    assert!(
        header
            .contains("int cgowrap_clash_Widget_set__int_mut(clashWidgetHandle* self, int value);")
    );
    assert!(header.contains(
        "int cgowrap_clash_Widget_set__double_mut(clashWidgetHandle* self, double value);"
    ));
    assert!(source.contains("cgowrap_clash_Widget_set__int_mut"));
    assert!(source.contains("cgowrap_clash_Widget_set__double_mut"));
}

#[test]
fn disambiguates_overloaded_constructors_without_panicking() {
    let root = std::env::temp_dir().join(format!("c_go_overload_ctor_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        class Widget {
        public:
            Widget() {}
            Widget(int value) {}
        };
        "#,
    )
    .unwrap();
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

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__void")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__int")
    );
}

#[test]
fn renders_go_facade_for_overloaded_constructors_with_explicit_names() {
    let root = std::env::temp_dir().join(format!("c_go_overload_ctor_go_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        class Widget {
        public:
            Widget() {}
            Widget(int nItemMax) {}
            Widget(const Widget& copy) {}
            int Size() const { return 0; }
        };
        "#,
    )
    .unwrap();
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

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let go_files = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go_files.len(), 1, "expected one Go facade file");
    let go = &go_files[0].contents;

    assert!(
        go.contains("func NewWidget() (*Widget, error) {"),
        "expected zero-arg Go constructor but got:\n{go}"
    );
    assert!(
        go.contains("func NewWidgetWithNItemMax(nItemMax int32) (*Widget, error) {"),
        "expected named int Go constructor but got:\n{go}"
    );
    assert!(
        go.contains("func NewWidgetFromCopy(copy *Widget) (*Widget, error) {"),
        "expected copy Go constructor but got:\n{go}"
    );
}
