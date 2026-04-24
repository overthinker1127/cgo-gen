use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_multi_header_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn generates_one_wrapper_set_per_header_from_single_config() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/AlphaThing.hpp"),
        r#"
        class AlphaThing {
        public:
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/BetaThing.hpp"),
        r#"
        class BetaThing {
        public:
            const char* GetName() const;
            void SetName(const char* name);
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    let alpha_header = output_dir.join("alpha_thing_wrapper.h");
    let alpha_source = output_dir.join("alpha_thing_wrapper.cpp");
    let alpha_ir = output_dir.join("alpha_thing_wrapper.ir.yaml");
    let alpha_go = output_dir.join("alpha_thing_wrapper.go");

    let beta_header = output_dir.join("beta_thing_wrapper.h");
    let beta_source = output_dir.join("beta_thing_wrapper.cpp");
    let beta_ir = output_dir.join("beta_thing_wrapper.ir.yaml");
    let beta_go = output_dir.join("beta_thing_wrapper.go");

    for path in [
        &alpha_header,
        &alpha_source,
        &alpha_ir,
        &alpha_go,
        &beta_header,
        &beta_source,
        &beta_ir,
        &beta_go,
    ] {
        assert!(path.exists(), "missing generated file: {}", path.display());
    }

    let alpha_header_text = fs::read_to_string(alpha_header).unwrap();
    let alpha_go_text = fs::read_to_string(alpha_go).unwrap();
    let beta_header_text = fs::read_to_string(beta_header).unwrap();
    let beta_go_text = fs::read_to_string(beta_go).unwrap();

    assert!(alpha_header_text.contains("AlphaThingHandle"));
    assert!(!alpha_header_text.contains("BetaThingHandle"));
    assert!(alpha_go_text.contains("type AlphaThing struct {"));
    assert!(alpha_go_text.contains("ptr *C.AlphaThingHandle"));
    assert!(!alpha_go_text.contains("type BetaThing struct {"));

    assert!(beta_header_text.contains("BetaThingHandle"));
    assert!(!beta_header_text.contains("AlphaThingHandle"));
    assert!(beta_go_text.contains("type BetaThing struct {"));
    assert!(beta_go_text.contains("ptr *C.BetaThingHandle"));
    assert!(!beta_go_text.contains("type AlphaThing struct {"));
}

#[test]
fn emits_unified_go_wrappers_for_each_supported_header() {
    let root = temp_dir("classification");
    fs::write(
        root.join("include/ModelThing.hpp"),
        r#"
        class ModelThing {
        public:
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/FacadeThing.hpp"),
        r#"
        class FacadeThing {
        public:
            int GetCount() const;
            void SetCount(int count);
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    let model_go = fs::read_to_string(output_dir.join("model_thing_wrapper.go")).unwrap();
    let facade_go = fs::read_to_string(output_dir.join("facade_thing_wrapper.go")).unwrap();

    assert!(model_go.contains("type ModelThing struct {"));
    assert!(model_go.contains("func (m *ModelThing) GetValue() int32 {"));
    assert!(facade_go.contains("type FacadeThing struct {"));
    assert!(facade_go.contains("func (f *FacadeThing) GetCount() int32 {"));
}

#[test]
fn multi_header_model_value_return_emits_delete_with_owned_go_wrapper() {
    let root = temp_dir("owned_opaque_delete");
    fs::write(
        root.join("include/KeyTypes.hpp"),
        r#"
        struct Table {
            struct iKey {
                int id;
            };
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/KeyApi.hpp"),
        r#"
        #include "KeyTypes.hpp"

        class KeyApi {
        public:
            Table::iKey GetKey() const;
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    let api_header = fs::read_to_string(output_dir.join("key_api_wrapper.h")).unwrap();
    let api_source = fs::read_to_string(output_dir.join("key_api_wrapper.cpp")).unwrap();
    let api_go = fs::read_to_string(output_dir.join("key_api_wrapper.go")).unwrap();

    assert!(api_header.contains("void cgowrap_TableiKey_delete(TableiKeyHandle* self);"));
    assert!(api_source.contains("delete reinterpret_cast<Table::iKey*>(self);"));
    assert!(api_go.contains("type TableiKey struct {"));
    assert!(api_go.contains("C.cgowrap_TableiKey_delete"));
    assert!(api_go.contains("return newOwnedTableiKey(raw)"));
}

#[test]
fn emits_unified_go_enums_without_classification() {
    let root = temp_dir("model-enum");
    fs::write(
        root.join("include/ModelTypes.hpp"),
        r#"
        enum Mode {
            MODE_A = 0,
            MODE_B = 1,
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    let go_models = fs::read_to_string(output_dir.join("model_types_wrapper.go")).unwrap();

    assert!(go_models.contains("type Mode int64"));
    assert!(go_models.contains("MODE_A Mode = 0"));
    assert!(go_models.contains("MODE_B Mode = 1"));
}

#[test]
fn unclassified_headers_still_emit_unified_go_wrappers() {
    let root = temp_dir("unclassified-go-structs");
    fs::write(
        root.join("include/Thing.hpp"),
        r#"
        class Thing {
        public:
            int GetValue() const;
            void SetValue(int value);
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let go_path = root.join("gen/thing_wrapper.go");
    assert!(
        go_path.exists(),
        "supported headers should emit unified Go files"
    );
    let go = fs::read_to_string(go_path).unwrap();
    assert!(go.contains("type Thing struct {"));
}
