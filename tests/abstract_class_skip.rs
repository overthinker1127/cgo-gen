use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, ir, parser, pipeline::context::PipelineContext};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_abstract_class_skip_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn skips_constructor_for_abstract_class() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class AbstractBase {
        public:
            virtual void Execute() = 0;
            virtual int GetValue() const = 0;
            void ConcreteMethod();
            virtual ~AbstractBase();
        };

        class Concrete {
        public:
            void DoWork();
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
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    // AbstractBase: no _new() wrapper
    assert!(
        !ir.functions
            .iter()
            .any(|f| f.name == "cgowrap_AbstractBase_new"),
        "abstract class must not have a constructor wrapper"
    );

    // AbstractBase: destructor and concrete method are still generated
    assert!(
        ir.functions
            .iter()
            .any(|f| f.name == "cgowrap_AbstractBase_delete"),
        "abstract class destructor wrapper must still be generated"
    );
    assert!(
        ir.functions
            .iter()
            .any(|f| f.name == "cgowrap_AbstractBase_ConcreteMethod"),
        "abstract class concrete method wrapper must still be generated"
    );

    // Concrete: _new() wrapper is present as normal
    assert!(
        ir.functions
            .iter()
            .any(|f| f.name == "cgowrap_Concrete_new"),
        "non-abstract class must still have a constructor wrapper"
    );

    // skipped_declarations records the abstract class constructor omission
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|s| s.cpp_name == "AbstractBase" && s.reason.contains("abstract class")),
        "abstract class constructor skip must be recorded in skipped_declarations"
    );
}
