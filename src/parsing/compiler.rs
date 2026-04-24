use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};

use crate::config::Config;

pub fn collect_clang_args(config: &Config, parse_entry: &Path) -> Result<Vec<String>> {
    let mut args = config.input.clang_args.clone();

    if !args.iter().any(|arg| arg == "-x") {
        args.push("-x".to_string());
        args.push("c++".to_string());
    }

    if !args.iter().any(|arg| arg.starts_with("-std=")) {
        args.push("-std=c++17".to_string());
    }

    add_parse_entry_parent_include(&mut args, parse_entry);
    add_platform_fallback_sysroot(&mut args);
    add_platform_fallback_includes(&mut args);

    Ok(args)
}

fn add_parse_entry_parent_include(args: &mut Vec<String>, parse_entry: &Path) {
    add_header_parent_include(args, parse_entry);
}

pub fn collect_translation_units(config: &Config) -> Result<Vec<PathBuf>> {
    let Some(dir) = &config.input.dir else {
        return Ok(Vec::new());
    };
    scan_dir_translation_units(dir)
}

fn add_header_parent_include(args: &mut Vec<String>, header: &Path) {
    let Some(parent) = header.parent() else {
        return;
    };
    let include = normalize_clang_path(parent);
    let mut has_parent_include = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-I" || arg == "-isystem" {
            if iter.next().is_some_and(|value| value == &include) {
                has_parent_include = true;
                break;
            }
            continue;
        }
        if arg == &format!("-I{include}") || arg == &format!("-isystem{include}") {
            has_parent_include = true;
            break;
        }
    }
    if !has_parent_include {
        args.push(format!("-I{include}"));
    }
}

fn add_platform_fallback_includes(args: &mut Vec<String>) {
    for include in discover_platform_fallback_include_dirs() {
        let include = normalize_clang_path(&include);
        let already_present = args
            .iter()
            .any(|arg| arg == &format!("-I{include}") || arg == &format!("-isystem{include}"));
        if !already_present {
            args.push(format!("-isystem{include}"));
        }
    }
}

fn add_platform_fallback_sysroot(args: &mut Vec<String>) {
    if env::consts::OS != "macos" || args.iter().any(|arg| arg == "-isysroot") {
        return;
    }

    let Some(sysroot) = discover_macos_sdk_path() else {
        return;
    };
    let sysroot = normalize_clang_path(&sysroot);
    args.push("-isysroot".to_string());
    args.push(sysroot);
}

fn discover_platform_fallback_include_dirs() -> Vec<PathBuf> {
    match env::consts::OS {
        "windows" => discover_windows_fallback_include_dirs(),
        "macos" => discover_macos_fallback_include_dirs(),
        "linux" => discover_linux_fallback_include_dirs(),
        _ => Vec::new(),
    }
}

fn discover_macos_fallback_include_dirs() -> Vec<PathBuf> {
    let mut includes = Vec::new();

    if let Some(resource_dir) = discover_command_output_dir(&["clang++", "-print-resource-dir"]) {
        includes.push(resource_dir.join("include"));
    }

    if let Some(developer_dir) = discover_command_output_dir(&["xcode-select", "-p"]) {
        includes.extend(macos_developer_include_candidates(&developer_dir));
    }

    if let Some(sdk_path) = discover_macos_sdk_path() {
        includes.extend(macos_sdk_include_candidates(&sdk_path));
    }

    if let Some(toolchain_bin) = discover_command_output_dir(&["xcrun", "--find", "clang++"]) {
        includes.extend(macos_toolchain_bin_include_candidates(&toolchain_bin));
    }

    includes
        .into_iter()
        .filter(|path| path.exists())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn macos_developer_include_candidates(developer_dir: &Path) -> Vec<PathBuf> {
    vec![
        developer_dir.join("Toolchains/XcodeDefault.xctoolchain/usr/include/c++/v1"),
        developer_dir.join("Toolchains/XcodeDefault.xctoolchain/usr/include"),
    ]
}

fn macos_sdk_include_candidates(sdk_path: &Path) -> Vec<PathBuf> {
    vec![
        sdk_path.join("usr/include/c++/v1"),
        sdk_path.join("usr/include"),
    ]
}

fn macos_toolchain_bin_include_candidates(clangxx_path: &Path) -> Vec<PathBuf> {
    let Some(toolchain_dir) = clangxx_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    else {
        return Vec::new();
    };

    vec![
        toolchain_dir.join("usr/include/c++/v1"),
        toolchain_dir.join("usr/include"),
    ]
}

fn discover_macos_sdk_path() -> Option<PathBuf> {
    env::var_os("SDKROOT")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| discover_command_output_dir(&["xcrun", "--show-sdk-path"]))
}

fn discover_windows_fallback_include_dirs() -> Vec<PathBuf> {
    let roots = [
        PathBuf::from("C:/msys64/ucrt64/lib/clang"),
        PathBuf::from("C:/Program Files/LLVM/lib/clang"),
    ];

    roots
        .into_iter()
        .filter_map(|root| latest_versioned_include_dir(&root))
        .filter(|path| path.exists())
        .collect()
}

fn discover_linux_fallback_include_dirs() -> Vec<PathBuf> {
    let mut includes = Vec::new();

    includes.extend(discover_linux_driver_include_dirs());

    if let Some(resource_dir) = discover_command_output_dir(&["clang", "-print-resource-dir"])
        .map(|dir| dir.join("include"))
    {
        includes.push(resource_dir);
    }

    if let Some(resource_dir) = discover_command_output_dir(&["clang++", "-print-resource-dir"])
        .map(|dir| dir.join("include"))
    {
        includes.push(resource_dir);
    }

    if let Some(gcc_include) = discover_command_output_dir(&["c++", "-print-file-name=include"]) {
        includes.push(gcc_include);
    }

    if let Some(gcc_include) = discover_command_output_dir(&["g++", "-print-file-name=include"]) {
        includes.push(gcc_include);
    }

    if let Some(sysroot) = discover_command_output_dir(&["c++", "-print-sysroot"]) {
        includes.extend(linux_sysroot_include_candidates(&sysroot));
    }

    if let Some(sysroot) = discover_command_output_dir(&["g++", "-print-sysroot"]) {
        includes.extend(linux_sysroot_include_candidates(&sysroot));
    }

    includes.extend([
        PathBuf::from("/usr/include"),
        PathBuf::from("/usr/local/include"),
        PathBuf::from("/usr/include/x86_64-linux-gnu"),
    ]);

    includes
        .into_iter()
        .filter(|path| path.exists())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn discover_linux_driver_include_dirs() -> Vec<PathBuf> {
    let candidates = [
        ["clang++", "-E", "-x", "c++", "-", "-v"],
        ["clang++-18", "-E", "-x", "c++", "-", "-v"],
        ["c++", "-E", "-x", "c++", "-", "-v"],
        ["g++", "-E", "-x", "c++", "-", "-v"],
    ];

    for command in candidates {
        let (program, args) = command.split_first().expect("driver candidate");
        let Ok(output) = Command::new(program).args(args).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let parsed = parse_driver_include_search_list(&String::from_utf8_lossy(&output.stderr));
        if !parsed.is_empty() {
            return parsed;
        }
    }

    Vec::new()
}

fn parse_driver_include_search_list(stderr: &str) -> Vec<PathBuf> {
    let mut includes = Vec::new();
    let mut in_search_list = false;

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed == "#include <...> search starts here:" {
            in_search_list = true;
            continue;
        }
        if trimmed == "End of search list." {
            break;
        }
        if !in_search_list || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("(framework directory)") {
            continue;
        }

        let candidate = PathBuf::from(trimmed);
        if candidate.exists() {
            includes.push(candidate);
        }
    }

    includes
}

fn linux_sysroot_include_candidates(sysroot: &Path) -> Vec<PathBuf> {
    if sysroot.as_os_str().is_empty() {
        return Vec::new();
    }

    vec![
        sysroot.join("usr/include"),
        sysroot.join("usr/local/include"),
        sysroot.join("include"),
        sysroot.join("include-fixed"),
    ]
}

fn discover_command_output_dir(command_with_args: &[&str]) -> Option<PathBuf> {
    let (program, args) = command_with_args.split_first()?;
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);
    if path.exists() { Some(path) } else { None }
}

fn latest_versioned_include_dir(root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("include"))
        .filter(|path| path.join("mm_malloc.h").exists())
        .max()
}

fn scan_dir_translation_units(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut source_units = BTreeSet::new();
    let mut header_units = BTreeSet::new();
    scan_dir_translation_units_recursive(dir, &mut source_units, &mut header_units)?;

    if !source_units.is_empty() {
        Ok(source_units.into_iter().collect())
    } else {
        Ok(header_units.into_iter().collect())
    }
}

fn normalize_clang_path(path: &Path) -> String {
    let value = path.display().to_string();
    if env::consts::OS == "windows" {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

fn scan_dir_translation_units_recursive(
    dir: &Path,
    source_units: &mut BTreeSet<PathBuf>,
    header_units: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read input directory: {}", dir.display()))?;

    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            scan_dir_translation_units_recursive(&path, source_units, header_units)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if is_source_translation_unit_file(&path) {
            source_units.insert(path.canonicalize().unwrap_or(path));
        } else if is_header_file(&path) {
            header_units.insert(path.canonicalize().unwrap_or(path));
        }
    }

    Ok(())
}

fn is_source_translation_unit_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "cc" | "cpp" | "cxx")
    )
}

fn is_header_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

pub fn ensure_parse_entry_exists(parse_entry: &Path) -> Result<()> {
    if !parse_entry.exists() {
        anyhow::bail!("parse entry not found: {}", parse_entry.display());
    }
    Ok(())
}

pub fn ensure_header_exists(path: &Path) -> Result<()> {
    ensure_parse_entry_exists(path)
}

#[cfg(test)]
mod tests {
    use super::parse_driver_include_search_list;
    use std::path::PathBuf;

    #[test]
    fn parses_driver_include_search_list_from_verbose_output() {
        let stderr = r#"
#include "..." search starts here:
#include <...> search starts here:
 /usr/include/c++/13
 /usr/include/x86_64-linux-gnu/c++/13
 /usr/lib/llvm-18/lib/clang/18/include
 /usr/local/include
 /usr/include/x86_64-linux-gnu
 /usr/include
End of search list.
"#;

        let includes = parse_driver_include_search_list(stderr);
        assert_eq!(
            includes,
            vec![
                PathBuf::from("/usr/include/c++/13"),
                PathBuf::from("/usr/include/x86_64-linux-gnu/c++/13"),
                PathBuf::from("/usr/lib/llvm-18/lib/clang/18/include"),
                PathBuf::from("/usr/local/include"),
                PathBuf::from("/usr/include/x86_64-linux-gnu"),
                PathBuf::from("/usr/include"),
            ]
            .into_iter()
            .filter(|path| path.exists())
            .collect::<Vec<_>>()
        );
    }
}
