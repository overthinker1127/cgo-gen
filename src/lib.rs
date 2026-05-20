mod analysis;
#[path = "cli.rs"]
mod cli_impl;
mod codegen;
mod config;
mod domain;
mod parsing;
mod pipeline;

pub use config::{Config, InputConfig, OutputConfig};
pub use domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind, RecordKind};
pub use parsing::macros::MacroConstantKind;
pub use pipeline::context::PipelineContext;

pub mod cli {
    pub use crate::cli_impl::run;
}

pub mod compiler {
    pub use crate::parsing::compiler::*;
}

pub mod facade {
    pub use crate::codegen::go_facade::*;
}

pub mod generator {
    pub use crate::codegen::c_abi::*;
}

pub mod ir {
    pub use crate::codegen::ir_norm::*;
}

pub mod parser {
    pub use crate::parsing::parser::*;
}
