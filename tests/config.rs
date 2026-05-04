use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use cgo_gen::config::Config;
use cgo_gen::{generator, pipeline::context::PipelineContext};
#[cfg(unix)]
use std::os::unix::fs::symlink;

struct EnvGuard {
    key: String,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            env::remove_var(&self.key);
        }
    }
}

fn set_test_env(key: String, value: &str) -> EnvGuard {
    unsafe {
        env::set_var(&key, value);
    }
    EnvGuard { key }
}

fn normalize_expected_path(path: &Path) -> String {
    let value = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();
    if cfg!(windows) {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_test_dir(label: &str) -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_{label}_{}_{}",
        std::process::id(),
        TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_directory_example_config() -> PathBuf {
    let dir = temp_test_dir("directory_example");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/UserProfile.hpp"), "class UserProfile {};").unwrap();
    fs::write(dir.join("include/AdminUser.hpp"), "class AdminUser {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: pkg/sdk
"#,
    )
    .unwrap();

    config_path
}

fn write_model_record_dir_config() -> PathBuf {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/model_record/include")
        .display()
        .to_string()
        .replace('\\', "/");
    let dir = temp_test_dir("model_record_dir_config");
    let config_path = dir.join("cppgo-wrap.yaml");

    fs::write(
        &config_path,
        format!(
            r#"
version: 1
input:
  dir: '{fixture_dir}'
  clang_args:
    - -std=c++11
    - -x
    - c++
    - '-I{fixture_dir}'
output:
  dir: pkg/model-record
"#
        ),
    )
    .unwrap();

    config_path
}

#[test]
fn loads_yaml_config() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
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
    assert_eq!(config.version, Some(1));
    assert_eq!(
        config.input.dir.as_ref(),
        Some(&dir.join("include").canonicalize().unwrap())
    );
    assert_eq!(config.output.go_version, "1.26");
    assert_eq!(config.output.header, "foo_wrapper.h");
}

#[test]
fn loads_output_go_version_override() {
    let dir = temp_test_dir("go_version_override");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
  go_version: "1.24"
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.output.go_version, "1.24");
}

#[test]
fn loads_dir_only_input_config() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_dir_only_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/model.hpp"), "class ModelThing {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
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
    let expected_dir = dir.join("include").canonicalize().unwrap();
    assert_eq!(config.input.dir.as_ref(), Some(&expected_dir));
    assert_eq!(config.output.header, "model_wrapper.h");
    assert_eq!(config.output.source, "model_wrapper.cpp");
    assert_eq!(config.output.ir, "model_wrapper.ir.yaml");
}

#[test]
fn loads_input_owner_overrides() {
    let dir = temp_test_dir("owner_overrides");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/factory.hpp"), "class Factory {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  owner:
    - DBHandlerFactory::CreateHandler
    - WidgetFactory::Create
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(
        config.input.owner,
        vec![
            "DBHandlerFactory::CreateHandler".to_string(),
            "WidgetFactory::Create".to_string()
        ]
    );
}

#[test]
fn rejects_removed_allow_diagnostics_key() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_removed_allow_diagnostics_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  allow_diagnostics: true
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("failed to parse YAML config"));
    assert!(error.contains("allow_diagnostics"));
}

#[test]
fn rejects_removed_input_include_dirs_key() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_removed_include_dirs_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  include_dirs:
    - include
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("failed to parse YAML config"));
    assert!(error.contains("include_dirs"));
}

#[test]
fn rejects_config_without_dir_or_headers() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_missing_input_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = format!("{:#}", Config::load(&config_path).unwrap_err());
    assert!(error.contains("config.input.dir or config.input.headers must be set"));
}

#[test]
fn loads_headers_only_input_config_relative_to_config_dir() {
    let dir = temp_test_dir("headers_only");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/alpha.hpp"), "class Alpha {};").unwrap();
    fs::write(dir.join("include/beta.h"), "class Beta {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/alpha.hpp
    - include/beta.h
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.input.dir, None);
    assert_eq!(
        config.input.headers,
        vec![
            dir.join("include/alpha.hpp").canonicalize().unwrap(),
            dir.join("include/beta.h").canonicalize().unwrap(),
        ]
    );
    assert_eq!(config.discovered_headers().unwrap(), config.input.headers);
}

#[test]
fn headers_only_single_header_derives_output_filenames() {
    let dir = temp_test_dir("headers_single_output_defaults");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/WidgetThing.hpp"), "class WidgetThing {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/WidgetThing.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.output.header, "widget_thing_wrapper.h");
    assert_eq!(config.output.source, "widget_thing_wrapper.cpp");
    assert_eq!(config.output.ir, "widget_thing_wrapper.ir.yaml");
}

#[test]
fn rejects_config_with_dir_and_headers() {
    let dir = temp_test_dir("dir_and_headers");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  headers:
    - include/foo.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = format!("{:#}", Config::load(&config_path).unwrap_err());
    assert!(error.contains("config.input.dir and config.input.headers are mutually exclusive"));
}

#[test]
fn rejects_missing_input_header() {
    let dir = temp_test_dir("missing_header");

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/missing.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = format!("{:#}", Config::load(&config_path).unwrap_err());
    assert!(error.contains("input.headers entry not found"));
}

#[test]
fn rejects_unsupported_input_header_extension() {
    let dir = temp_test_dir("unsupported_header_extension");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.txt"), "class Foo {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.txt
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = format!("{:#}", Config::load(&config_path).unwrap_err());
    assert!(error.contains("input.headers entry must be a supported header"));
}

#[test]
fn rejects_duplicate_input_headers_after_canonicalization() {
    let dir = temp_test_dir("duplicate_headers");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.hpp
    - include/../include/foo.hpp
output:
  dir: gen
  header: custom_wrapper.h
  source: custom_wrapper.cpp
  ir: custom_wrapper.ir.yaml
"#,
    )
    .unwrap();

    let error = format!("{:#}", Config::load(&config_path).unwrap_err());
    assert!(error.contains("input.headers entry is duplicated"));
}

#[test]
fn rejects_removed_reserved_config_keys() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_removed_reserved_keys_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
project_root: .
input:
  dir: include
files:
  model:
policies:
  string_mode: c_str
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("failed to parse YAML config"));
}

#[test]
fn derives_output_filenames_from_header_stem() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_basename_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
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
    assert_eq!(config.output.header, "foo_wrapper.h");
    assert_eq!(config.output.source, "foo_wrapper.cpp");
    assert_eq!(config.output.ir, "foo_wrapper.ir.yaml");
    assert_eq!(config.go_filename("Foo"), "foo_wrapper.go");
    assert!(config.output_dir().ends_with("gen"));
}

#[test]
fn scoped_header_from_dir_only_config_switches_back_to_header_mode() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_scoped_dir_only_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/model.hpp"), "class ModelThing {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
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
    let scoped = config.scoped_to_header(&config.input.dir.as_ref().unwrap().join("model.hpp"));

    assert_eq!(scoped.input.dir, config.input.dir);
    assert_eq!(scoped.output.header, "model_wrapper.h");
    assert_eq!(scoped.output.source, "model_wrapper.cpp");
    assert_eq!(scoped.output.ir, "model_wrapper.ir.yaml");
}

#[test]
fn loads_directory_wrapper_example_config() {
    let config_path = write_directory_example_config();
    let config = Config::load(&config_path).unwrap();

    assert_eq!(
        config.input.dir.as_ref(),
        Some(
            &config_path
                .parent()
                .unwrap()
                .join("include")
                .canonicalize()
                .unwrap()
        )
    );
    assert!(config.output.dir.ends_with(Path::new("pkg").join("sdk")));
}

#[test]
fn directory_wrapper_example_scopes_per_header_output_names() {
    let config_path = write_directory_example_config();
    let config = Config::load(&config_path).unwrap();

    let dir = config.input.dir.as_ref().unwrap();
    let profile = config.scoped_to_header(&dir.join("UserProfile.hpp"));
    let admin = config.scoped_to_header(&dir.join("AdminUser.hpp"));

    assert_eq!(profile.output.header, "user_profile_wrapper.h");
    assert_eq!(profile.output.source, "user_profile_wrapper.cpp");
    assert_eq!(profile.output.ir, "user_profile_wrapper.ir.yaml");
    assert_eq!(profile.go_filename(""), "user_profile_wrapper.go");

    assert_eq!(admin.output.header, "admin_user_wrapper.h");
    assert_eq!(admin.output.source, "admin_user_wrapper.cpp");
    assert_eq!(admin.output.ir, "admin_user_wrapper.ir.yaml");
    assert_eq!(admin.go_filename(""), "admin_user_wrapper.go");
}

#[test]
fn resolves_relative_clang_include_args_from_config_dir() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_relative_clang_args_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/inc")).unwrap();
    fs::create_dir_all(dir.join("deps/sys")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  clang_args:
    - -Ideps/inc
    - -isystem
    - deps/sys
output:
  dir: gen
"#,
    )
    .unwrap();

    let (config, raw_clang_args) = Config::load_with_raw_clang_args(&config_path).unwrap();

    assert_eq!(
        config.input.clang_args,
        vec![
            format!("-I{}", normalize_expected_path(&dir.join("deps/inc"))),
            "-isystem".to_string(),
            normalize_expected_path(&dir.join("deps/sys")),
        ]
    );
    assert_eq!(
        raw_clang_args.as_slice(),
        &[
            "-Ideps/inc".to_string(),
            "-isystem".to_string(),
            "deps/sys".to_string(),
        ]
    );
}

#[test]
fn expands_env_tokens_in_clang_args() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_env_clang_args_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/inc")).unwrap();
    fs::create_dir_all(dir.join("deps/sys")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let plain_flag = format!("C_GO_TEST_CLANG_ARG_FLAG_{}", std::process::id());
    let inline_include = format!("C_GO_TEST_CLANG_ARG_INCLUDE_{}", std::process::id());
    let system_include = format!("C_GO_TEST_CLANG_ARG_SYSTEM_{}", std::process::id());
    let braced_flag = format!("C_GO_TEST_CLANG_ARG_BRACED_{}", std::process::id());

    let _plain_flag = set_test_env(plain_flag.clone(), "-DMODE=1");
    let _inline_include = set_test_env(inline_include.clone(), "deps/inc");
    let _system_include = set_test_env(system_include.clone(), "deps/sys");
    let _braced_flag = set_test_env(braced_flag.clone(), "-Winvalid-offsetof");

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        format!(
            r#"
version: 1
input:
  dir: include
  clang_args:
    - ${plain_flag}
    - -I${{{inline_include}}}
    - -isystem
    - $({system_include})
    - ${{{braced_flag}}}
output:
  dir: gen
"#
        ),
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();

    assert_eq!(
        config.input.clang_args,
        vec![
            "-DMODE=1".to_string(),
            format!("-I{}", normalize_expected_path(&dir.join("deps/inc"))),
            "-isystem".to_string(),
            normalize_expected_path(&dir.join("deps/sys")),
            "-Winvalid-offsetof".to_string(),
        ]
    );
}

#[test]
fn rejects_missing_env_tokens_in_clang_args() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_missing_env_clang_args_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let missing = format!("C_GO_TEST_MISSING_CLANG_ARG_{}", std::process::id());
    unsafe {
        env::remove_var(&missing);
    }

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        format!(
            r#"
version: 1
input:
  dir: include
  clang_args:
    - -I${missing}
output:
  dir: gen
"#
        ),
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("input.clang_args"));
    assert!(error.contains(&missing));
}

#[test]
fn loads_gen_model_config() {
    let config = Config::load(write_model_record_dir_config()).unwrap();

    assert!(
        config
            .input
            .dir
            .as_ref()
            .is_some_and(|path| path.is_absolute())
    );
    assert!(
        config
            .output
            .dir
            .ends_with(Path::new("pkg").join("model-record"))
    );
}

#[cfg(unix)]
#[test]
fn resolves_symlinked_external_project_paths_from_config_dir() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_symlink_project_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);

    let external = dir.join("external-sdk");
    fs::create_dir_all(external.join("include")).unwrap();
    fs::write(external.join("include/foo.hpp"), "int foo();").unwrap();

    let workspace = dir.join("workspace");
    fs::create_dir_all(workspace.join("third_party")).unwrap();
    symlink(&external, workspace.join("third_party/external-sdk")).unwrap();

    let config_path = workspace.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: third_party/external-sdk/include
  clang_args:
    - -Ithird_party/external-sdk/include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let expected_include = external.join("include").canonicalize().unwrap();

    assert_eq!(config.input.dir.as_ref(), Some(&expected_include));
    assert_eq!(
        config.input.clang_args,
        vec![format!("-I{}", normalize_expected_path(&expected_include))]
    );
}

#[test]
fn preserves_raw_clang_args_without_injection() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_raw_clang_args_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("manual/inc")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  clang_args:
    - -Imanual/inc
    - -DMODE=1
output:
  dir: gen
"#,
    )
    .unwrap();

    let (config, raw_clang_args) = Config::load_with_raw_clang_args(&config_path).unwrap();
    let expected_manual = format!("-I{}", normalize_expected_path(&dir.join("manual/inc")));

    assert_eq!(
        raw_clang_args.as_slice(),
        &["-Imanual/inc".to_string(), "-DMODE=1".to_string()]
    );
    assert_eq!(
        config.input.clang_args,
        vec![expected_manual, "-DMODE=1".to_string()]
    );
}

#[test]
fn resolves_relative_ldflags_from_config_dir() {
    let dir = temp_test_dir("relative_ldflags");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/lib")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  ldflags:
    - -Ldeps/lib
    - -lfoo
    - -L
    - deps/lib
    - lib/libbar.a
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();

    assert_eq!(
        config.input.ldflags,
        vec![
            format!("-L{}", normalize_expected_path(&dir.join("deps/lib"))),
            "-lfoo".to_string(),
            "-L".to_string(),
            normalize_expected_path(&dir.join("deps/lib")),
            normalize_expected_path(&dir.join("lib/libbar.a")),
        ]
    );
}

#[test]
fn expands_env_tokens_in_ldflags() {
    let dir = temp_test_dir("env_ldflags");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/lib")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let inline_lib = format!("C_GO_TEST_LDFLAG_LIB_{}", std::process::id());
    let plain_lib = format!("C_GO_TEST_LDFLAG_PLAIN_{}", std::process::id());
    let _inline_lib = set_test_env(inline_lib.clone(), "deps/lib");
    let _plain_lib = set_test_env(plain_lib.clone(), "-lfoo");

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        format!(
            r#"
version: 1
input:
  dir: include
  ldflags:
    - -L${{{inline_lib}}}
    - ${plain_lib}
output:
  dir: gen
"#
        ),
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();

    assert_eq!(
        config.input.ldflags,
        vec![
            format!("-L{}", normalize_expected_path(&dir.join("deps/lib"))),
            "-lfoo".to_string(),
        ]
    );
}

#[test]
fn emits_resolved_ldflags_into_build_flags_go() {
    let dir = temp_test_dir("build_flags_ldflags");
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/lib")).unwrap();
    fs::write(
        dir.join("include/Foo.hpp"),
        r#"
        class Foo {
        public:
            Foo() = default;
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
  ldflags:
    - -Ldeps/lib
    - -lfoo
    - lib/libbar.a
output:
  dir: gen
"#,
    )
    .unwrap();

    let ctx = PipelineContext::from_config_path(&config_path)
        .unwrap()
        .with_go_module(Some("example.com/ldflags/demo".to_string()));
    generator::generate_all(&ctx, true).unwrap();

    let build_flags = fs::read_to_string(ctx.output_dir().join("build_flags.go")).unwrap();
    let resolved_lib_dir = normalize_expected_path(&dir.join("deps/lib"));

    assert!(build_flags.contains("#cgo LDFLAGS:"));
    assert!(build_flags.contains(&format!(
        "-L{resolved_lib_dir} -lfoo ${{SRCDIR}}/../lib/libbar.a"
    )));
}
