use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_function_pointer_skip_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn skips_declarations_using_function_pointer_types() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        int add(int lhs, int rhs);
        void set_callback(void (*cb)(int code));

        class Api {
        public:
            int GetValue() const;
            void SetCallback(void (*cb)(int code));
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
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    assert!(ir.functions.iter().any(|item| item.name == "cgowrap_add"));
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Api_GetValue")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_set_callback")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Api_SetCallback")
    );

    assert_eq!(ir.support.skipped_declarations.len(), 2);
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "set_callback" && item.reason.contains("function pointer"))
    );
    assert!(ir.support.skipped_declarations.iter().any(
        |item| item.cpp_name == "Api::SetCallback" && item.reason.contains("function pointer")
    ));
}

#[test]
fn skips_operator_declarations() {
    let root = temp_dir("operators");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
            Value operator+(const Value& rhs) const;
            bool operator==(const Value& rhs) const;
            int GetCode() const;
        };

        Value operator-(const Value& lhs, const Value& rhs);
        int plain_add(int lhs, int rhs);
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
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_plain_add")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Value_GetCode")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator+"))
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator=="))
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator-"))
    );

    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Value::operator+"
                && item.reason.contains("operator declarations"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Value::operator=="
                && item.reason.contains("operator declarations"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "operator-"
                && item.reason.contains("operator declarations"))
    );
}

#[test]
fn skips_double_pointer_model_declarations() {
    let root = temp_dir("double-pointer-models");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            bool CreateThing(ThingModel** out);
            ThingModel** GetThingPtrPtr();
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
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name == "Api::CreateThing")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name == "Api::GetThingPtrPtr")
    );
    assert!(ir.support.skipped_declarations.iter().any(|item| {
        item.cpp_name == "Api::CreateThing" && item.reason.contains("double-pointer")
    }));
    assert!(ir.support.skipped_declarations.iter().any(|item| {
        item.cpp_name == "Api::GetThingPtrPtr" && item.reason.contains("double-pointer")
    }));

    generator::generate_all(&ctx, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/api_wrapper.cpp")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(raw_header.contains("bool cgowrap_Api_IsReady(const ApiHandle* self);"));
    assert!(!raw_header.contains("CreateThing"));
    assert!(!raw_header.contains("GetThingPtrPtr"));
    assert!(raw_source.contains("cgowrap_Api_IsReady"));
    assert!(!raw_source.contains("CreateThing"));
    assert!(!raw_source.contains("GetThingPtrPtr"));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(!go_facade.contains("CreateThing"));
    assert!(!go_facade.contains("GetThingPtrPtr"));
}

#[test]
fn skips_double_pointer_string_declarations() {
    let root = temp_dir("double-pointer-string");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        int Count();
        void GetMessage(char **out);
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
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    assert!(ir.functions.iter().any(|item| item.name == "cgowrap_Count"));
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name == "GetMessage")
    );
    assert!(
        ir.support.skipped_declarations.iter().any(|item| {
            item.cpp_name == "GetMessage" && item.reason.contains("double-pointer")
        })
    );

    generator::generate_all(&ctx, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/api_wrapper.h")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/api_wrapper.go")).unwrap();

    assert!(raw_header.contains("int cgowrap_Count(void);"));
    assert!(!raw_header.contains("GetMessage"));
    assert!(go_facade.contains("func Count() int32 {"));
    assert!(!go_facade.contains("GetMessage"));
}
