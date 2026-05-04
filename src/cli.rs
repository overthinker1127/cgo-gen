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
    #[command(about = "Generate C ABI, C++, and Go wrapper files")]
    Generate {
        #[arg(
            long,
            value_name = "PATH",
            help = "Read generator settings from this YAML config file"
        )]
        config: PathBuf,
        #[arg(
            long,
            default_value_t = false,
            help = "Also write a normalized <name>_wrapper.ir.yaml dump next to generated files"
        )]
        dump_ir: bool,
        #[arg(
            long,
            value_name = "MODULE",
            help = "Write go.mod and build_flags.go in output.dir for this Go module path"
        )]
        go_module: Option<String>,
    },
    #[command(about = "Print or write the normalized intermediate representation")]
    Ir {
        #[arg(
            long,
            value_name = "PATH",
            help = "Read generator settings from this YAML config file"
        )]
        config: PathBuf,
        #[arg(
            long,
            value_name = "PATH",
            help = "Write IR to this file instead of stdout"
        )]
        output: Option<PathBuf>,
        #[arg(
            long,
            value_enum,
            default_value_t = IrFormat::Yaml,
            help = "Choose the IR output format"
        )]
        format: IrFormat,
    },
    #[command(about = "Validate config and supported API shape without writing output files")]
    Check {
        #[arg(
            long,
            value_name = "PATH",
            help = "Read generator settings from this YAML config file"
        )]
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
            match (output.as_ref(), format) {
                (Some(path), IrFormat::Yaml) => generator::write_ir(&path, &ir)?,
                (Some(path), IrFormat::Json) => {
                    let base_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                    let dump_ir = generator::ir_with_source_headers_relative_to(&ir, base_dir);
                    std::fs::write(path, serde_json::to_string_pretty(&dump_ir)?)?
                }
                (None, IrFormat::Yaml) => {
                    let dump_ir =
                        generator::ir_with_source_headers_relative_to(&ir, &ctx.output_dir());
                    print!("{}", serde_yaml::to_string(&dump_ir)?)
                }
                (None, IrFormat::Json) => {
                    let dump_ir =
                        generator::ir_with_source_headers_relative_to(&ir, &ctx.output_dir());
                    print!("{}", serde_json::to_string_pretty(&dump_ir)?)
                }
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    fn subcommand_help(name: &str) -> String {
        let mut command = Cli::command();
        command
            .find_subcommand_mut(name)
            .expect("subcommand should exist")
            .render_help()
            .to_string()
    }

    #[test]
    fn subcommands_have_about_text() {
        let help = Cli::command().render_help().to_string();

        assert!(help.contains("generate  Generate C ABI, C++, and Go wrapper files"));
        assert!(
            help.contains("ir        Print or write the normalized intermediate representation")
        );
        assert!(help.contains(
            "check     Validate config and supported API shape without writing output files"
        ));
    }

    #[test]
    fn generate_help_explains_options() {
        let help = subcommand_help("generate");

        assert!(help.contains("Read generator settings from this YAML config file"));
        assert!(help.contains("Also write a normalized <name>_wrapper.ir.yaml dump"));
        assert!(help.contains("Write go.mod and build_flags.go in output.dir"));
    }

    #[test]
    fn ir_and_check_help_explain_options() {
        let ir_help = subcommand_help("ir");
        let check_help = subcommand_help("check");

        assert!(ir_help.contains("Read generator settings from this YAML config file"));
        assert!(ir_help.contains("Write IR to this file instead of stdout"));
        assert!(ir_help.contains("Choose the IR output format"));
        assert!(check_help.contains("Read generator settings from this YAML config file"));
    }
}
