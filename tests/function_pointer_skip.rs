use std::{env, fs, path::PathBuf};

use cgo_gen::{Config, PipelineContext, generator, ir, parser};

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
  dirs:
    - include
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
fn generates_operator_declarations() {
    let root = temp_dir("operators");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
            Value operator+(const Value& rhs) const;
            bool operator==(const Value& rhs) const;
            operator bool() const;
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
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let value = parsed
        .records
        .iter()
        .find(|record| record.name == "Value")
        .unwrap();
    assert_eq!(
        value
            .methods
            .iter()
            .map(|method| method.name.as_str())
            .collect::<Vec<_>>(),
        vec!["GetCode"]
    );
    assert_eq!(
        value
            .operators
            .iter()
            .map(|operator| operator.spelling.as_str())
            .collect::<Vec<_>>(),
        vec!["operator bool", "operator+", "operator=="]
    );
    assert_eq!(
        parsed
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        vec!["plain_add"]
    );
    assert_eq!(
        parsed
            .free_operators
            .iter()
            .map(|operator| operator.spelling.as_str())
            .collect::<Vec<_>>(),
        vec!["operator-"]
    );

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
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Value_OperPlus"
                && item.cpp_name == "Value::operator+"
                && item.operator.is_some())
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Value_OperEqual"
                && item.cpp_name == "Value::operator=="
                && item.operator.is_some())
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Value_OperBool"
                && item.cpp_name == "Value::operator bool"
                && item.operator.is_some())
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_OperMinus"
                && item.cpp_name == "operator-"
                && item.operator.is_some())
    );
    assert!(ir.support.skipped_declarations.is_empty());
}

#[test]
fn parses_operator_header_definition_metadata() {
    let root = temp_dir("operator_definitions");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        namespace demo {
        class Value {
        public:
            Value operator+(const Value& rhs) const;
            bool operator==(const Value& rhs) const { return true; }
        };

        Value operator-(const Value& lhs, const Value& rhs);
        inline Value operator*(const Value& lhs, const Value& rhs) { return Value(); }
        }
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let value = parsed
        .records
        .iter()
        .find(|record| record.name == "Value")
        .unwrap();

    let plus = value
        .operators
        .iter()
        .find(|operator| operator.spelling == "operator+")
        .unwrap();
    assert_eq!(plus.owner.as_deref(), Some("demo::Value"));
    assert_eq!(plus.token, parser::CppOperatorToken::Plus);
    assert!(!plus.has_header_definition);

    let eq = value
        .operators
        .iter()
        .find(|operator| operator.spelling == "operator==")
        .unwrap();
    assert_eq!(eq.owner.as_deref(), Some("demo::Value"));
    assert_eq!(eq.token, parser::CppOperatorToken::Equal);
    assert!(eq.has_header_definition);

    let minus = parsed
        .free_operators
        .iter()
        .find(|operator| operator.spelling == "operator-")
        .unwrap();
    assert_eq!(minus.owner, None);
    assert_eq!(minus.token, parser::CppOperatorToken::Minus);
    assert!(!minus.has_header_definition);

    let star = parsed
        .free_operators
        .iter()
        .find(|operator| operator.spelling == "operator*")
        .unwrap();
    assert_eq!(star.owner, None);
    assert_eq!(star.token, parser::CppOperatorToken::Multiply);
    assert!(star.has_header_definition);
}

#[test]
fn merges_operator_definition_metadata_across_translation_units() {
    let root = temp_dir("operator_multi_tu_dedupe");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
        #ifdef DEFINE_OPERATOR_BODY
            Value operator+(const Value& rhs) const { return rhs; }
        #else
            Value operator+(const Value& rhs) const;
        #endif
            int GetCode() const;
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/with_body.cpp"),
        r#"
        #define DEFINE_OPERATOR_BODY
        #include "Api.hpp"
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/without_body.cpp"),
        r#"
        #include "Api.hpp"
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();

    assert_eq!(parsed.records.len(), 1);
    let value = &parsed.records[0];
    assert_eq!(value.operators.len(), 1);
    assert!(value.operators[0].has_header_definition);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert_eq!(
        ir.functions
            .iter()
            .filter(|function| function.cpp_name == "Value::GetCode")
            .count(),
        1
    );
}

#[test]
fn skips_function_bodies_while_detecting_inline_operator_definitions() {
    let root = temp_dir("operator_body_diagnostic");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
            bool operator==(const Value& rhs) const { return missing_symbol; }
            int GetCode() const;
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
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let value = parsed
        .records
        .iter()
        .find(|record| record.name == "Value")
        .unwrap();

    assert_eq!(value.operators.len(), 1);
    assert!(value.operators[0].has_header_definition);
}

#[test]
fn parses_conversion_operators_as_member_operators() {
    let root = temp_dir("conversion_operator");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Convertible {
        public:
            operator bool() const;
            int GetCode() const;
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
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let convertible = parsed
        .records
        .iter()
        .find(|record| record.name == "Convertible")
        .unwrap();

    assert_eq!(
        convertible
            .methods
            .iter()
            .map(|method| method.name.as_str())
            .collect::<Vec<_>>(),
        vec!["GetCode"]
    );
    assert_eq!(convertible.operators.len(), 1);
    assert_eq!(convertible.operators[0].spelling, "operator bool");
    assert_eq!(
        convertible.operators[0].token,
        parser::CppOperatorToken::Conversion("bool".to_string())
    );
}

#[test]
fn distinguishes_assignment_operator_tokens_for_wrapper_names() {
    let root = temp_dir("assignment_operator_names");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
            Value& operator+=(const Value& rhs);
            Value& operator-=(const Value& rhs);
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
  dirs:
    - include
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
            .any(|function| function.name == "cgowrap_Value_OperPlusAssign")
    );
    assert!(
        ir.functions
            .iter()
            .any(|function| function.name == "cgowrap_Value_OperMinusAssign")
    );
}

#[test]
fn parses_friend_operators_as_free_operators() {
    let root = temp_dir("friend_operator");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        namespace demo {
        class Value {
        public:
            friend Value operator+(const Value& lhs, const Value& rhs);
            int GetCode() const;
        };
        }
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dirs:
    - include
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();

    assert_eq!(
        parsed
            .records
            .iter()
            .find(|record| record.name == "Value")
            .unwrap()
            .operators
            .len(),
        1
    );
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|function| function.name == "cgowrap_demo_OperPlus"
                && function.cpp_name == "demo::operator+")
    );
}

#[test]
fn generates_default_argument_variants_for_call_operator() {
    let root = temp_dir("call_operator_default_args");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Scale {
        public:
            int operator()(int value = 1) const;
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
  dirs:
    - include
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
            .any(|function| function.name == "cgowrap_Scale_OperFunc__int_const")
    );
    assert!(
        ir.functions
            .iter()
            .any(|function| function.name == "cgowrap_Scale_OperFunc__void_const")
    );
}

#[test]
fn skips_allocator_operators_without_affecting_lifecycle_wrappers() {
    let root = temp_dir("allocator_operators");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include <cstddef>

        class AllocHook {
        public:
            AllocHook() = default;
            static void* operator new(std::size_t size) { return ::operator new(size); }
            static void operator delete(void* ptr) noexcept { ::operator delete(ptr); }
            static void* operator new[](std::size_t size) { return ::operator new[](size); }
            static void operator delete[](void* ptr) noexcept { ::operator delete[](ptr); }
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
  dirs:
    - include
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
            .any(|function| function.name == "cgowrap_AllocHook_new")
    );
    assert!(
        ir.functions
            .iter()
            .any(|function| function.name == "cgowrap_AllocHook_delete")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|function| function.name.contains("OperNew")
                || function.name.contains("OperDelete")
                || function.name.contains("OperUnsupported"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "AllocHook::operator new")
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "AllocHook::operator delete")
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "AllocHook::operator new[]")
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "AllocHook::operator delete[]")
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
  dirs:
    - include
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
  dirs:
    - include
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
