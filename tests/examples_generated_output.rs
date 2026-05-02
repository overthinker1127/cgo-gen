use std::{
    env, fs,
    path::{Path, PathBuf},
};

use cgo_gen::{generator, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_examples_generated_output_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn assert_generated_matches(example: &str, go_module: Option<&str>, expected_files: &[&str]) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(example);
    let output_root = temp_output_dir(example.replace('/', "_").as_str());
    let output_dir = output_root.join("generated");
    let mut ctx = PipelineContext::from_config_path(root.join("config.yaml"))
        .unwrap()
        .with_output_dir(output_dir.clone());
    if let Some(go_module) = go_module {
        ctx = ctx.with_go_module(Some(go_module.to_string()));
    }

    generator::generate_all(&ctx, true).unwrap();

    for relative in expected_files {
        let generated = fs::read_to_string(output_dir.join(relative)).unwrap();
        let committed = fs::read_to_string(root.join("generated").join(relative)).unwrap();
        let generated = normalize_ir_source_header_prefixes(relative, &generated);
        let committed = normalize_ir_source_header_prefixes(relative, &committed);
        assert_eq!(
            generated, committed,
            "{example}/generated/{relative} is stale"
        );
    }
}

fn normalize_ir_source_header_prefixes(relative: &str, contents: &str) -> String {
    if !relative.ends_with(".ir.yaml") {
        return contents.to_string();
    }

    contents
        .lines()
        .map(|line| {
            let Some((prefix, suffix)) = line.split_once("/examples/") else {
                return line.to_string();
            };
            if !prefix.trim_start().starts_with("- ") {
                return line.to_string();
            }
            format!("- <repo>/examples/{suffix}")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn checked_in_example_generated_outputs_are_current() {
    assert_generated_matches(
        "examples/01-c-library",
        None,
        &[
            "calculator_wrapper.h",
            "calculator_wrapper.cpp",
            "calculator_wrapper.go",
            "calculator_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/02-cpp-class",
        None,
        &[
            "counter_wrapper.h",
            "counter_wrapper.cpp",
            "counter_wrapper.go",
            "counter_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/03-cpp-inventory",
        None,
        &[
            "inventory_item_wrapper.h",
            "inventory_item_wrapper.cpp",
            "inventory_item_wrapper.go",
            "inventory_item_wrapper.ir.yaml",
            "inventory_service_wrapper.h",
            "inventory_service_wrapper.cpp",
            "inventory_service_wrapper.go",
            "inventory_service_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/04-go-module",
        Some("example.com/cgo-gen/examples/04-go-module/generated"),
        &[
            "score_wrapper.h",
            "score_wrapper.cpp",
            "score_wrapper.go",
            "score_wrapper.ir.yaml",
            "go.mod",
            "build_flags.go",
        ],
    );
    assert_generated_matches(
        "examples/05-headers-list",
        None,
        &[
            "selected_widget_wrapper.h",
            "selected_widget_wrapper.cpp",
            "selected_widget_wrapper.go",
            "selected_widget_wrapper.ir.yaml",
            "selected_counter_wrapper.h",
            "selected_counter_wrapper.cpp",
            "selected_counter_wrapper.go",
            "selected_counter_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/06-owner-return",
        None,
        &[
            "session_factory_wrapper.h",
            "session_factory_wrapper.cpp",
            "session_factory_wrapper.go",
            "session_factory_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/07-enums",
        None,
        &[
            "device_controller_wrapper.h",
            "device_controller_wrapper.cpp",
            "device_controller_wrapper.go",
            "device_controller_wrapper.ir.yaml",
        ],
    );
    assert_generated_matches(
        "examples/08-overloading",
        None,
        &[
            "overload_math_wrapper.h",
            "overload_math_wrapper.cpp",
            "overload_math_wrapper.go",
            "overload_math_wrapper.ir.yaml",
        ],
    );
}
