use std::{collections::BTreeMap, ops::Deref, path::PathBuf};

use anyhow::Result;

use crate::{config::Config, domain::model_projection::ModelProjection};

#[derive(Debug, Clone)]
pub struct PipelineContext {
    pub config: Config,
    pub known_model_types: Vec<String>,
    pub known_enum_types: Vec<String>,
    pub preferred_model_aliases: BTreeMap<String, String>,
    pub known_model_projections: Vec<ModelProjection>,
    pub go_module: Option<String>,
    pub target_header: Option<PathBuf>,
    pub raw_clang_args: Vec<String>,
}

impl PipelineContext {
    pub fn new(config: Config) -> Self {
        let raw_clang_args = config.input.clang_args.clone();
        PipelineContext {
            raw_clang_args,
            config,
            known_model_types: vec![],
            known_enum_types: vec![],
            preferred_model_aliases: BTreeMap::new(),
            known_model_projections: vec![],
            go_module: None,
            target_header: None,
        }
    }

    pub fn with_go_module(mut self, go_module: Option<String>) -> Self {
        self.go_module = go_module;
        self
    }

    pub fn with_raw_clang_args(mut self, raw_clang_args: Vec<String>) -> Self {
        self.raw_clang_args = raw_clang_args;
        self
    }

    pub fn with_output_dir(mut self, output_dir: PathBuf) -> Self {
        self.config.output.dir = output_dir;
        self
    }

    pub fn with_known_model_types(mut self, known_model_types: Vec<String>) -> Self {
        self.known_model_types = known_model_types;
        self
    }

    pub fn with_known_enum_types(mut self, known_enum_types: Vec<String>) -> Self {
        self.known_enum_types = known_enum_types;
        self
    }

    pub fn with_preferred_model_aliases(
        mut self,
        preferred_model_aliases: BTreeMap<String, String>,
    ) -> Self {
        self.preferred_model_aliases = preferred_model_aliases;
        self
    }

    pub fn with_known_model_projections(
        mut self,
        known_model_projections: Vec<ModelProjection>,
    ) -> Self {
        self.known_model_projections = known_model_projections;
        self
    }

    /// Create a context scoped to a single header, adjusting output filenames.
    pub fn scoped_to_header(&self, header: PathBuf) -> Self {
        let scoped_config = self.config.scoped_to_header(&header);
        PipelineContext {
            config: scoped_config,
            known_model_types: self.known_model_types.clone(),
            known_enum_types: self.known_enum_types.clone(),
            preferred_model_aliases: self.preferred_model_aliases.clone(),
            known_model_projections: self.known_model_projections.clone(),
            go_module: self.go_module.clone(),
            target_header: Some(header),
            raw_clang_args: self.raw_clang_args.clone(),
        }
    }

    pub fn known_model_projection(&self, cpp_type: &str) -> Option<&ModelProjection> {
        let base = base_cpp_type_name(cpp_type);
        self.known_model_projections.iter().find(|projection| {
            let normalized = base_cpp_type_name(&projection.cpp_type);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
    }

    pub fn is_known_model_type(&self, cpp_type: &str) -> bool {
        let base = base_cpp_type_name(cpp_type);
        self.known_model_types.iter().any(|candidate| {
            let normalized = base_cpp_type_name(candidate);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
    }

    pub fn is_known_enum_type(&self, cpp_type: &str) -> bool {
        let base = enum_cpp_type_name(cpp_type);
        self.known_enum_types.iter().any(|candidate| {
            let normalized = enum_cpp_type_name(candidate);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
    }

    pub fn known_enum_go_type(&self, cpp_type: &str) -> Option<String> {
        let base = enum_cpp_type_name(cpp_type);
        self.known_enum_types
            .iter()
            .find(|candidate| {
                let normalized = enum_cpp_type_name(candidate);
                normalized == base
                    || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                    || base.rsplit("::").next().unwrap_or(&base) == normalized
            })
            .map(|candidate| {
                enum_cpp_type_name(candidate)
                    .rsplit("::")
                    .next()
                    .unwrap_or(candidate)
                    .to_string()
            })
    }

    pub fn raw_clang_args(&self) -> &[String] {
        &self.raw_clang_args
    }

    pub fn owner_marks_callable(&self, cpp_name: &str) -> bool {
        self.config
            .input
            .owner
            .iter()
            .any(|candidate| candidate.trim() == cpp_name)
    }

    pub fn from_config_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let (config, raw_clang_args) = Config::load_with_raw_clang_args(path)?;
        Ok(PipelineContext::new(config).with_raw_clang_args(raw_clang_args))
    }
}

impl Deref for PipelineContext {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

fn base_cpp_type_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

fn enum_cpp_type_name(value: &str) -> String {
    let base = base_cpp_type_name(value);
    base.strip_prefix("enum ")
        .unwrap_or(&base)
        .trim()
        .to_string()
}
