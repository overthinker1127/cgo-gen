use std::{env, fs};

use cgo_gen::{
    config::Config,
    domain::kind::{IrTypeKind, RecordKind},
    generator, ir, parser,
    pipeline::context::PipelineContext,
};

fn temp_output_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_test_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn parses_fixture_and_builds_ir() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    assert_eq!(parsed.records.len(), 1);
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.enums.len(), 1);
    assert_eq!(parsed.records[0].kind, RecordKind::Class);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_foo_Bar_new")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_foo_add")
    );
    assert!(ir.functions.iter().any(|item| {
        item.name == "cgowrap_foo_Bar_name" && item.returns.kind == IrTypeKind::String
    }));
}

#[test]
fn generates_wrapper_files() {
    let mut config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    config.output.dir = temp_output_dir("generate");
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = fs::read_to_string(config.output_dir().join(&config.output.header)).unwrap();
    let source = fs::read_to_string(config.output_dir().join(&config.output.source)).unwrap();
    let ir_yaml = fs::read_to_string(config.output_dir().join(&config.output.ir)).unwrap();

    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("int cgowrap_foo_add(int lhs, int rhs);"));
    assert!(source.contains("new foo::Bar(value)"));
    assert!(
        source.contains("std::string result = reinterpret_cast<const foo::Bar*>(self)->name();")
    );
    assert!(ir_yaml.contains("parser_backend: libclang"));
}

#[test]
fn parses_struct_and_class_as_distinct_record_kinds() {
    let root = temp_output_dir("record_kinds");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        struct Counter {
            int value;
        };

        class Widget {
        public:
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
  dir: gen
"#,
    )
    .unwrap();

    let ctx = PipelineContext::new(Config::load(root.join("config.yaml")).unwrap());
    let parsed = parser::parse(&ctx).unwrap();

    let counter = parsed
        .records
        .iter()
        .find(|record| record.name == "Counter")
        .unwrap();
    let widget = parsed
        .records
        .iter()
        .find(|record| record.name == "Widget")
        .unwrap();

    assert_eq!(counter.kind, RecordKind::Struct);
    assert_eq!(widget.kind, RecordKind::Class);
}
