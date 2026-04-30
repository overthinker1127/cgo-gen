use std::{
    env, fs,
    path::{Path, PathBuf},
};

use cgo_gen::{config::Config, generator, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_example_cpp_inventory_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn checked_in_cpp_inventory_example_uses_handle_backed_item_and_reference_cursor() {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/03-cpp-inventory");
    let mut config = Config::load(example_dir.join("config.yaml")).unwrap();
    config.output.dir = temp_output_dir("generate");
    let ctx = PipelineContext::new(config.clone());

    assert_eq!(config.discovered_headers().unwrap().len(), 2);
    generator::generate_all(&ctx, true).unwrap();

    let go_item = fs::read_to_string(config.output.dir.join("inventory_item_wrapper.go")).unwrap();
    let go_service =
        fs::read_to_string(config.output.dir.join("inventory_service_wrapper.go")).unwrap();
    assert!(!config.output.dir.join("build_flags.go").exists());
    assert!(!config.output.dir.join("go.mod").exists());

    assert!(go_item.contains("type InventoryItem struct {"));
    assert!(go_item.contains("ptr *C.InventoryItemHandle"));
    assert!(go_item.contains("func NewInventoryItem() (*InventoryItem, error) {"));
    assert!(go_item.contains("func (i *InventoryItem) SetName(name string) {"));
    assert!(go_item.contains("func (i *InventoryItem) SetQuantity(quantity int32) {"));

    assert!(
        go_service
            .contains("func (i *InventoryService) LoadItem(id int32, out *InventoryItem) bool {")
    );
    assert!(
        go_service.contains(
            "func (i *InventoryService) NextItem(cursor *int32, out *InventoryItem) bool {"
        )
    );
    assert!(go_service.contains("cArg0 := C.int32_t(*cursor)"));
    assert!(go_service.contains("*cursor = int32(cArg0)"));

    let with_module = PipelineContext::from_config_path(example_dir.join("config.yaml"))
        .unwrap()
        .with_go_module(Some("example.com/demo/pkg".to_string()))
        .with_output_dir(temp_output_dir("generate_with_module"));
    generator::generate_all(&with_module, true).unwrap();

    let build_flags = fs::read_to_string(with_module.output.dir.join("build_flags.go")).unwrap();
    let go_mod = fs::read_to_string(with_module.output.dir.join("go.mod")).unwrap();
    assert!(build_flags.contains("#cgo CFLAGS: -I${SRCDIR}"));
    assert!(build_flags.contains("#cgo CXXFLAGS: -I${SRCDIR}"));
    assert_eq!(go_mod, "module example.com/demo/pkg\n\ngo 1.25\n");
}
