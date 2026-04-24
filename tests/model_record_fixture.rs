use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_model_record_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path.push("gen");
    fs::create_dir_all(&path).unwrap();
    path
}

fn pick_clangxx() -> String {
    for candidate in ["clang++-18", "clang++"] {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.to_string();
        }
    }
    panic!("clang++ compiler not found")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_dir() -> PathBuf {
    project_root().join("tests/fixtures/model_record")
}

fn scoped_model_record_context(config: Config) -> PipelineContext {
    let header = config
        .input
        .dir
        .as_ref()
        .expect("input dir")
        .join("DataRecord.h");
    generator::prepare_config(&PipelineContext::new(config))
        .unwrap()
        .scoped_to_header(header)
}

#[test]
fn parses_and_generates_wrapper_for_model_record_fixture() {
    let mut config = Config::load("tests/fixtures/model_record/config.yaml").unwrap();
    config.output.dir = temp_output_dir("generate");

    let ctx = scoped_model_record_context(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    assert_eq!(parsed.records.len(), 1);
    assert_eq!(parsed.functions.len(), 0);
    assert_eq!(parsed.enums.len(), 0);
    assert_eq!(parsed.records[0].name, "DataRecord");
    assert_eq!(parsed.records[0].methods.len(), 22);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    assert_eq!(ir.functions.len(), 24);
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_DataRecord_GetName")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_DataRecord_SetSlot1_Val")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_DataRecord_GetTenantId")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_DataRecord_GetSlot2_Val")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_DataRecord_SetSlot3_Act")
    );

    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = fs::read_to_string(ctx.output_dir().join(&ctx.output.header)).unwrap();
    let source = fs::read_to_string(ctx.output_dir().join(&ctx.output.source)).unwrap();
    let go_struct_path = ctx.output_dir().join(ctx.go_filename("DataRecord"));
    let go_structs = fs::read_to_string(go_struct_path).unwrap();

    assert!(header.contains("typedef struct DataRecordHandle DataRecordHandle;"));
    assert!(header.contains("DataRecordHandle* cgowrap_DataRecord_new(void);"));
    assert!(header.contains("const char* cgowrap_DataRecord_GetName(DataRecordHandle* self);"));
    assert!(header.contains("uint32_t cgowrap_DataRecord_GetTenantId(DataRecordHandle* self);"));
    assert!(header.contains("uint32_t cgowrap_DataRecord_GetNodeId(DataRecordHandle* self);"));
    assert!(header.contains(
        "void cgowrap_DataRecord_SetSlot1_Val(DataRecordHandle* self, const char* sVal);"
    ));
    assert!(
        header.contains("const char* cgowrap_DataRecord_GetSlot2_Val(DataRecordHandle* self);")
    );
    assert!(
        header.contains(
            "void cgowrap_DataRecord_SetSlot3_Act(DataRecordHandle* self, uint16_t nAct);"
        )
    );
    assert!(source.contains("return reinterpret_cast<DataRecordHandle*>(new DataRecord());"));
    assert!(source.contains("reinterpret_cast<DataRecord*>(self)->SetSlot1_Val(sVal);"));
    assert!(source.contains("reinterpret_cast<DataRecord*>(self)->GetSlot2_Val()"));
    assert!(
        source.contains(
            "reinterpret_cast<DataRecord*>(self)->SetSlot3_Act(static_cast<uint16>(nAct));"
        )
    );
    assert!(go_structs.contains("type DataRecord struct {"));
    assert!(go_structs.contains("func NewDataRecord() (*DataRecord, error) {"));
    assert!(go_structs.contains("func (d *DataRecord) GetName() (string, error) {"));
    assert!(go_structs.contains("func (d *DataRecord) SetSlot1Val(sVal string) {"));
    assert!(go_structs.contains("func (d *DataRecord) GetTenantId() uint32 {"));
    assert!(go_structs.contains("func (d *DataRecord) GetSlot2Val() (string, error) {"));
    assert!(go_structs.contains("func (d *DataRecord) SetSlot3Act(nAct uint16) {"));
    assert!(!go_structs.contains("func (d *DataRecord) GetSlot1_Val("));
}

#[test]
fn generated_wrapper_compiles_and_runs_against_model_record_fixture() {
    let mut config = Config::load("tests/fixtures/model_record/config.yaml").unwrap();
    config.output.dir = temp_output_dir("compile");

    let ctx = scoped_model_record_context(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        r#"
        #include "data_record_wrapper.h"
        #include <cstring>

        int main() {
            DataRecordHandle* item = cgowrap_DataRecord_new();
            if (item == nullptr) return 10;

            cgowrap_DataRecord_SetId(item, 42);
            if (cgowrap_DataRecord_GetId(item) != 42) return 11;

            cgowrap_DataRecord_SetTenantId(item, 7);
            if (cgowrap_DataRecord_GetTenantId(item) != 7) return 12;

            cgowrap_DataRecord_SetNodeId(item, 99);
            if (cgowrap_DataRecord_GetNodeId(item) != 99) return 13;

            cgowrap_DataRecord_SetName(item, "alice");
            if (std::strcmp(cgowrap_DataRecord_GetName(item), "alice") != 0) return 14;

            cgowrap_DataRecord_SetCode(item, "alpha");
            if (std::strcmp(cgowrap_DataRecord_GetCode(item), "alpha") != 0) return 15;

            cgowrap_DataRecord_SetSlot1_Act(item, 3);
            if (cgowrap_DataRecord_GetSlot1_Act(item) != 3) return 16;

            cgowrap_DataRecord_SetSlot1_Val(item, "hello");
            if (std::strcmp(cgowrap_DataRecord_GetSlot1_Val(item), "hello") != 0) return 17;

            cgowrap_DataRecord_SetSlot2_Act(item, 4);
            if (cgowrap_DataRecord_GetSlot2_Act(item) != 4) return 18;

            cgowrap_DataRecord_SetSlot2_Val(item, "beta");
            if (std::strcmp(cgowrap_DataRecord_GetSlot2_Val(item), "beta") != 0) return 19;

            cgowrap_DataRecord_SetSlot3_Act(item, 5);
            if (cgowrap_DataRecord_GetSlot3_Act(item) != 5) return 20;

            cgowrap_DataRecord_SetSlot3_Val(item, "gamma");
            if (std::strcmp(cgowrap_DataRecord_GetSlot3_Val(item), "gamma") != 0) return 21;

            cgowrap_DataRecord_delete(item);
            return 0;
        }
        "#,
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let root = project_root();
    let fixture = fixture_dir();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++11")
        .arg(ctx.output_dir().join(&ctx.output.source))
        .arg(fixture.join("src/DataRecord.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(ctx.output_dir())
        .arg("-I")
        .arg(fixture.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn unified_go_wrapper_renders_model_record_methods() {
    let mut config = Config::load("tests/fixtures/model_record/config.yaml").unwrap();
    config.output.dir = temp_output_dir("model-auto");

    let ctx = scoped_model_record_context(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let go_struct_path = ctx.output_dir().join(ctx.go_filename("DataRecord"));
    let go_wrapper = fs::read_to_string(go_struct_path).unwrap();

    assert!(go_wrapper.contains("type DataRecord struct {"));
    assert!(go_wrapper.contains("func NewDataRecord() (*DataRecord, error) {"));
    assert!(go_wrapper.contains("func (d *DataRecord) GetCode() (string, error) {"));
    assert!(go_wrapper.contains("func (d *DataRecord) GetSlot1Act() uint16 {"));
    assert!(go_wrapper.contains("func (d *DataRecord) GetSlot3Val() (string, error) {"));
    assert!(go_wrapper.contains("func (d *DataRecord) SetSlot2Val(sVal string) {"));
    assert!(!go_wrapper.contains("func (d *DataRecord) GetSlot1_Act("));
    assert!(go_wrapper.contains("func (d *DataRecord) Close() {"));
}
