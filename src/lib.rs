pub mod analysis;
pub mod cli;
pub mod codegen;
pub mod config;
pub mod domain;
pub mod parsing;
pub mod pipeline;

pub use codegen::c_abi as generator;
pub use codegen::go_facade as facade;
pub use codegen::ir_norm as ir;
pub use parsing::compiler;
pub use parsing::parser;
