use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::{config::Config, generator, ir, pipeline::context::PipelineContext};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Generate conservative C ABI wrappers from C++ headers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value_t = false)]
        dump_ir: bool,
        #[arg(long)]
        go_module: Option<String>,
    },
    Ir {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = IrFormat::Yaml)]
        format: IrFormat,
    },
    Check {
        #[arg(long)]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum IrFormat {
    Yaml,
    Json,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            config,
            dump_ir,
            go_module,
        } => {
            let (config, raw_clang_args) = Config::load_with_raw_clang_args(config)?;
            let ctx = PipelineContext::new(config)
                .with_raw_clang_args(raw_clang_args)
                .with_go_module(go_module);
            generator::generate_all(&ctx, dump_ir)?;
        }
        Command::Ir {
            config,
            output,
            format,
        } => {
            let ctx = PipelineContext::from_config_path(config)?;
            let (ctx, parsed) = generator::prepare_with_parsed(&ctx)?;
            let ir = ir::normalize(&ctx, &parsed)?;
            match (output, format) {
                (Some(path), IrFormat::Yaml) => generator::write_ir(&path, &ir)?,
                (Some(path), IrFormat::Json) => {
                    std::fs::write(path, serde_json::to_string_pretty(&ir)?)?
                }
                (None, IrFormat::Yaml) => print!("{}", serde_yaml::to_string(&ir)?),
                (None, IrFormat::Json) => print!("{}", serde_json::to_string_pretty(&ir)?),
            }
        }
        Command::Check { config } => {
            let ctx = PipelineContext::from_config_path(config)?;
            let (ctx, parsed) = generator::prepare_with_parsed(&ctx)?;
            let ir = ir::normalize(&ctx, &parsed)?;
            println!(
                "ok: {} headers, {} records, {} functions, {} enums, {} abi functions",
                parsed.headers.len(),
                parsed.records.len(),
                parsed.functions.len(),
                parsed.enums.len(),
                ir.functions.len()
            );
        }
    }
    Ok(())
}
