use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cgo_gen::{Config, PipelineContext, generator};

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
  dirs:
    - include
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
  dirs:
    - include
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
  dirs:
    - include
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

#[test]
fn headers_only_generation_emits_wrappers_only_for_listed_headers() {
    let root = temp_dir("headers_only_selected_outputs");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Selected.hpp"),
        r#"
        #include "Skipped.hpp"

        class Selected {
        public:
            Selected() {}
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("AlsoSelected.hpp"),
        r#"
        class AlsoSelected {
        public:
            AlsoSelected() {}
            int GetValue() const { return 11; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Skipped.hpp"),
        r#"
        class Skipped {
        public:
            Skipped() {}
            int GetValue() const { return 9; }
        };
        "#,
    )
    .unwrap();

    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Selected.hpp
    - include/AlsoSelected.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    assert!(output_dir.join("selected_wrapper.h").exists());
    assert!(output_dir.join("selected_wrapper.cpp").exists());
    assert!(output_dir.join("selected_wrapper.go").exists());
    assert!(output_dir.join("also_selected_wrapper.h").exists());
    assert!(output_dir.join("also_selected_wrapper.cpp").exists());
    assert!(output_dir.join("also_selected_wrapper.go").exists());
    assert!(!output_dir.join("skipped_wrapper.h").exists());
    assert!(!output_dir.join("skipped_wrapper.cpp").exists());
    assert!(!output_dir.join("skipped_wrapper.go").exists());
}

#[test]
fn multiple_owned_dirs_generate_shared_model_handles_and_compile() {
    let root = temp_dir("multiple_owned_dirs_compile");
    let a_dir = root.join("A");
    let b_dir = root.join("B");
    fs::create_dir_all(&a_dir).unwrap();
    fs::create_dir_all(&b_dir).unwrap();

    fs::write(
        a_dir.join("A.hpp"),
        r#"
        #pragma once
        #include "../B/B.hpp"

        class A {
        public:
            A() {}
            B child;
            B* Child() { return &child; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        b_dir.join("B.hpp"),
        r#"
        #pragma once

        class B {
        public:
            B() {}
            int Value() const { return 7; }
        };
        "#,
    )
    .unwrap();

    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  dirs:
    - A
    - B
output:
  dir: gen
"#,
    )
    .unwrap();

    let ctx = PipelineContext::from_config_path(root.join("config.yaml"))
        .unwrap()
        .with_go_module(Some("example.com/demo".to_string()));
    generator::generate_all(&ctx, true).unwrap();

    let output_dir = root.join("gen");
    assert!(output_dir.join("a_wrapper.h").exists());
    assert!(output_dir.join("b_wrapper.h").exists());

    let a_header = fs::read_to_string(output_dir.join("a_wrapper.h")).unwrap();
    assert!(a_header.contains("typedef struct BHandle BHandle;"));
    assert!(a_header.contains("BHandle* cgowrap_A_Child(AHandle* self);"));

    let build_flags = fs::read_to_string(output_dir.join("build_flags.go")).unwrap();
    assert!(build_flags.contains("#cgo CXXFLAGS: -I${SRCDIR}"));
    assert!(!build_flags.contains("-I${SRCDIR}/../A"));
    assert!(!build_flags.contains("-I${SRCDIR}/../B"));

    compile_generated_cpp(&root, "a_wrapper.cpp");
    compile_generated_cpp(&root, "b_wrapper.cpp");
}

fn compile_generated_cpp(root: &Path, source: &str) {
    let output = Command::new("c++")
        .arg("-std=c++17")
        .arg("-I")
        .arg(root.join("gen"))
        .arg("-I")
        .arg(root.join("A"))
        .arg("-I")
        .arg(root.join("B"))
        .arg("-c")
        .arg(root.join("gen").join(source))
        .arg("-o")
        .arg(root.join("gen").join(format!("{source}.o")))
        .output()
        .unwrap_or_else(|error| panic!("failed to run c++: {error}"));

    assert!(
        output.status.success(),
        "failed to compile {source}:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
