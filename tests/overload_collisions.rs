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

#[test]
fn renders_dispatchers_for_unambiguous_go_overloads() {
    let root =
        std::env::temp_dir().join(format!("c_go_overload_dispatcher_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        #include <string>

        class Widget {
        public:
            Widget() {}
            bool set(int value) { return true; }
            bool set(bool value) { return value; }
            std::string describe(int value) { return "int"; }
            std::string describe(bool value) { return value ? "true" : "false"; }
        };

        bool apply(int value) { return true; }
        bool apply(bool value) { return value; }
        std::string label(int value) { return "int"; }
        std::string label(bool value) { return value ? "true" : "false"; }
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

    assert!(go.contains("import \"fmt\""), "expected fmt import:\n{go}");
    assert!(go.contains("func ApplyInt32(value int32) bool {"));
    assert!(go.contains("func ApplyBool(value bool) bool {"));
    assert!(go.contains("func Apply(args ...any) (bool, error) {"));
    assert!(go.contains("return ApplyInt32(arg0), nil"));
    assert!(go.contains("return ApplyBool(arg0), nil"));
    assert!(go.contains("func LabelInt32(value int32) (string, error) {"));
    assert!(go.contains("func LabelBool(value bool) (string, error) {"));
    assert!(go.contains("func Label(args ...any) (string, error) {"));
    assert!(go.contains("return LabelInt32(arg0)"));
    assert!(go.contains("return LabelBool(arg0)"));
    assert!(go.contains("return \"\", fmt.Errorf(\"no matching overload for Label\")"));
    assert!(go.contains("func (w *Widget) SetInt32(value int32) bool {"));
    assert!(go.contains("func (w *Widget) SetBool(value bool) bool {"));
    assert!(go.contains("func (w *Widget) Set(args ...any) (bool, error) {"));
    assert!(go.contains("return w.SetInt32(arg0), nil"));
    assert!(go.contains("return w.SetBool(arg0), nil"));
    assert!(go.contains("fmt.Errorf(\"no matching overload for Widget.Set\""));
    assert!(go.contains("func (w *Widget) DescribeInt32(value int32) (string, error) {"));
    assert!(go.contains("func (w *Widget) DescribeBool(value bool) (string, error) {"));
    assert!(go.contains("func (w *Widget) Describe(args ...any) (string, error) {"));
    assert!(go.contains("return w.DescribeInt32(arg0)"));
    assert!(go.contains("return w.DescribeBool(arg0)"));
    assert!(go.contains("return \"\", fmt.Errorf(\"Widget receiver is nil\")"));
    assert!(go.contains("return \"\", fmt.Errorf(\"no matching overload for Widget.Describe\")"));
}

#[test]
fn expands_trailing_cxx_default_arguments_as_overload_variants() {
    let root = std::env::temp_dir().join(format!("c_go_default_args_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        class Widget {
        public:
            Widget(int size, bool owned = false) {}
            int set(int value, bool notify = false) { return value; }
        };

        int apply(int value, bool notify = false) { return value; }
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
            .any(|item| item.name == "cgowrap_apply__int")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_apply__int_bool")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_set__int_mut")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_set__int_bool_mut")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__int")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__int_bool")
    );

    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go_files = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go_files.len(), 1, "expected one Go facade file");
    let go = &go_files[0].contents;

    assert!(header.contains("int cgowrap_apply__int(int value);"));
    assert!(header.contains("int cgowrap_apply__int_bool(int value, bool notify);"));
    assert!(header.contains("WidgetHandle* cgowrap_Widget_new__int(int size);"));
    assert!(header.contains("WidgetHandle* cgowrap_Widget_new__int_bool(int size, bool owned);"));
    assert!(source.contains("return apply(value);"));
    assert!(source.contains("return apply(value, notify);"));
    assert!(
        source.contains("return reinterpret_cast<WidgetHandle*>(new Widget(size));"),
        "expected short constructor call in:\n{source}"
    );
    assert!(
        source.contains("return reinterpret_cast<WidgetHandle*>(new Widget(size, owned));"),
        "expected full constructor call in:\n{source}"
    );
    assert!(source.contains("return reinterpret_cast<Widget*>(self)->set(value);"));
    assert!(source.contains("return reinterpret_cast<Widget*>(self)->set(value, notify);"));

    assert!(go.contains("func ApplyInt32(value int32) int32 {"));
    assert!(go.contains("func ApplyInt32Bool(value int32, notify bool) int32 {"));
    assert!(go.contains("func Apply(args ...any) (int32, error) {"));
    assert!(go.contains("return ApplyInt32(arg0), nil"));
    assert!(go.contains("return ApplyInt32Bool(arg0, arg1), nil"));
    assert!(go.contains("func (w *Widget) SetInt32(value int32) int32 {"));
    assert!(go.contains("func (w *Widget) SetInt32Bool(value int32, notify bool) int32 {"));
    assert!(go.contains("func (w *Widget) Set(args ...any) (int32, error) {"));
    assert!(go.contains("return w.SetInt32(arg0), nil"));
    assert!(go.contains("return w.SetInt32Bool(arg0, arg1), nil"));
}

#[test]
fn skips_default_argument_variant_when_real_overload_already_owns_the_arity() {
    let root =
        std::env::temp_dir().join(format!("c_go_default_arg_collision_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/clash.hpp"),
        r#"
        int clash(int value) { return value; }
        int clash(int value, bool notify = false) { return notify ? value + 1 : value; }
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
    let names = ir
        .functions
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();

    assert!(
        names.contains(&"cgowrap_clash__int"),
        "expected real one-argument overload in {names:?}"
    );
    assert!(
        names.contains(&"cgowrap_clash__int_bool"),
        "expected two-argument overload in {names:?}"
    );
    assert!(
        !names.iter().any(|name| name.ends_with("_2")),
        "default variant should not create a duplicate suffix: {names:?}"
    );

    let source = render_source(&ctx, &ir);
    assert!(source.contains("int cgowrap_clash__int(int value)"));
    assert!(source.contains("int cgowrap_clash__int_bool(int value, bool notify)"));
    assert!(!source.contains("cgowrap_clash__int_2"));
}

#[test]
fn skips_dispatcher_for_model_ref_and_pointer_ambiguity() {
    let root = std::env::temp_dir().join(format!(
        "c_go_overload_dispatcher_ambiguous_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        class Model {
        public:
            Model() {}
        };

        class Widget {
        public:
            Widget() {}
            bool set(Model& value) { return true; }
            bool set(Model* value) { return value != 0; }
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

    assert!(go.contains("func (w *Widget) SetModelRef(value *Model) bool {"));
    assert!(go.contains("func (w *Widget) SetModelPtr(value *Model) bool {"));
    assert!(
        !go.contains("func (w *Widget) Set(args ...any)"),
        "ambiguous Model&/Model* overloads should not get a dispatcher:\n{go}"
    );
}
