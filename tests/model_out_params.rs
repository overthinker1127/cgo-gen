use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_model_out_params_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn recognizes_known_model_out_params_in_facade_wrappers() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
            void SetValue(int value);
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
            bool GetThing(int id, ThingModel& out);
            bool GetThingPtr(int id, ThingModel* out);
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

    let raw_output_dir = root.join("gen");
    let api_header = fs::read_to_string(raw_output_dir.join("api_wrapper.h")).unwrap();
    let api_source = fs::read_to_string(raw_output_dir.join("api_wrapper.cpp")).unwrap();
    let api_go = fs::read_to_string(root.join("gen/api_wrapper.go")).unwrap();

    assert!(api_header.contains("typedef struct ThingModelHandle ThingModelHandle;"));
    assert!(
        api_header
            .contains("bool cgowrap_Api_GetThing(ApiHandle* self, int id, ThingModelHandle* out);")
    );
    assert!(
        api_header.contains(
            "bool cgowrap_Api_GetThingPtr(ApiHandle* self, int id, ThingModelHandle* out);"
        )
    );
    assert!(api_source.contains("*reinterpret_cast<ThingModel*>(out)"));
    assert!(api_source.contains("reinterpret_cast<ThingModel*>(out)"));
    assert!(api_go.contains("func (a *Api) GetThing(id int32, out *ThingModel) bool {"));
    assert!(api_go.contains("func (a *Api) GetThingPtr(id int32, out *ThingModel) bool {"));
    assert!(!api_go.contains("mapThingModelFromHandle"));
}

#[test]
fn skips_mismatched_getter_setter_fields_without_failing_generation() {
    let root = temp_dir("mismatch_skip");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
            void SetValue(int value);
            unsigned short GetNextHop() const;
            void SetNextHop(short value);
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
            bool GetThing(int id, ThingModel& out);
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

    let api_go = fs::read_to_string(root.join("gen/api_wrapper.go")).unwrap();

    assert!(root.join("gen/api_wrapper.go").exists());
    assert!(api_go.contains("package"));
}
