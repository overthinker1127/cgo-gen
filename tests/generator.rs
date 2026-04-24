use cgo_gen::{
    config::Config,
    domain::kind::IrTypeKind,
    generator::{self, render_go_structs, render_header, render_source},
    ir, parser,
    pipeline::context::PipelineContext,
};
use std::{env, fs};

#[test]
fn renders_header_and_source_from_fixture() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let header = render_header(&ctx, &ir);
    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("fooBarHandle* cgowrap_foo_Bar_new(int value);"));
    assert!(header.contains("char* cgowrap_foo_Bar_name(const fooBarHandle* self);"));

    let source = render_source(&ctx, &ir);
    assert!(source.contains(&format!("#include \"{}\"", config.output.header)));
    assert!(source.contains("return reinterpret_cast<fooBarHandle*>(new foo::Bar(value));"));
    assert!(source.contains("delete reinterpret_cast<foo::Bar*>(self);"));
}

#[test]
fn renders_unified_go_wrapper() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let go = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type FooBar struct {"));
    assert!(
        go[0]
            .contents
            .contains("func Add(lhs int32, rhs int32) int32 {")
    );
}

#[test]
fn parsed_struct_pointers_use_handle_wrappers_while_foreign_structs_stay_direct() {
    let root = std::env::temp_dir().join(format!(
        "c_go_parsed_struct_pointer_routing_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include <sys/time.h>

        struct Counter {
            int value;
        };

        bool TakeCounter(struct Counter* value);
        bool TakeTimeval(struct timeval* value);
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

    let ctx = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let go = render_go_structs(&ctx, &ir).unwrap();

    let take_counter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeCounter")
        .unwrap();
    assert_eq!(take_counter.params[0].ty.kind, IrTypeKind::ModelPointer);
    assert_eq!(take_counter.params[0].ty.c_type, "CounterHandle*");

    let take_timeval = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeTimeval")
        .unwrap();
    assert_eq!(
        take_timeval.params[0].ty.kind,
        IrTypeKind::ExternStructPointer
    );
    assert_eq!(take_timeval.params[0].ty.c_type, "struct timeval*");

    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type Counter struct {"));
    assert!(
        go[0]
            .contents
            .contains("func TakeCounter(value *Counter) bool {")
    );
    assert!(
        !go[0]
            .contents
            .contains("func TakeCounter(value *C.struct_Counter) bool {")
    );
    assert!(
        go[0]
            .contents
            .contains("func TakeTimeval(value *C.struct_timeval) bool {")
    );
}

#[test]
fn preserves_const_char_spelling_but_normalizes_c_value_type() {
    let root = std::env::temp_dir().join(format!("c_go_const_char_value_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Api {
        public:
            const char GetMarker() const { return 'A'; }
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

    let config = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let marker = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetMarker")
        .unwrap();
    assert_eq!(marker.returns.cpp_type, "const char");
    assert_eq!(marker.returns.c_type, "char");
}

#[test]
fn renders_typedef_anonymous_enums_with_alias_name() {
    let root = std::env::temp_dir().join(format!("c_go_typedef_enum_alias_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef enum {
            FooDisabled = 0,
            FooEnabled = 1,
        } FooState;
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

    let config = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    let header = render_header(&config, &ir);
    let go = render_go_structs(&config, &ir).unwrap();

    assert!(parsed.enums.iter().any(|item| item.name == "FooState"));
    assert!(!header.contains("FooState"));
    assert!(go[0].contents.contains("type FooState int64"));
    assert!(go[0].contents.contains("FooDisabled FooState = 0"));
    assert!(go[0].contents.contains("FooEnabled FooState = 1"));
}

#[test]
fn renders_standalone_anonymous_enums_as_untyped_go_constants() {
    let root =
        std::env::temp_dir().join(format!("c_go_standalone_anon_enum_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        enum {
            FeatureDisabled = 0,
            FeatureEnabled = 1,
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

    let config = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    let header = render_header(&config, &ir);
    let go = render_go_structs(&config, &ir).unwrap();

    assert!(parsed.enums.iter().any(|item| item.is_anonymous));
    assert!(!header.contains("__anonymous_enum_"));
    assert!(go[0].contents.contains("FeatureDisabled = 0"));
    assert!(go[0].contents.contains("FeatureEnabled = 1"));
    assert!(!go[0].contents.contains("type __anonymous_enum_"));
    assert!(!go[0].contents.contains("FeatureDisabled __anonymous_enum_"));
    assert!(!go[0].contents.contains("int64"));
}

#[test]
fn renders_standalone_integer_macros_as_go_constants() {
    let root = std::env::temp_dir().join(format!("c_go_macro_constants_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        #define RTRK_BSRMETHOD_NOTDEFINE        (0)
        #define RTRK_BSRMETHOD_EWT              (10)
        #define RTRK_BSRMETHOD_WAITCNT          (20)
        #define TEST_INDEX                      10
        #define STARTUP_PENDING                 0x01
        #define MAKE_FLAG(value)                ((value) << 1)
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

    let ctx = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    assert!(
        parsed
            .macros
            .iter()
            .any(|item| item.name == "RTRK_BSRMETHOD_NOTDEFINE" && item.value == "0")
    );
    assert!(
        parsed
            .macros
            .iter()
            .any(|item| item.name == "RTRK_BSRMETHOD_EWT" && item.value == "10")
    );
    assert!(
        parsed
            .macros
            .iter()
            .any(|item| item.name == "STARTUP_PENDING" && item.value == "0x01")
    );
    assert!(
        parsed
            .macros
            .iter()
            .any(|item| item.name == "TEST_INDEX" && item.value == "10")
    );
    assert!(!parsed.macros.iter().any(|item| item.name == "MAKE_FLAG"));

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert!(
        ir.constants
            .iter()
            .any(|item| item.name == "RTRK_BSRMETHOD_WAITCNT" && item.value == "20")
    );
    assert!(
        ir.constants
            .iter()
            .any(|item| item.name == "TEST_INDEX" && item.value == "10")
    );

    let go = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go.len(), 1);
    let go_text = &go[0].contents;
    assert!(go_text.contains("const ("));
    assert!(go_text.contains("RTRK_BSRMETHOD_NOTDEFINE = 0"));
    assert!(go_text.contains("RTRK_BSRMETHOD_EWT = 10"));
    assert!(go_text.contains("RTRK_BSRMETHOD_WAITCNT = 20"));
    assert!(go_text.contains("TEST_INDEX = 10"));
    assert!(go_text.contains("STARTUP_PENDING = 0x01"));
    assert!(!go_text.contains("MAKE_FLAG"));
    assert!(!go_text.contains("import \"C\""));
}

#[test]
fn renders_typedef_enum_alias_method_params_as_value_enums() {
    let root =
        std::env::temp_dir().join(format!("c_go_typedef_enum_method_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef enum _State {
            StateDisabled = 0,
            StateEnabled = 1,
        } State;

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool UseState(State value) const { return value == StateEnabled; }
            State EchoState(State value) const { return value; }
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

    let ctx = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let use_state = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::UseState")
        .unwrap();
    let echo_state = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::EchoState")
        .unwrap();

    assert_eq!(use_state.params[1].ty.kind, IrTypeKind::Enum);
    assert_eq!(echo_state.returns.kind, IrTypeKind::Enum);
    assert!(header.contains("bool cgowrap_Api_UseState(const ApiHandle* self, int64_t value);"));
    assert!(
        header.contains("int64_t cgowrap_Api_EchoState(const ApiHandle* self, int64_t value);")
    );
    assert!(!header.contains("StateHandle"));
    assert!(!source.contains("StateHandle"));
    assert!(
        source.contains("UseState(static_cast<_State>(value))")
            || source.contains("UseState(static_cast<enum _State>(value))")
    );
    assert!(
        source.contains("return static_cast<int64_t>(reinterpret_cast<const Api*>(self)->EchoState(static_cast<_State>(value)));")
            || source.contains("return static_cast<int64_t>(reinterpret_cast<const Api*>(self)->EchoState(static_cast<enum _State>(value)));")
    );

    assert!(go_text.contains("type _State int64"));
    assert!(go_text.contains("func (a *Api) UseState(value _State) bool {"));
    assert!(go_text.contains("result := C.cgowrap_Api_UseState(a.ptr, C.int64_t(value))"));
    assert!(go_text.contains("func (a *Api) EchoState(value _State) _State {"));
    assert!(go_text.contains("return _State(C.cgowrap_Api_EchoState(a.ptr, C.int64_t(value)))"));
    assert!(!go_text.contains("type State struct {"));
}

#[test]
fn normalizes_primitive_alias_pointer_and_reference_c_types_in_header() {
    let root =
        std::env::temp_dir().join(format!("c_go_alias_pointer_header_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include <stdint.h>
        typedef int32_t int32;
        typedef uint32_t uint32;

        bool TakeAliasPtr(int32* value);
        bool TakeAliasRef(uint32& value);
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
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);

    let ptr = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasPtr")
        .unwrap();
    assert_eq!(ptr.params[0].ty.cpp_type, "int32*");
    assert_eq!(ptr.params[0].ty.c_type, "int32_t*");

    let r#ref = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasRef")
        .unwrap();
    assert_eq!(r#ref.params[0].ty.cpp_type, "uint32&");
    assert_eq!(r#ref.params[0].ty.c_type, "uint32_t*");

    assert!(header.contains("bool cgowrap_TakeAliasPtr(int32_t* value);"));
    assert!(header.contains("bool cgowrap_TakeAliasRef(uint32_t* value);"));
    assert!(!header.contains("int32* value"));
    assert!(!header.contains("uint32* value"));
}

#[test]
fn generate_with_go_module_writes_build_flags_and_go_mod() {
    let root = env::temp_dir().join(format!("c_go_go_package_metadata_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(root.join("include/Api.hpp"), "int Add(int lhs, int rhs);").unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
  clang_args:
    - -I${SDK_INCLUDE}
    - -isystem
    - system/include
    - -isysteminline/include
    - -DMODE=1
    - -std=c++20
    - -Wall
    - -Winvalid-offsetof
output:
  dir: out
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("SDK_INCLUDE", root.join("sdk/include"));
    }

    let config = PipelineContext::from_config_path(root.join("config.yaml"))
        .unwrap()
        .with_go_module(Some("example.com/demo/pkg".to_string()));
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    cgo_gen::generator::generate(&config, &ir, false, &Default::default()).unwrap();

    let go_mod = fs::read_to_string(root.join("out/go.mod")).unwrap();
    assert_eq!(go_mod, "module example.com/demo/pkg\n\ngo 1.25\n");

    let build_flags = fs::read_to_string(root.join("out/build_flags.go")).unwrap();
    assert!(build_flags.contains("package out"));
    assert!(build_flags.contains("#cgo CFLAGS: -I${SRCDIR}"));
    assert!(build_flags.contains(&format!(
        "#cgo CXXFLAGS: -I${{SRCDIR}} -I{} -DMODE=1 -std=c++20",
        root.join("sdk/include").display()
    )));
    assert!(!build_flags.contains("-isystem"));
    assert!(!build_flags.contains("system/include"));
    assert!(!build_flags.contains("inline/include"));
    assert!(!build_flags.contains("-Winvalid-offsetof"));
    assert!(!build_flags.contains("-Wall"));
}

#[test]
fn struct_fields_generate_synthetic_accessors() {
    let root = env::temp_dir().join(format!(
        "c_go_struct_field_accessors_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Counter.hpp"),
        r#"
        #include <stdint.h>

        struct Counter {
            int value;
            uint32_t total_count;
            const int read_only = 7;
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();

    assert!(header.contains("int cgowrap_Counter_GetValue(const CounterHandle* self);"));
    assert!(header.contains("void cgowrap_Counter_SetValue(CounterHandle* self, int value);"));
    assert!(
        header.contains("unsigned int cgowrap_Counter_GetTotalCount(const CounterHandle* self);")
    );
    assert!(
        header.contains(
            "void cgowrap_Counter_SetTotalCount(CounterHandle* self, unsigned int value);"
        )
    );
    assert!(header.contains("int cgowrap_Counter_GetReadOnly(const CounterHandle* self);"));
    assert!(!header.contains("cgowrap_Counter_SetReadOnly"));

    assert!(source.contains("return reinterpret_cast<const Counter*>(self)->value;"));
    assert!(source.contains("reinterpret_cast<Counter*>(self)->value = value;"));
    assert!(source.contains("return reinterpret_cast<const Counter*>(self)->total_count;"));
    assert!(source.contains("reinterpret_cast<Counter*>(self)->total_count = value;"));

    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type Counter struct {"));
    assert!(
        go[0]
            .contents
            .contains("func (c *Counter) GetValue() int32 {")
    );
    assert!(
        go[0]
            .contents
            .contains("func (c *Counter) SetValue(value int32) {")
    );
    assert!(
        go[0]
            .contents
            .contains("func (c *Counter) GetTotalCount() uint32 {")
    );
    assert!(
        go[0]
            .contents
            .contains("func (c *Counter) SetTotalCount(value uint32) {")
    );
    assert!(!go[0].contents.contains("SetReadOnly("));
}

#[test]
fn struct_fixed_model_array_fields_render_element_type_in_source() {
    let root = env::temp_dir().join(format!(
        "c_go_fixed_model_array_fields_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Holder.hpp"),
        r#"
        struct Item {
            int value;
        };

        struct Holder {
            Item items[3];
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = &go[0].contents;

    assert!(header.contains("ItemHandle** cgowrap_Holder_GetItems(const HolderHandle* self);"));
    assert!(
        header.contains(
            "ItemHandle* cgowrap_Holder_GetItemsAt(const HolderHandle* self, int index);"
        )
    );
    assert!(
        header.contains("void cgowrap_Holder_SetItems(HolderHandle* self, ItemHandle** value);")
    );
    assert!(header.contains(
        "void cgowrap_Holder_SetItemsAt(HolderHandle* self, int index, ItemHandle* value);"
    ));

    assert!(source.contains(
        "_r[_i] = reinterpret_cast<ItemHandle*>(const_cast<Item*>(&reinterpret_cast<const Holder*>(self)->items[_i]));"
    ));
    assert!(source.contains(
        "if (self == nullptr || index < 0 || index >= 3) {\n        return nullptr;\n    }\n    return reinterpret_cast<ItemHandle*>(const_cast<Item*>(&reinterpret_cast<const Holder*>(self)->items[index]));"
    ));
    assert!(source.contains(
        "reinterpret_cast<Holder*>(self)->items[_i] = *reinterpret_cast<Item*>(value[_i]);"
    ));
    assert!(source.contains(
        "if (self == nullptr || value == nullptr || index < 0 || index >= 3) {\n        return;\n    }\n    reinterpret_cast<Holder*>(self)->items[index] = *reinterpret_cast<Item*>(value);"
    ));
    assert!(!source.contains("new Item(reinterpret_cast<const Holder*>(self)->items[_i])"));
    assert!(!source.contains("reinterpret_cast<Item[3]*>(value[_i])"));
    assert!(go_text.contains("func (h *Holder) GetItems() ([]*Item, error) {"));
    assert!(go_text.contains("result[i] = h.GetItemsAt(i)"));
    assert!(go_text.contains("func (h *Holder) GetItemsAt(index int) *Item {"));
    assert!(go_text.contains("func (h *Holder) SetItemsAt(index int, value *Item) {"));
    assert!(go_text.contains("if len(value) != 3 {"));
    assert!(go_text.contains("h.SetItemsAt(i, value[i])"));
}

#[test]
fn renders_model_value_return_as_owned_handle_copy() {
    let root = env::temp_dir().join(format!("c_go_model_pointer_return_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class MTime {
        public:
            MTime() : value_(7) {}
            int GetValue() const { return value_; }
        private:
            int value_;
        };

        class Api {
        public:
            MTime GetCreateTime() const { return MTime(); }
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);

    let getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetCreateTime")
        .unwrap();
    assert_eq!(getter.returns.kind, IrTypeKind::ModelValue);
    assert_eq!(getter.returns.c_type, "MTimeHandle*");
    assert!(header.contains("MTimeHandle* cgowrap_Api_GetCreateTime(const ApiHandle* self);"));
    assert!(source.contains(
        "return reinterpret_cast<MTimeHandle*>(new MTime(reinterpret_cast<const Api*>(self)->GetCreateTime()));"
    ));
}

#[test]
fn opaque_model_value_return_gets_synthetic_delete_and_owned_go_wrapper() {
    let ctx = PipelineContext::new(Config::default());
    let self_param = ir::IrParam {
        name: "self".to_string(),
        ty: ir::IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: "Api*".to_string(),
            c_type: "ApiHandle*".to_string(),
            handle: Some("ApiHandle".to_string()),
        },
    };
    let ir = ir::IrModule {
        version: 1,
        module: "cgowrap".to_string(),
        source_headers: vec![],
        records: vec![],
        opaque_types: vec![
            ir::OpaqueType {
                name: "ApiHandle".to_string(),
                cpp_type: "Api".to_string(),
            },
            ir::OpaqueType {
                name: "TableiKeyHandle".to_string(),
                cpp_type: "Table::iKey".to_string(),
            },
        ],
        functions: vec![
            ir::IrFunction {
                name: "cgowrap_Api_new".to_string(),
                kind: ir::IrFunctionKind::Constructor,
                cpp_name: "Api".to_string(),
                method_of: Some("ApiHandle".to_string()),
                owner_cpp_type: Some("Api".to_string()),
                is_const: None,
                field_accessor: None,
                returns: ir::IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: "Api*".to_string(),
                    c_type: "ApiHandle*".to_string(),
                    handle: Some("ApiHandle".to_string()),
                },
                params: vec![],
            },
            ir::IrFunction {
                name: "cgowrap_Api_delete".to_string(),
                kind: ir::IrFunctionKind::Destructor,
                cpp_name: "~Api".to_string(),
                method_of: Some("ApiHandle".to_string()),
                owner_cpp_type: Some("Api".to_string()),
                is_const: None,
                field_accessor: None,
                returns: ir::IrType {
                    kind: IrTypeKind::Void,
                    cpp_type: "void".to_string(),
                    c_type: "void".to_string(),
                    handle: None,
                },
                params: vec![self_param.clone()],
            },
            ir::IrFunction {
                name: "cgowrap_Api_GetKey".to_string(),
                kind: ir::IrFunctionKind::Method,
                cpp_name: "Api::GetKey".to_string(),
                method_of: Some("ApiHandle".to_string()),
                owner_cpp_type: Some("Api".to_string()),
                is_const: Some(true),
                field_accessor: None,
                returns: ir::IrType {
                    kind: IrTypeKind::ModelValue,
                    cpp_type: "Table::iKey".to_string(),
                    c_type: "TableiKeyHandle*".to_string(),
                    handle: Some("TableiKeyHandle".to_string()),
                },
                params: vec![self_param],
            },
        ],
        enums: vec![],
        constants: vec![],
        callbacks: vec![],
        support: ir::SupportMetadata {
            parser_backend: "test".to_string(),
            notes: vec![],
            skipped_declarations: vec![],
        },
    };

    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(header.contains("void cgowrap_TableiKey_delete(TableiKeyHandle* self);"));
    assert!(source.contains("delete reinterpret_cast<Table::iKey*>(self);"));
    assert!(go_text.contains(
        "type TableiKey struct {\n    ptr *C.TableiKeyHandle\n    owned bool\n    root *bool\n}"
    ));
    assert!(go_text.contains("func (t *TableiKey) Close() {"));
    assert!(go_text.contains("return newOwnedTableiKey(raw)"));
}

#[test]
fn renders_model_pointer_and_reference_returns_as_borrowed_handles() {
    let root = env::temp_dir().join(format!("c_go_model_value_return_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        struct Child {
            int value;
        };

        class Api {
        public:
            Child* GetChildPtr() { return &child_; }
            Child& GetChildRef() { return child_; }
        private:
            Child child_;
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let source = render_source(&ctx, &ir);

    let ptr_getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetChildPtr")
        .unwrap();
    let ref_getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetChildRef")
        .unwrap();
    assert_eq!(ptr_getter.returns.kind, IrTypeKind::ModelPointer);
    assert_eq!(ref_getter.returns.kind, IrTypeKind::ModelReference);
    assert!(source.contains(
        "return reinterpret_cast<ChildHandle*>(reinterpret_cast<Api*>(self)->GetChildPtr());"
    ));
    assert!(source.contains(
        "return reinterpret_cast<ChildHandle*>(&reinterpret_cast<Api*>(self)->GetChildRef());"
    ));
}

#[test]
fn renders_model_value_field_accessors_as_borrowed_get_and_explicit_set() {
    let root = env::temp_dir().join(format!("c_go_model_value_field_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Models.hpp"),
        r#"
        struct Child {
            int value;
        };

        struct Parent {
            Child child;
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(header.contains("ChildHandle* cgowrap_Parent_GetChild(ParentHandle* self);"));
    assert!(
        header.contains("void cgowrap_Parent_SetChild(ParentHandle* self, ChildHandle* value);")
    );
    assert!(source.contains(
        "return reinterpret_cast<ChildHandle*>(&reinterpret_cast<Parent*>(self)->child);"
    ));
    assert!(
        source
            .contains("reinterpret_cast<Parent*>(self)->child = *reinterpret_cast<Child*>(value);")
    );
    assert!(go_text.contains("func (p *Parent) GetChild() *Child {"));
    assert!(go_text.contains("func (p *Parent) SetChild(value *Child) {"));
}

#[test]
fn renders_go_fixed_array_typedef_aliases_with_canonical_unsigned_types() {
    let root = env::temp_dir().join(format!(
        "c_go_reason_subscription_fixed_array_typedef_alias_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef unsigned int ReasonCode;
        typedef unsigned int SubscribeId;

        struct Info {
            ReasonCode codes[4];
            SubscribeId ids[4];
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(header.contains("unsigned int* cgowrap_Info_GetCodes(const InfoHandle* self);"));
    assert!(header.contains("void cgowrap_Info_SetCodes(InfoHandle* self, unsigned int* value);"));
    assert!(header.contains("unsigned int* cgowrap_Info_GetIds(const InfoHandle* self);"));
    assert!(header.contains("void cgowrap_Info_SetIds(InfoHandle* self, unsigned int* value);"));

    assert!(go_text.contains("func (i *Info) GetCodes() ([]uint32, error) {"));
    assert!(go_text.contains("func (i *Info) SetCodes(value []uint32) {"));
    assert!(go_text.contains("func (i *Info) GetIds() ([]uint32, error) {"));
    assert!(go_text.contains("func (i *Info) SetIds(value []uint32) {"));
    assert!(go_text.contains("cSlice := (*[4]C.uint32_t)(unsafe.Pointer(raw))"));
    assert!(go_text.contains("(*C.uint32_t)(unsafe.Pointer(&value[0]))"));
    assert!(go_text.contains("result := make([]uint32, 4)"));
}

#[test]
fn renders_reason_and_subscription_fixed_arrays_as_uint32_slices() {
    let root = env::temp_dir().join(format!(
        "c_go_fixed_array_typedef_alias_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef unsigned int ReasonCode;
        typedef unsigned int SubscriptionId;
        #define MAX_SUBSCRIPTION_IDS 16

        struct StatusInfo {
            ReasonCode PrimaryReasonCodes[64];
            ReasonCode SecondaryReasonCodes[64];
        };

        struct SubscriptionCodes {
            SubscriptionId SubscriptionIds[MAX_SUBSCRIPTION_IDS];
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(header.contains(
        "unsigned int* cgowrap_StatusInfo_GetPrimaryReasonCodes(const StatusInfoHandle* self);"
    ));
    assert!(header.contains(
        "void cgowrap_StatusInfo_SetPrimaryReasonCodes(StatusInfoHandle* self, unsigned int* value);"
    ));
    assert!(header.contains(
        "unsigned int* cgowrap_StatusInfo_GetSecondaryReasonCodes(const StatusInfoHandle* self);"
    ));
    assert!(header.contains(
        "void cgowrap_StatusInfo_SetSecondaryReasonCodes(StatusInfoHandle* self, unsigned int* value);"
    ));
    assert!(header.contains(
        "unsigned int* cgowrap_SubscriptionCodes_GetSubscriptionIds(const SubscriptionCodesHandle* self);"
    ));
    assert!(header.contains(
        "void cgowrap_SubscriptionCodes_SetSubscriptionIds(SubscriptionCodesHandle* self, unsigned int* value);"
    ));

    assert!(go_text.contains("func (s *StatusInfo) GetPrimaryReasonCodes() ([]uint32, error) {"));
    assert!(go_text.contains("func (s *StatusInfo) SetPrimaryReasonCodes(value []uint32) {"));
    assert!(go_text.contains("func (s *StatusInfo) GetSecondaryReasonCodes() ([]uint32, error) {"));
    assert!(go_text.contains("func (s *StatusInfo) SetSecondaryReasonCodes(value []uint32) {"));
    assert!(
        go_text.contains("func (s *SubscriptionCodes) GetSubscriptionIds() ([]uint32, error) {")
    );
    assert!(go_text.contains("func (s *SubscriptionCodes) SetSubscriptionIds(value []uint32) {"));
    assert!(go_text.contains("cSlice := (*[64]C.uint32_t)(unsafe.Pointer(raw))"));
    assert!(go_text.contains("cSlice := (*[16]C.uint32_t)(unsafe.Pointer(raw))"));
    assert!(go_text.contains("(*C.uint32_t)(unsafe.Pointer(&value[0]))"));
}

#[test]
fn avoids_false_bool_suffix_for_underscore_backed_field_setters_but_keeps_real_overloads() {
    let root = env::temp_dir().join(format!(
        "c_go_false_overload_suffix_detection_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        struct _SYS_IF_MONITOR_IODSM {
            bool bModifyFlag;
        };

        class Api {
        public:
            void SetFlag(bool value) {}
            void SetFlag(int value) {}
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(go_text.contains("func (s *SYSIFMONITORIODSM) SetBModifyFlag(value bool) {"));
    assert!(!go_text.contains("SetBModifyFlagBool("));
    assert!(go_text.contains("func (a *Api) SetFlagBool(value bool) {"));
    assert!(go_text.contains("func (a *Api) SetFlagInt32(value int32) {"));
}
