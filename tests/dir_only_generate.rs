use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_dir_only_generate_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn dir_only_generation_uses_classified_headers_for_model_and_facade_outputs() {
    let root = temp_dir("classified_outputs");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() {}
            int GetId() const { return 7; }
            void SetId(int value) { (void)value; }
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
            Api() {}
            bool GetThingById(int id, ThingModel* out) { return id > 0; }
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
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");

    assert!(output_dir.join("thing_model_wrapper.h").exists());
    assert!(output_dir.join("api_wrapper.h").exists());
    assert!(output_dir.join("thing_model_wrapper.go").exists());
    assert!(output_dir.join("api_wrapper.go").exists());
}

#[test]
fn nested_output_dir_places_all_generated_files_at_output_root() {
    let root = temp_dir("nested_output");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Thing.hpp"),
        r#"
        class Thing {
        public:
            Thing() {}
            int GetValue() const { return 7; }
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
  dir: ./gen/test
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    assert!(root.join("gen/test").is_dir());
    assert!(root.join("gen/test/thing_wrapper.go").exists());
    assert!(root.join("gen/test/thing_wrapper.h").exists());
    assert!(root.join("gen/test/thing_wrapper.cpp").exists());
    assert!(root.join("gen/test/thing_wrapper.ir.yaml").exists());
}

#[test]
fn dir_generation_skips_standalone_outputs_for_owner_inline_headers() {
    let root = temp_dir("owner_inline_headers");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #pragma once

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
        };
        "#,
    )
    .unwrap();

    fs::write(
        include_dir.join("Api-inl.hpp"),
        r#"
        #pragma once
        #include "Api.hpp"

        inline bool Api::IsReady() const { return true; }
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
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    assert!(output_dir.join("api_wrapper.h").exists());
    assert!(output_dir.join("api_wrapper.cpp").exists());
    assert!(output_dir.join("api_wrapper.go").exists());
    assert!(!output_dir.join("api_inl_wrapper.h").exists());
    assert!(!output_dir.join("api_inl_wrapper.cpp").exists());
    assert!(!output_dir.join("api_inl_wrapper.go").exists());

    let raw_source = fs::read_to_string(output_dir.join("api_wrapper.cpp")).unwrap();
    assert!(raw_source.contains("cgowrap_Api_IsReady"));
    assert!(raw_source.contains("#include \"Api.hpp\""));
}
