use std::{env, fs};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_facade_test_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn generates_go_facade_for_simple_free_function_header() {
    let mut config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    config.output.dir = temp_output_dir("generate");

    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let go_facade = fs::read_to_string(config.output_dir().join(config.go_filename(""))).unwrap();

    assert!(go_facade.contains("import \"C\""));
    assert!(go_facade.contains(&format!(
        "#include \"{}\"",
        config.generated_header_include(&config.output.header)
    )));
    assert!(go_facade.contains("func Add(lhs int32, rhs int32) int32 {"));
    assert!(go_facade.contains("C.cgowrap_foo_add(C.int(lhs), C.int(rhs))"));
}

#[test]
fn generates_go_facade_for_bool_and_string_returns() {
    let root = temp_output_dir("bool-string");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once
        #include <string>

        bool is_ready();
        std::string version();
        const char* banner();
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let go_facade = fs::read_to_string(config.output_dir().join(config.go_filename(""))).unwrap();

    assert!(go_facade.contains("import \"errors\""));
    assert!(go_facade.contains("func IsReady() bool {"));
    assert!(go_facade.contains("result := C.cgowrap_is_ready()"));
    assert!(go_facade.contains("return bool(result)"));
    assert!(go_facade.contains("func Version() (string, error) {"));
    assert!(go_facade.contains("raw := C.cgowrap_version()"));
    assert!(go_facade.contains("defer C.cgowrap_string_free(raw)"));
    assert!(go_facade.contains("return C.GoString(raw), nil"));
    assert!(go_facade.contains("func Banner() (string, error) {"));
    assert!(go_facade.contains("raw := C.cgowrap_banner()"));
    let banner_section = go_facade
        .split("func Banner() (string, error) {")
        .nth(1)
        .unwrap();
    let banner_body = banner_section.split("}\n").next().unwrap();
    assert!(!banner_body.contains("string_free"));
}

#[test]
fn rejects_namespaced_facade_functions_that_collide_in_go_exports() {
    let root = temp_output_dir("namespace-collision");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once

        namespace alpha { int init(); }
        namespace beta { int init(); }
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let error = generator::generate(&ctx, &ir, true, &Default::default())
        .unwrap_err()
        .to_string();

    assert!(error.contains("facade export collision"));
}

#[test]
fn preserves_numeric_suffix_underscores_in_go_method_names() {
    let root = temp_output_dir("numeric-underscore");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Media.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once

        class IsMediaDelivery {
        public:
            IsMediaDelivery() = default;
            ~IsMediaDelivery() = default;
            int GetFailCnt_1() const;
            int GetFailCnt1() const;
            void SetFailCnt_1(int value);
            void SetFailCnt1(int value);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let go_facade = fs::read_to_string(root.join("out/media_wrapper.go")).unwrap();

    assert!(go_facade.contains("func (i *IsMediaDelivery) GetFailCnt_1() int32"));
    assert!(go_facade.contains("func (i *IsMediaDelivery) GetFailCnt1() int32"));
    assert!(go_facade.contains("func (i *IsMediaDelivery) SetFailCnt_1(value int32)"));
    assert!(go_facade.contains("func (i *IsMediaDelivery) SetFailCnt1(value int32)"));
}

#[test]
fn supports_facade_classes_with_object_pointer_constructor_params() {
    let root = temp_output_dir("unsupported-constructor");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once

        class NsLeg;

        class NsLeg {
        public:
            NsLeg(NsLeg* parent);
            ~NsLeg();
            int GetValue() const;
        };

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int GetValue() const;
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type NsLeg struct {"));
    assert!(go_facade.contains("func NewNsLeg(parent *NsLeg) (*NsLeg, error) {"));
    assert!(go_facade.contains("var cArg0 *C.NsLegHandle"));
    assert!(go_facade.contains("if parent != nil {"));
    assert!(go_facade.contains("cArg0 = parent.ptr"));
    assert!(go_facade.contains("func (n *NsLeg) GetValue() int32 {"));
    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("func (a *Api) GetValue() int32 {"));
}

#[test]
fn supports_facade_classes_with_object_reference_constructor_params() {
    let root = temp_output_dir("unsupported-constructor-ref");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once

        class NsLeg;

        class NsLeg {
        public:
            NsLeg(NsLeg& parent);
            ~NsLeg();
            int GetValue() const;
        };

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int GetValue() const;
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type NsLeg struct {"));
    assert!(go_facade.contains("func NewNsLeg(parent *NsLeg) (*NsLeg, error) {"));
    assert!(go_facade.contains("if parent == nil {"));
    assert!(go_facade.contains("panic(\"reference facade/model argument cannot be nil\")"));
    assert!(go_facade.contains("cArg0 = parent.ptr"));
    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("func (a *Api) GetValue() int32 {"));
}

#[test]
fn exposes_object_out_params_as_direct_wrapper_pointer_arguments() {
    let root = temp_output_dir("model-method");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            int Clear();
            bool GetThing(int id, ThingModel& out);
            bool GetThingByKey(const char* key, ThingModel* out);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("ptr *C.ApiHandle"));
    assert!(go_facade.contains("func NewApi() (*Api, error) {"));
    assert!(go_facade.contains("C.cgowrap_Api_new()"));
    assert!(go_facade.contains("func (a *Api) Close() {"));
    assert!(go_facade.contains("C.cgowrap_Api_delete(a.ptr)"));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(go_facade.contains("result := C.cgowrap_Api_IsReady(a.ptr)"));
    assert!(go_facade.contains("return bool(result)"));
    assert!(go_facade.contains("func (a *Api) Clear() int32 {"));
    assert!(go_facade.contains("return int32(C.cgowrap_Api_Clear(a.ptr))"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int32, out *ThingModel) bool {"));
    assert!(go_facade.contains("requireThingModelHandle(out)"));
    assert!(
        go_facade
            .contains("C.cgowrap_Api_GetThing(a.ptr, C.int(id), requireThingModelHandle(out))")
    );
    assert!(go_facade.contains("func (a *Api) GetThingByKey(key string, out *ThingModel) bool {"));
    assert!(go_facade.contains("cArg0 := C.CString(key)"));
    assert!(go_facade.contains("defer C.free(unsafe.Pointer(cArg0))"));
    assert!(go_facade.contains("optionalThingModelHandle(out)"));
    assert!(
        go_facade
            .contains("C.cgowrap_Api_GetThingByKey(a.ptr, cArg0, optionalThingModelHandle(out))")
    );
    assert!(!go_facade.contains("mapThingModelFromHandle"));
}

#[test]
fn keeps_unknown_model_refs_in_raw_wrappers_but_filters_them_from_go_facade() {
    let root = temp_output_dir("unknown-model-raw-first");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("UnknownThing.hpp"),
        r#"
        class UnknownThing {
        public:
            UnknownThing() = default;
            ~UnknownThing() = default;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"
        #include "UnknownThing.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int Count() const;
            bool GetThing(int id, ThingModel& out);
            bool GetUnknown(int id, UnknownThing& out);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let ir_yaml = fs::read_to_string(root.join("out/api_wrapper.ir.yaml")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(raw_header.contains("typedef struct UnknownThingHandle UnknownThingHandle;"));
    assert!(raw_header.contains(
        "bool cgowrap_Api_GetUnknown(ApiHandle* self, int id, UnknownThingHandle* out);"
    ));
    assert!(raw_source.contains("cgowrap_Api_GetUnknown"));
    assert!(raw_source.contains("*reinterpret_cast<UnknownThing*>(out)"));
    assert!(ir_yaml.contains("cpp_name: Api::GetUnknown"));
    assert!(go_facade.contains("func (a *Api) Count() int32 {"));
    assert!(go_facade.contains("return int32(C.cgowrap_Api_Count(a.ptr))"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int32, out *ThingModel) bool {"));
    assert!(go_facade.contains("func (a *Api) GetUnknown(id int32, out *UnknownThing) bool {"));
}

#[test]
fn supports_by_value_internal_types_without_aborting_supported_facade_output() {
    let root = temp_output_dir("unknown-model-by-value-support");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("UnknownThing.hpp"),
        r#"
        class UnknownThing {
        public:
            UnknownThing() = default;
            ~UnknownThing() = default;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"
        #include "UnknownThing.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int Count() const;
            bool GetThing(int id, ThingModel& out);
            bool SaveUnknown(UnknownThing value);
            UnknownThing BuildUnknown() const;
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let prepared =
        generator::prepare_config(&PipelineContext::new(Config::load(&config_path).unwrap()))
            .unwrap();
    let facade_header = prepared
        .discovered_headers()
        .unwrap()
        .into_iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("Api.hpp"))
        .unwrap();
    let config = prepared.scoped_to_header(facade_header);
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true, &Default::default()).unwrap();

    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let ir_yaml = fs::read_to_string(root.join("out/api_wrapper.ir.yaml")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(
        raw_header
            .contains("bool cgowrap_Api_GetThing(ApiHandle* self, int id, ThingModelHandle* out);")
    );
    assert!(raw_source.contains("cgowrap_Api_GetThing"));
    assert!(
        raw_header
            .contains("bool cgowrap_Api_SaveUnknown(ApiHandle* self, UnknownThingHandle* value);")
    );
    assert!(raw_source.contains("cgowrap_Api_SaveUnknown"));
    assert!(raw_source.contains("*reinterpret_cast<UnknownThing*>(value)"));
    assert!(
        !ir.support
            .skipped_declarations
            .iter()
            .any(|item| { item.cpp_name == "Api::SaveUnknown" })
    );
    assert!(ir_yaml.contains("cpp_name: Api::SaveUnknown"));
    assert!(ir_yaml.contains("cpp_name: Api::BuildUnknown"));
    assert!(go_facade.contains("func (a *Api) Count() int32 {"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int32, out *ThingModel) bool {"));
    assert!(go_facade.contains("func (a *Api) SaveUnknown(value *UnknownThing) bool {"));
    assert!(go_facade.contains("func (a *Api) BuildUnknown() *UnknownThing {"));
}

#[test]
fn keeps_non_model_methods_on_general_api_path_even_if_names_look_like_lookup_apis() {
    let root = temp_output_dir("general-api-routing");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool ListThing(int id) const;
            int NextThing(int cursor);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("func (a *Api) ListThing(id int32) bool {"));
    assert!(go_facade.contains("result := C.cgowrap_Api_ListThing(a.ptr, C.int(id))"));
    assert!(go_facade.contains("return bool(result)"));
    assert!(go_facade.contains("func (a *Api) NextThing(cursor int32) int32 {"));
    assert!(!go_facade.contains("mapThingModelFromHandle"));
}

#[test]
fn exposes_model_pointer_and_reference_returns_as_borrowed_wrappers_in_go_facade() {
    let root = temp_output_dir("model-view-return");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            ThingModel* GetThingPtr();
            ThingModel& GetThingRef();
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(raw_source.contains(
        "return reinterpret_cast<ThingModelHandle*>(reinterpret_cast<Api*>(self)->GetThingPtr());"
    ));
    assert!(raw_source.contains(
        "return reinterpret_cast<ThingModelHandle*>(&reinterpret_cast<Api*>(self)->GetThingRef());"
    ));
    assert!(go_facade.contains("func (a *Api) GetThingPtr() *ThingModel {"));
    assert!(go_facade.contains("func (a *Api) GetThingRef() *ThingModel {"));
    assert!(go_facade.contains("return newBorrowedThingModel(raw, a.root)"));
}

#[test]
fn renders_owner_marked_model_pointer_returns_as_owned_in_go_facade() {
    let root = temp_output_dir("owner-marked-model-pointer-return");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Factory.hpp"),
        r#"
        class DBHandler {
        public:
            virtual ~DBHandler() = default;
            int GetValue() const { return 7; }
            virtual void ProcDml() = 0;
        };

        class ConcreteHandler : public DBHandler {
        public:
            void ProcDml() override {}
        };

        class DBHandlerFactory {
        public:
            DBHandlerFactory() = default;
            ~DBHandlerFactory() = default;
            DBHandler* CreateHandler() { return new ConcreteHandler(); }
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  owner:
    - DBHandlerFactory::CreateHandler
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let raw_source = fs::read_to_string(root.join("out/factory_wrapper.cpp")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/factory_wrapper.go")).unwrap();

    assert!(raw_source.contains(
        "return reinterpret_cast<DBHandlerHandle*>(reinterpret_cast<DBHandlerFactory*>(self)->CreateHandler());"
    ));
    assert!(go_facade.contains("func (d *DBHandlerFactory) CreateHandler() *DBHandler {"));
    assert!(go_facade.contains("return &DBHandler{ptr: raw, owned: true, root: new(bool)}"));
    assert!(!go_facade.contains("return newBorrowedDBHandler(raw, d.root)"));
}

#[test]
fn renders_const_model_borrow_returns_in_go_facade() {
    let root = temp_output_dir("const-model-borrow-raw-only");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            const ThingModel& GetThing() const;
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(
        raw_header.contains("const ThingModelHandle* cgowrap_Api_GetThing(const ApiHandle* self);")
    );
    assert!(raw_source.contains(
        "return reinterpret_cast<const ThingModelHandle*>(&reinterpret_cast<const Api*>(self)->GetThing());"
    ));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(go_facade.contains("func (a *Api) GetThing() *ThingModel {"));
    assert!(go_facade.contains("return newBorrowedThingModel(raw, a.root)"));
}

#[test]
fn supports_object_reference_params_even_outside_last_position() {
    let root = temp_output_dir("model-not-last");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            bool GetThing(ThingModel& out, int id);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(go_facade.contains("func (a *Api) GetThing(out *ThingModel, id int32) bool {"));
    assert!(!go_facade.contains("mapThingModelFromHandle"));
}

#[test]
fn renders_next_style_methods_with_reference_cursor_and_handle_backed_model_out_param() {
    let root = temp_output_dir("next-style-method");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("WebhookRecord.hpp"),
        r#"
        class WebhookRecord {
        public:
            WebhookRecord() = default;
            ~WebhookRecord() = default;
            const char* GetUrl() const;
            void SetUrl(const char* value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("ApiClient.hpp"),
        r#"
        #include "WebhookRecord.hpp"
        #include <stdint.h>

        class ApiClient {
        public:
            ApiClient() = default;
            ~ApiClient() = default;
            bool NextWebhook(int32_t& pos, WebhookRecord& out);
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_model = fs::read_to_string(root.join("out/webhook_record_wrapper.go")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_client_wrapper.go")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_client_wrapper.cpp")).unwrap();

    assert!(go_model.contains("type WebhookRecord struct {"));
    assert!(go_model.contains("ptr *C.WebhookRecordHandle"));
    assert!(go_model.contains("func (w *WebhookRecord) SetUrl(value string) {"));

    assert!(
        go_facade
            .contains("func (a *ApiClient) NextWebhook(pos *int32, out *WebhookRecord) bool {")
    );
    assert!(go_facade.contains("if pos == nil {"));
    assert!(go_facade.contains("panic(\"pos reference is nil\")"));
    assert!(go_facade.contains("cArg0 := C.int32_t(*pos)"));
    assert!(go_facade.contains(
        "result := C.cgowrap_ApiClient_NextWebhook(a.ptr, &cArg0, requireWebhookRecordHandle(out))"
    ));
    assert!(go_facade.contains("*pos = int32(cArg0)"));
    assert!(go_facade.contains("return bool(result)"));

    assert!(raw_source.contains(
        "return reinterpret_cast<ApiClient*>(self)->NextWebhook(*pos, *reinterpret_cast<WebhookRecord*>(out));"
    ));
}

#[test]
fn generates_callback_typedefs_and_facade_bridge_helpers() {
    let root = temp_output_dir("callback-facade");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once
        #include <stdint.h>

        typedef uint32_t AppId;
        typedef const char* AppString;
        typedef int32_t int32;
        typedef void (*EventCallback)(AppId appId, uint32_t eventId, AppString data, int32 size);

        void SetEventCallback(EventCallback cb);
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let ir_dump = fs::read_to_string(root.join("out/api_wrapper.ir.yaml")).unwrap();
    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(ir_dump.contains("callbacks:"));
    assert!(ir_dump.contains("name: EventCallback"));
    assert!(raw_header.contains(
        "typedef void (*EventCallback)(unsigned int appId, unsigned int eventId, const char* data, int32_t size);"
    ));
    assert!(raw_header.contains("void cgowrap_SetEventCallback_bridge(bool use_cb0);"));

    assert!(raw_source.contains("extern \"C\" {"));
    assert!(raw_source.contains("void go_cgowrap_SetEventCallback_cb0("));
    assert!(raw_source.contains("EventCallback cgowrap_SetEventCallback_cb0_trampoline"));
    assert!(raw_source.contains(
        "cgowrap_SetEventCallback(use_cb0 ? cgowrap_SetEventCallback_cb0_trampoline : nullptr);"
    ));

    assert!(go_facade.contains("import \"sync\""));
    assert!(go_facade.contains(
        "type EventCallback func(appId uint32, eventId uint32, data string, size int32)"
    ));
    assert!(go_facade.contains("var cgowrap_SetEventCallback_cb0 struct {"));
    assert!(go_facade.contains("//export go_cgowrap_SetEventCallback_cb0"));
    assert!(go_facade.contains("func SetEventCallback(cb EventCallback) {"));
    assert!(go_facade.contains("cgowrap_SetEventCallback_cb0.fn = cb"));
    assert!(go_facade.contains("C.cgowrap_SetEventCallback_bridge(C.bool(cb != nil))"));
}
