use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_facade_only_generation_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn legacy_model_classification_still_emits_direct_facade_methods() {
    let root = temp_dir("legacy-model-classification");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
            void SetValue(int value);
            int Clear();
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

    let go = fs::read_to_string(root.join("gen/thing_model_wrapper.go")).unwrap();

    assert!(go.contains("type ThingModel struct {"));
    assert!(go.contains("func NewThingModel() (*ThingModel, error) {"));
    assert!(go.contains("func (t *ThingModel) GetValue() int32 {"));
    assert!(go.contains("func (t *ThingModel) SetValue(value int32) {"));
    assert!(go.contains("func (t *ThingModel) Clear() int32 {"));
}

#[test]
fn known_model_out_params_work_without_model_filters() {
    let root = temp_dir("known-model-out-params");
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

    let model_go = fs::read_to_string(root.join("gen/thing_model_wrapper.go")).unwrap();
    let api_go = fs::read_to_string(root.join("gen/api_wrapper.go")).unwrap();

    assert!(model_go.contains("func (t *ThingModel) GetValue() int32 {"));
    assert!(model_go.contains("func (t *ThingModel) SetValue(value int32) {"));
    assert!(api_go.contains("func (a *Api) GetThing(id int32, out *ThingModel) bool {"));
    assert!(api_go.contains("func (a *Api) GetThingPtr(id int32, out *ThingModel) bool {"));
    assert!(api_go.contains("requireThingModelHandle(out)"));
    assert!(api_go.contains("optionalThingModelHandle(out)"));
}
