use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, ir, parser, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_typedef_alias_resolution_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn resolves_typedef_aliases_via_canonical_types() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef unsigned int ModuleId;
        typedef int ResultCode;

        ModuleId get_id();
        ResultCode reset_id(ModuleId value);

        class Api {
        public:
            ModuleId GetId() const;
            ResultCode SetId(ModuleId value);
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

    let get_id = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_get_id")
        .unwrap();
    assert_eq!(get_id.returns.cpp_type, "ModuleId");
    assert_eq!(get_id.returns.c_type, "unsigned int");

    let reset_id = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_reset_id")
        .unwrap();
    assert_eq!(reset_id.returns.cpp_type, "ResultCode");
    assert_eq!(reset_id.returns.c_type, "int");
    assert_eq!(reset_id.params[0].ty.cpp_type, "ModuleId");
    assert_eq!(reset_id.params[0].ty.c_type, "unsigned int");

    let method = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_Api_SetId")
        .unwrap();
    assert_eq!(method.returns.cpp_type, "ResultCode");
    assert_eq!(method.params[1].ty.cpp_type, "ModuleId");
}

#[test]
fn resolves_typedef_alias_fixed_array_fields_via_canonical_types() {
    let root = temp_dir("fixed_array");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef unsigned int ReasonCode;

        struct Info {
            ReasonCode codes[4];
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

    let getter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "Info::GetCodes")
        .unwrap();
    assert_eq!(getter.returns.cpp_type, "ReasonCode[4]");
    assert_eq!(getter.returns.c_type, "unsigned int*");

    let setter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "Info::SetCodes")
        .unwrap();
    assert_eq!(setter.params[1].ty.cpp_type, "ReasonCode[4]");
    assert_eq!(setter.params[1].ty.c_type, "unsigned int*");
}

#[test]
fn resolves_reason_and_subscription_fixed_arrays_via_canonical_unsigned_types() {
    let root = temp_dir("fixed_array_aliases");
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

    let nrd_getter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "StatusInfo::GetPrimaryReasonCodes")
        .unwrap();
    assert_eq!(nrd_getter.returns.cpp_type, "ReasonCode[64]");
    assert_eq!(nrd_getter.returns.c_type, "unsigned int*");

    let nrd_setter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "StatusInfo::SetPrimaryReasonCodes")
        .unwrap();
    assert_eq!(nrd_setter.params[1].ty.cpp_type, "ReasonCode[64]");
    assert_eq!(nrd_setter.params[1].ty.c_type, "unsigned int*");

    let acw_getter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "StatusInfo::GetSecondaryReasonCodes")
        .unwrap();
    assert_eq!(acw_getter.returns.cpp_type, "ReasonCode[64]");
    assert_eq!(acw_getter.returns.c_type, "unsigned int*");

    let acw_setter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "StatusInfo::SetSecondaryReasonCodes")
        .unwrap();
    assert_eq!(acw_setter.params[1].ty.cpp_type, "ReasonCode[64]");
    assert_eq!(acw_setter.params[1].ty.c_type, "unsigned int*");

    let subscribe_getter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "SubscriptionCodes::GetSubscriptionIds")
        .unwrap();
    assert_eq!(subscribe_getter.returns.cpp_type, "SubscriptionId[16]");
    assert_eq!(subscribe_getter.returns.c_type, "unsigned int*");

    let subscribe_setter = ir
        .functions
        .iter()
        .find(|item| item.cpp_name == "SubscriptionCodes::SetSubscriptionIds")
        .unwrap();
    assert_eq!(subscribe_setter.params[1].ty.cpp_type, "SubscriptionId[16]");
    assert_eq!(subscribe_setter.params[1].ty.c_type, "unsigned int*");
}
