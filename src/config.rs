use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub version: Option<u32>,
    pub input: InputConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InputConfig {
    #[serde(default)]
    pub dir: Option<PathBuf>,
    #[serde(default)]
    pub clang_args: Vec<String>,
    #[serde(default)]
    pub ldflags: Vec<String>,
    #[serde(default)]
    pub owner: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_dir")]
    pub dir: PathBuf,
    #[serde(default = "default_header_name")]
    pub header: String,
    #[serde(default = "default_source_name")]
    pub source: String,
    #[serde(default = "default_ir_name")]
    pub ir: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            dir: default_output_dir(),
            header: default_header_name(),
            source: default_source_name(),
            ir: default_ir_name(),
        }
    }
}

pub const WRAPPER_PREFIX: &str = "cgowrap";

fn default_output_dir() -> PathBuf {
    PathBuf::from("gen")
}
fn default_header_name() -> String {
    "wrapper.h".to_string()
}
fn default_source_name() -> String {
    "wrapper.cpp".to_string()
}
fn default_ir_name() -> String {
    "wrapper.ir.yaml".to_string()
}

fn resolve_ldflags(flags: &mut Vec<String>, base_dir: &Path) -> Result<()> {
    let mut resolved = Vec::with_capacity(flags.len());
    let mut index = 0;

    while index < flags.len() {
        let arg = &flags[index];

        if arg == "-L" {
            resolved.push(arg.clone());
            if let Some(value) = flags.get(index + 1) {
                let expanded = expand_env_vars_in_str(value, "input.ldflags")?;
                resolved.push(resolve_relative_clang_path_arg(&expanded, base_dir)?);
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("-L") {
            let expanded = expand_env_vars_in_str(value, "input.ldflags")?;
            resolved.push(format!(
                "-L{}",
                resolve_relative_clang_path_arg(&expanded, base_dir)?
            ));
            index += 1;
            continue;
        }

        resolved.push(expand_env_vars_in_str(arg, "input.ldflags")?);
        index += 1;
    }

    *flags = resolved;
    Ok(())
}

fn resolve_relative_clang_args(args: &mut Vec<String>, base_dir: &Path) -> Result<()> {
    let mut resolved = Vec::with_capacity(args.len());
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if arg == "-I" || arg == "-isystem" {
            resolved.push(arg.clone());
            if let Some(value) = args.get(index + 1) {
                resolved.push(resolve_relative_clang_path_arg(value, base_dir)?);
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("-I") {
            resolved.push(format!(
                "-I{}",
                resolve_relative_clang_path_arg(value, base_dir)?
            ));
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("-isystem") {
            resolved.push(format!(
                "-isystem{}",
                resolve_relative_clang_path_arg(value, base_dir)?
            ));
            index += 1;
            continue;
        }

        resolved.push(expand_env_vars_in_str(arg, "input.clang_args")?);
        index += 1;
    }

    *args = resolved;
    Ok(())
}

fn resolve_relative_clang_path_arg(value: &str, base_dir: &Path) -> Result<String> {
    let value = expand_env_vars_in_str(value, "input.clang_args")?;

    if value.is_empty() {
        return Ok(String::new());
    }

    let path = Path::new(&value);
    if path.is_absolute() {
        return Ok(normalize_clang_config_path(path));
    }

    let joined = base_dir.join(path);
    Ok(normalize_clang_config_path(
        &joined.canonicalize().unwrap_or(joined),
    ))
}

/// `-L${ICORE_BASE}/lib` 처럼 문자열 중간에 포함된 `${VAR}` / `$(VAR)` / `$VAR` 패턴을 모두 치환합니다.
fn expand_env_vars_in_str(value: &str, context: &str) -> Result<String> {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(dollar) = rest.find('$') {
        result.push_str(&rest[..dollar]);
        rest = &rest[dollar..];

        // ${VAR} 형식
        if let Some(inner) = rest.strip_prefix("${") {
            if let Some(end) = inner.find('}') {
                let name = &inner[..end];
                if valid_env_name(name).is_some() {
                    match env::var(name) {
                        Ok(val) => {
                            result.push_str(&val);
                            rest = &inner[end + 1..];
                            continue;
                        }
                        Err(env::VarError::NotPresent) => {
                            bail!(
                                "environment variable `{name}` referenced in {context} is not set"
                            )
                        }
                        Err(env::VarError::NotUnicode(_)) => bail!(
                            "environment variable `{name}` referenced in {context} is not valid unicode"
                        ),
                    }
                }
            }
        }

        // $(VAR) 형식
        if let Some(inner) = rest.strip_prefix("$(") {
            if let Some(end) = inner.find(')') {
                let name = &inner[..end];
                if valid_env_name(name).is_some() {
                    match env::var(name) {
                        Ok(val) => {
                            result.push_str(&val);
                            rest = &inner[end + 1..];
                            continue;
                        }
                        Err(env::VarError::NotPresent) => {
                            bail!(
                                "environment variable `{name}` referenced in {context} is not set"
                            )
                        }
                        Err(env::VarError::NotUnicode(_)) => bail!(
                            "environment variable `{name}` referenced in {context} is not valid unicode"
                        ),
                    }
                }
            }
        }

        // $VAR 형식
        {
            let tail = &rest[1..];
            let name_len = tail
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(tail.len());
            let name = &tail[..name_len];
            if valid_env_name(name).is_some() {
                match env::var(name) {
                    Ok(val) => {
                        result.push_str(&val);
                        rest = &tail[name_len..];
                        continue;
                    }
                    Err(env::VarError::NotPresent) => {
                        bail!("environment variable `{name}` referenced in {context} is not set")
                    }
                    Err(env::VarError::NotUnicode(_)) => bail!(
                        "environment variable `{name}` referenced in {context} is not valid unicode"
                    ),
                }
            }
        }

        // 치환 못한 $ 는 그대로
        result.push('$');
        rest = &rest[1..];
    }

    result.push_str(rest);
    Ok(result)
}

fn valid_env_name(value: &str) -> Option<&str> {
    let mut chars = value.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        Some(value)
    } else {
        None
    }
}

fn normalize_clang_config_path(path: &Path) -> String {
    let value = path.display().to_string();
    if cfg!(windows) {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

fn resolve_path(path: &mut PathBuf, base_dir: &Path) {
    if path.is_relative() {
        *path = base_dir.join(&*path);
    }
    if let Ok(canonical) = path.canonicalize() {
        *path = canonical;
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, value: PathBuf) {
    if !paths.iter().any(|candidate| candidate == &value) {
        paths.push(value);
    }
}

fn is_supported_header_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

fn collect_headers_from_dir(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        bail!("input.dir not found: {}", dir.display());
    }
    if !dir.is_dir() {
        bail!("input.dir must be a directory: {}", dir.display());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read header directory: {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to list header directory: {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_headers_from_dir(&path, output)?;
            continue;
        }
        if !is_supported_header_path(&path) {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        push_unique_path(output, canonical);
    }

    Ok(())
}
impl Config {
    pub fn load_with_raw_clang_args(path: impl AsRef<Path>) -> Result<(Self, Vec<String>)> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let raw_clang_args = raw_clang_args_from_yaml(&raw)
            .with_context(|| format!("failed to parse raw clang args: {}", path.display()))?;
        let mut config: Self = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;
        config.resolve_relative_paths(path)?;
        config.validate()?;
        Ok((config, raw_clang_args))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::load_with_raw_clang_args(path)?.0)
    }

    pub fn discovered_headers(&self) -> Result<Vec<PathBuf>> {
        let mut headers = Vec::new();
        if let Some(dir) = &self.input.dir {
            collect_headers_from_dir(dir, &mut headers)?;
        }
        Ok(headers)
    }

    pub fn uses_default_output_names(&self) -> bool {
        self.output.header == default_header_name()
            && self.output.source == default_source_name()
            && self.output.ir == default_ir_name()
    }

    pub fn scoped_to_header(&self, header: &Path) -> Self {
        let mut scoped = self.clone();
        scoped.apply_output_defaults_for_header(header);
        scoped
    }

    fn resolve_relative_paths(&mut self, config_path: &Path) -> Result<()> {
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        if let Some(dir) = &mut self.input.dir {
            resolve_path(dir, base_dir);
        }
        resolve_relative_clang_args(&mut self.input.clang_args, base_dir)?;
        resolve_ldflags(&mut self.input.ldflags, base_dir)?;
        if self.output.dir.is_relative() {
            self.output.dir = base_dir.join(&self.output.dir);
        }
        self.apply_output_defaults();
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        let Some(dir) = &self.input.dir else {
            bail!("config.input.dir must be set");
        };
        if dir.exists() && !dir.is_dir() {
            bail!(
                "config.input.dir must point to a directory: {}",
                dir.display()
            );
        }
        Ok(())
    }

    pub fn go_filename(&self, _value: &str) -> String {
        let stem = Path::new(&self.output.header)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("wrapper");
        format!("{stem}.go")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.output.dir.clone()
    }

    pub fn generated_header_include(&self, header: &str) -> String {
        header.to_string()
    }

    fn apply_output_defaults(&mut self) {
        let Ok(headers) = self.discovered_headers() else {
            return;
        };
        if headers.len() == 1 {
            self.apply_output_defaults_for_header(&headers[0]);
        }
    }

    fn apply_output_defaults_for_header(&mut self, header: &Path) {
        if !self.uses_default_output_names() {
            return;
        }
        let Some(stem) = header.file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        let basename = format!("{}_wrapper", to_snake_case(stem));
        self.output.header = format!("{basename}.h");
        self.output.source = format!("{basename}.cpp");
        self.output.ir = format!("{basename}.ir.yaml");
    }
}

#[derive(Debug, Deserialize, Default)]
struct RuntimeInputConfig {
    #[serde(default)]
    clang_args: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RuntimeConfig {
    #[serde(default)]
    input: RuntimeInputConfig,
}

fn raw_clang_args_from_yaml(raw: &str) -> Result<Vec<String>> {
    Ok(serde_yaml::from_str::<RuntimeConfig>(raw)?.input.clang_args)
}

fn to_snake_case(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return String::new();
    }

    let mut tokens = Vec::new();
    let mut start = 0;
    for index in 1..chars.len() {
        let prev = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        let boundary = (prev.is_lowercase() && current.is_uppercase())
            || (prev.is_ascii_digit() && !current.is_ascii_digit())
            || (!prev.is_ascii_digit() && current.is_ascii_digit())
            || (prev.is_uppercase()
                && current.is_uppercase()
                && next.map(|ch| ch.is_lowercase()).unwrap_or(false));

        if boundary {
            tokens.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    tokens.push(chars[start..].iter().collect::<String>());

    tokens
        .into_iter()
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}
