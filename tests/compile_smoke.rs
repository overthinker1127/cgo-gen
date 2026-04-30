use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::var_os("CGO_GEN_TEST_TEMP_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("CARGO_TARGET_DIR").map(|dir| PathBuf::from(dir).join("compile_smoke"))
        })
        .unwrap_or_else(env::temp_dir);
    path.push(format!(
        "c_go_compile_test_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
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

fn write_simple_cpp_config(root: &Path) -> PathBuf {
    let config_path = root.join("config.yaml");
    let project_root = project_root();
    fs::write(
        &config_path,
        format!(
            r#"version: 1
input:
  dir: {}
  clang_args:
    - -std=c++17
output:
  dir: out
"#,
            project_root.join("examples/02-cpp-class/input").display()
        ),
    )
    .unwrap();
    config_path
}

#[test]
fn generated_wrapper_compiles_and_runs_against_sample_cpp_library() {
    let root = temp_output_dir("link");
    let config = Config::load(write_simple_cpp_config(&root)).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            metricsCounterHandle* counter = cgowrap_metrics_Counter_new(7);
            if (counter == nullptr) return 10;
            if (cgowrap_metrics_clamp_to_zero(-1) != 0) return 11;
            if (cgowrap_metrics_Counter_value(counter) != 7) return 12;
            cgowrap_metrics_Counter_increment(counter, 2);
            if (cgowrap_metrics_Counter_value(counter) != 9) return 13;
            char* name = cgowrap_metrics_Counter_label(counter);
            if (name == nullptr) return 14;
            cgowrap_string_free(name);
            cgowrap_metrics_Counter_delete(counter);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let project_root = project_root();
    let status = Command::new(&compiler)
        .current_dir(&project_root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(project_root.join("examples/02-cpp-class/input/counter.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(project_root.join("examples/02-cpp-class/input"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_enum_and_alias_overload_header() {
    let root = temp_output_dir("iserialize_alias");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/iSerialize.h"),
        r#"
        #include <stdint.h>
        typedef unsigned int uint32;
        typedef unsigned long long uint64;

        enum eSeriType {
            eSeriTypeNone = 0,
            eSeriTypeValue = 1,
        };

        class iSerialItem {
        public:
            iSerialItem() : value_(0) {}
            inline void GetVal(uint64 &val) { val = value_; }

        private:
            uint64 value_;
        };

        class iSerialize {
        public:
            iSerialize() = default;
            inline bool Add(uint32 nCode, uint64 val) { return nCode != 0 || val != 0; }
            inline bool Get(uint32 nCode, uint64 &val) {
                val = static_cast<uint64>(nCode) + 1;
                return true;
            }
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            iSerializeHandle* ser = cgowrap_iSerialize_new();
            if (ser == nullptr) return 10;
            if (!cgowrap_iSerialize_Add(ser, 7, 9)) return 11;
            uint64_t value = 0;
            if (!cgowrap_iSerialize_Get(ser, 7, &value)) return 12;
            if (value != 8) return 13;
            iSerialItemHandle* item = cgowrap_iSerialItem_new();
            if (item == nullptr) return 14;
            cgowrap_iSerialItem_GetVal(item, &value);
            cgowrap_iSerialItem_delete(item);
            cgowrap_iSerialize_delete(ser);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_struct_field_accessors() {
    let root = temp_output_dir("struct_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Counter.hpp"),
        r#"
        #include <stdint.h>

        struct Counter {
            int value;
            uint32_t total_count;
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            CounterHandle* counter = cgowrap_Counter_new();
            if (counter == nullptr) return 10;
            cgowrap_Counter_SetValue(counter, 9);
            if (cgowrap_Counter_GetValue(counter) != 9) return 11;
            cgowrap_Counter_SetTotalCount(counter, 42);
            if (cgowrap_Counter_GetTotalCount(counter) != 42) return 12;
            cgowrap_Counter_delete(counter);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_char_array_field_accessors() {
    let root = temp_output_dir("char_array_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Agent.hpp"),
        r#"
        struct Agent {
            char login_id[33];
            char pbx_login_id[11];
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = fs::read_to_string(config.output_dir().join(&config.output.header)).unwrap();
    let go_wrapper = fs::read_to_string(config.output_dir().join(config.go_filename(""))).unwrap();

    assert!(!header.contains("char[33]Handle"));
    assert!(!header.contains("char[11]Handle"));
    assert!(header.contains("const char* cgowrap_Agent_GetLoginId(const AgentHandle* self);"));
    assert!(
        header.contains("void cgowrap_Agent_SetLoginId(AgentHandle* self, const char* value);")
    );
    assert!(go_wrapper.contains("func (a *Agent) GetLoginId() (string, error) {"));
    assert!(go_wrapper.contains("func (a *Agent) SetLoginId(value string) {"));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        #include <cstring>
        int main() {{
            AgentHandle* agent = cgowrap_Agent_new();
            if (agent == nullptr) return 10;
            cgowrap_Agent_SetLoginId(agent, "agent-1001");
            cgowrap_Agent_SetPbxLoginId(agent, "101");
            if (std::strcmp(cgowrap_Agent_GetLoginId(agent), "agent-1001") != 0) return 11;
            if (std::strcmp(cgowrap_Agent_GetPbxLoginId(agent), "101") != 0) return 12;
            cgowrap_Agent_delete(agent);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_fixed_model_array_field_accessors() {
    let root = temp_output_dir("fixed_model_array_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Holder.hpp"),
        r#"
        struct Item {
            int value;
        };

        struct Holder {
            Item items[3];
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            HolderHandle* holder = cgowrap_Holder_new();
            if (holder == nullptr) return 10;

            ItemHandle* item0 = cgowrap_Item_new();
            ItemHandle* item1 = cgowrap_Item_new();
            ItemHandle* item2 = cgowrap_Item_new();
            if (item0 == nullptr || item1 == nullptr || item2 == nullptr) return 11;

            cgowrap_Item_SetValue(item0, 10);
            cgowrap_Item_SetValue(item1, 20);
            cgowrap_Item_SetValue(item2, 30);

            ItemHandle* items[3] = {{ item0, item1, item2 }};
            cgowrap_Holder_SetItems(holder, items);

            ItemHandle** roundtrip = cgowrap_Holder_GetItems(holder);
            if (roundtrip == nullptr) return 12;
            if (cgowrap_Item_GetValue(roundtrip[0]) != 10) return 13;
            if (cgowrap_Item_GetValue(roundtrip[1]) != 20) return 14;
            if (cgowrap_Item_GetValue(roundtrip[2]) != 30) return 15;
            if (cgowrap_Item_GetValue(cgowrap_Holder_GetItemsAt(holder, 1)) != 20) return 16;
            free(roundtrip);

            cgowrap_Item_delete(item0);
            cgowrap_Item_delete(item1);
            cgowrap_Item_delete(item2);
            cgowrap_Holder_delete(holder);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_model_value_borrow_semantics() {
    let root = temp_output_dir("model_value_snapshot");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Models.hpp"),
        r#"
        #include <stdint.h>

        struct Child {
            int value;
        };

        struct Parent {
            Child child;
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            ParentHandle* parent = cgowrap_Parent_new();
            if (parent == nullptr) return 10;
            ChildHandle* initial = cgowrap_Parent_GetChild(parent);
            if (initial == nullptr) return 11;
            cgowrap_Child_SetValue(initial, 3);
            if (cgowrap_Child_GetValue(initial) != 3) return 12;

            ChildHandle* borrowed = cgowrap_Parent_GetChild(parent);
            if (borrowed == nullptr) return 13;
            if (cgowrap_Child_GetValue(borrowed) != 3) return 14;
            cgowrap_Child_SetValue(borrowed, 9);
            if (cgowrap_Child_GetValue(initial) != 9) return 15;

            ChildHandle* latest = cgowrap_Parent_GetChild(parent);
            if (latest == nullptr) return 16;
            if (cgowrap_Child_GetValue(latest) != 9) return 17;

            cgowrap_Parent_delete(parent);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_abstract_model_pointer_returns() {
    let root = temp_output_dir("abstract_model_pointer_return");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Factory.hpp"),
        r#"
        class DBHandler {
        public:
            virtual ~DBHandler() = default;
            int GetValue() const { return 7; }
            virtual void ProcDml() = 0;
        };

        class ConcreteHandler : public DBHandler {
        public:
            void ProcDml() override {}
        };

        class DBHandlerFactory {
        public:
            DBHandler* CreateHandler() { return new ConcreteHandler(); }
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
  owner:
    - DBHandlerFactory::CreateHandler
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let source = fs::read_to_string(config.output_dir().join(&config.output.source)).unwrap();
    let go_facade = fs::read_to_string(config.output_dir().join("factory_wrapper.go")).unwrap();
    assert!(source.contains(
        "return reinterpret_cast<DBHandlerHandle*>(reinterpret_cast<DBHandlerFactory*>(self)->CreateHandler());"
    ));
    assert!(!source.contains("new DBHandler(*result)"));
    assert!(go_facade.contains("return &DBHandler{ptr: raw, owned: true, root: new(bool)}"));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            DBHandlerFactoryHandle* factory = cgowrap_DBHandlerFactory_new();
            if (factory == nullptr) return 10;
            DBHandlerHandle* handler = cgowrap_DBHandlerFactory_CreateHandler(factory);
            if (handler == nullptr) return 11;
            if (cgowrap_DBHandler_GetValue(handler) != 7) return 12;
            cgowrap_DBHandler_delete(handler);
            cgowrap_DBHandlerFactory_delete(factory);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_target_last_cyclic_header_prelude() {
    let root = temp_output_dir("cyclic_header_prelude");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Memory.hpp"),
        r#"
        #pragma once
        #include "Monitor.hpp"

        class Memory {
        public:
            Monitor* monitor;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Monitor.hpp"),
        r#"
        #pragma once
        #include "Memory.hpp"

        class Monitor {
        public:
            Monitor() = default;
            bool Init() { return true; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("driver.cpp"),
        r#"
        #include "Memory.hpp"
        #include "Monitor.hpp"
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let source = root.join("out/monitor_wrapper.cpp");
    let source_text = fs::read_to_string(&source).unwrap();
    let memory_include = source_text.find("#include \"Memory.hpp\"").unwrap();
    let monitor_include = source_text.find("#include \"Monitor.hpp\"").unwrap();
    assert!(memory_include < monitor_include);

    let smoke_cpp = root.join("out/smoke.cpp");
    fs::write(
        &smoke_cpp,
        r#"
        #include "monitor_wrapper.h"
        int main() {
            MonitorHandle* monitor = cgowrap_Monitor_new();
            if (monitor == nullptr) return 10;
            if (!cgowrap_Monitor_Init(monitor)) return 11;
            cgowrap_Monitor_delete(monitor);
            return 0;
        }
        "#,
    )
    .unwrap();

    let binary = root.join("out/smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(&source)
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(root.join("out"))
        .arg("-I")
        .arg(&include_dir)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_support_header_prelude_tokens() {
    let root = temp_output_dir("support_header_prelude");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("iSerialize.h"),
        r#"
        #pragma once
        class iSerialize {};
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("iSilType.h"),
        r#"
        #pragma once
        typedef unsigned int iMsChnl_t;
        typedef unsigned long long iChLeg_t;
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("iSiDef.h"),
        r#"
        #pragma once
        #define SIL_NAME128 128
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("AliasHolder.hpp"),
        r#"
        #pragma once

        class AliasHolder {
        public:
            AliasHolder() : ms_channel_(0), leg_(0) {}

            bool AllocChnlId(unsigned int &ext_id, iMsChnl_t &channel_id) {
                ext_id = 7;
                channel_id = 9;
                return true;
            }

            bool GetLeg(unsigned short idx, iChLeg_t &leg) {
                leg = idx;
                return true;
            }

            void Serialize(iSerialize &ar) { (void)&ar; }

        private:
            char node_name_[SIL_NAME128];
            iMsChnl_t ms_channel_;
            iChLeg_t leg_;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("driver.cpp"),
        r#"
        #include "iSerialize.h"
        #include "iSilType.h"
        #include "iSiDef.h"
        #include "AliasHolder.hpp"
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    generator::generate_all(&ctx, true).unwrap();

    let source = root.join("out/alias_holder_wrapper.cpp");
    let source_text = fs::read_to_string(&source).unwrap();
    assert!(source_text.contains("#include \"iSerialize.h\""));
    assert!(source_text.contains("#include \"iSilType.h\""));
    assert!(source_text.contains("#include \"iSiDef.h\""));

    let smoke_cpp = root.join("out/smoke.cpp");
    fs::write(
        &smoke_cpp,
        r#"
        #include "alias_holder_wrapper.h"
        int main() {
            AliasHolderHandle* holder = cgowrap_AliasHolder_new();
            if (holder == nullptr) return 10;
            unsigned int ext_id = 0;
            unsigned int ms_channel = 0;
            unsigned long long leg = 0;
            if (!cgowrap_AliasHolder_AllocChnlId(holder, &ext_id, &ms_channel)) return 11;
            if (!cgowrap_AliasHolder_GetLeg(holder, 3, &leg)) return 12;
            cgowrap_AliasHolder_delete(holder);
            return 0;
        }
        "#,
    )
    .unwrap();

    let binary = root.join("out/smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(&source)
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(root.join("out"))
        .arg("-I")
        .arg(&include_dir)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_const_model_value_and_reference_args() {
    let root = temp_output_dir("const_model_args");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/FlowApi.hpp"),
        r#"
        struct FlowData {
            int value;
        };

        class FlowApi {
        public:
            FlowApi() = default;

            void SetFlow(const FlowData flow) { flow_ = flow; }
            bool CompareFlow(const FlowData &flow) const { return flow_.value == flow.value; }

        private:
            FlowData flow_;
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            FlowApiHandle* api = cgowrap_FlowApi_new();
            if (api == nullptr) return 10;
            FlowDataHandle* flow = cgowrap_FlowData_new();
            if (flow == nullptr) return 11;
            cgowrap_FlowData_SetValue(flow, 9);
            cgowrap_FlowApi_SetFlow(api, flow);
            if (!cgowrap_FlowApi_CompareFlow(api, flow)) return 12;
            cgowrap_FlowData_delete(flow);
            cgowrap_FlowApi_delete(api);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}
