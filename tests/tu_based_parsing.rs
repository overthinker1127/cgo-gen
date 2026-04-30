use std::{env, fs, path::PathBuf};

use cgo_gen::{compiler, config::Config, parser, pipeline::context::PipelineContext};

fn temp_fixture_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_tu_based_parsing_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn dir_only_config_collects_only_owned_header_declarations() {
    let fixture = temp_fixture_dir("owned_only");
    fs::create_dir_all(fixture.join("include")).unwrap();
    fs::create_dir_all(fixture.join("external")).unwrap();

    fs::write(
        fixture.join("include/owned.hpp"),
        r#"
        namespace demo {
        class Owned {
        public:
            int GetValue() const { return 7; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("external/foreign.hpp"),
        r#"
        namespace demo {
        class Foreign {
        public:
            int GetValue() const { return 9; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/tu.cpp"),
        r#"
        #include "owned.hpp"
        #include "../external/foreign.hpp"

        namespace demo {
        class LocalOnly {
        public:
            int GetValue() const { return 11; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        format!(
            r#"
version: 1
input:
  dir: include
  clang_args:
    - -I{}
    - -I{}
output:
  dir: gen
"#,
            fixture.join("include").display(),
            fixture.join("external").display()
        ),
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();

    assert!(
        parsed
            .headers
            .iter()
            .any(|header| header.ends_with("owned.hpp"))
    );
    assert!(
        !parsed
            .headers
            .iter()
            .any(|header| header.ends_with("foreign.hpp"))
    );
    assert!(parsed.records.iter().any(|record| record.name == "Owned"));
    assert!(
        !parsed
            .records
            .iter()
            .any(|record| record.name == "LocalOnly")
    );
    assert!(!parsed.records.iter().any(|record| record.name == "Foreign"));
}

#[test]
fn dir_only_config_prefers_sources_when_present() {
    let fixture = temp_fixture_dir("header_entries_ignored");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/MemoryStore.h"),
        r#"
        class MemoryStore {
        public:
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/DataHandler.cpp"),
        r#"
        #include "MemoryStore.h"
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let units = compiler::collect_translation_units(&config).unwrap();

    assert_eq!(units.len(), 1);
    assert!(units[0].ends_with("DataHandler.cpp"));
}

#[test]
fn dir_only_config_expands_classified_header_directory_into_all_grouped_headers() {
    let fixture = temp_fixture_dir("classified_dir_expansion");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/entry.hpp"),
        r#"
        #include "shared.hpp"

        namespace demo {
        class Entry {
        public:
            int GetValue() const { return 7; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/shared.hpp"),
        r#"
        namespace demo {
        class Shared {
        public:
            int GetValue() const { return 9; }
        };
        }
        "#,
    )
    .unwrap();

    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let units = compiler::collect_translation_units(&config).unwrap();

    assert_eq!(units.len(), 2);
    assert!(units.iter().any(|path| path.ends_with("entry.hpp")));
    assert!(units.iter().any(|path| path.ends_with("shared.hpp")));
}

#[test]
fn scoped_header_keeps_dir_translation_unit_context() {
    let fixture = temp_fixture_dir("scoped_dir_context");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/types.hpp"),
        r#"
        typedef unsigned long long SharedId;
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/entry.hpp"),
        r#"
        class Entry {
        public:
            SharedId GetId() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/tu.cpp"),
        r#"
        #include "types.hpp"
        #include "entry.hpp"
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let scoped = PipelineContext::new(config).scoped_to_header(fixture.join("include/entry.hpp"));
    let units = compiler::collect_translation_units(&scoped.config).unwrap();
    let parsed = parser::parse(&scoped).unwrap();

    assert_eq!(units.len(), 1);
    assert!(units[0].ends_with("tu.cpp"));
    assert!(parsed.records.iter().any(|record| record.name == "Entry"));
    assert!(
        !parsed
            .headers
            .iter()
            .any(|header| header.ends_with("types.hpp"))
    );
}

#[test]
fn dir_only_config_keeps_standalone_headers_even_when_sources_exist() {
    let fixture = temp_fixture_dir("mixed_tree_standalone_headers");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/A.hpp"),
        r#"
        class A {
        public:
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/B.hpp"),
        r#"
        class B {
        public:
            int GetValue() const { return 9; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/tu.cpp"),
        r#"
        #include "A.hpp"
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();

    assert!(parsed.records.iter().any(|record| record.name == "A"));
    assert!(parsed.records.iter().any(|record| record.name == "B"));
}

#[test]
fn dir_only_config_recurses_through_deeply_nested_input_tree() {
    let fixture = temp_fixture_dir("deep_nested_tree");
    let nested_dir = fixture.join("include/api/v1/models");
    fs::create_dir_all(&nested_dir).unwrap();

    fs::write(
        nested_dir.join("Thing.hpp"),
        r#"
        class Thing {
        public:
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/api/v1/Api.cpp"),
        r#"
        #include "models/Thing.hpp"
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let units = compiler::collect_translation_units(&config).unwrap();
    let parsed = parser::parse(&PipelineContext::new(config)).unwrap();

    assert_eq!(units.len(), 1);
    assert!(units[0].ends_with("include/api/v1/Api.cpp"));
    assert!(
        parsed
            .headers
            .iter()
            .any(|header| header.ends_with("include/api/v1/models/Thing.hpp"))
    );
    assert!(parsed.records.iter().any(|record| record.name == "Thing"));
}

#[test]
fn headers_only_config_uses_listed_headers_as_translation_units() {
    let fixture = temp_fixture_dir("headers_only_translation_units");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(fixture.join("include/A.hpp"), "class A {};").unwrap();
    fs::write(fixture.join("include/B.hpp"), "class B {};").unwrap();
    fs::write(fixture.join("include/C.hpp"), "class C {};").unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/B.hpp
    - include/A.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let units = compiler::collect_translation_units(&config).unwrap();

    assert_eq!(
        units,
        vec![
            fixture.join("include/B.hpp").canonicalize().unwrap(),
            fixture.join("include/A.hpp").canonicalize().unwrap(),
        ]
    );
}

#[test]
fn headers_only_config_collects_only_listed_header_declarations() {
    let fixture = temp_fixture_dir("headers_only_owned_set");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/entry.hpp"),
        r#"
        #include "dependency.hpp"

        class Entry {
        public:
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/dependency.hpp"),
        r#"
        class Dependency {
        public:
            int GetValue() const { return 9; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/entry.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let parsed = parser::parse(&PipelineContext::new(config)).unwrap();

    assert!(
        parsed
            .headers
            .iter()
            .any(|header| header.ends_with("entry.hpp"))
    );
    assert!(
        !parsed
            .headers
            .iter()
            .any(|header| header.ends_with("dependency.hpp"))
    );
    assert!(parsed.records.iter().any(|record| record.name == "Entry"));
    assert!(
        !parsed
            .records
            .iter()
            .any(|record| record.name == "Dependency")
    );
}
