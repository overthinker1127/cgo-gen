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
            let summary = generator::generate_all(&ctx, dump_ir)?;
            println!("{}", format_generation_summary(&summary));
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
                "{}",
                format_check_summary(&CheckSummary {
                    headers: parsed.headers.len(),
                    records: parsed.records.len(),
                    functions: parsed.functions.len(),
                    enums: parsed.enums.len(),
                    abi_functions: ir.functions.len(),
                    skipped_declarations: &ir.support.skipped_declarations,
                })
            );
        }
    }
    Ok(())
}

struct CheckSummary<'a> {
    headers: usize,
    records: usize,
    functions: usize,
    enums: usize,
    abi_functions: usize,
    skipped_declarations: &'a [ir::SkippedDeclaration],
}

fn format_check_summary(summary: &CheckSummary<'_>) -> String {
    let skipped_count = summary.skipped_declarations.len();
    if skipped_count == 0 {
        return format!(
            "ok: {} headers, {} records, {} functions, {} enums, {} abi functions",
            summary.headers,
            summary.records,
            summary.functions,
            summary.enums,
            summary.abi_functions
        );
    }

    let mut output = format!(
        "ok with warnings: {} headers, {} records, {} functions, {} enums, {} abi functions, {} skipped declarations\nskipped declarations:",
        summary.headers,
        summary.records,
        summary.functions,
        summary.enums,
        summary.abi_functions,
        skipped_count
    );
    for skipped in summary.skipped_declarations.iter().take(5) {
        output.push_str(&format!("\n- {}: {}", skipped.cpp_name, skipped.reason));
    }
    if skipped_count > 5 {
        output.push_str(&format!(
            "\n... and {} more skipped declarations",
            skipped_count - 5
        ));
    }

    output
}

fn format_generation_summary(summary: &generator::GenerationSummary) -> String {
    let file_count = summary.generated_file_count();
    let file_label = if file_count == 1 { "file" } else { "files" };
    let output_dirs = summary.output_dirs();

    match output_dirs.as_slice() {
        [dir] => format!("generated {file_count} {file_label} in {}", dir.display()),
        [] => format!("generated {file_count} {file_label}"),
        dirs => format!(
            "generated {file_count} {file_label} in {} output dirs",
            dirs.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::{CheckSummary, Cli, format_check_summary, format_generation_summary};
    use crate::generator::GenerationSummary;
    use crate::ir::SkippedDeclaration;

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

    #[test]
    fn generation_summary_mentions_file_count_and_output_dir() {
        let mut summary = GenerationSummary::default();
        summary.record_for_test("examples/01-c-library/generated/calculator_wrapper.h");
        summary.record_for_test("examples/01-c-library/generated/calculator_wrapper.cpp");
        summary.record_for_test("examples/01-c-library/generated/calculator_wrapper.go");
        summary.record_for_test("examples/01-c-library/generated/calculator_wrapper.ir.yaml");

        assert_eq!(
            format_generation_summary(&summary),
            "generated 4 files in examples/01-c-library/generated"
        );
    }

    #[test]
    fn check_summary_without_skips_matches_existing_output() {
        assert_eq!(
            format_check_summary(&CheckSummary {
                headers: 1,
                records: 2,
                functions: 3,
                enums: 4,
                abi_functions: 5,
                skipped_declarations: &[],
            }),
            "ok: 1 headers, 2 records, 3 functions, 4 enums, 5 abi functions"
        );
    }

    #[test]
    fn check_summary_with_skips_includes_warning_and_details() {
        let skipped = vec![
            SkippedDeclaration {
                cpp_name: "Value::operator==".to_string(),
                reason: "operator declarations are unsupported in v1".to_string(),
            },
            SkippedDeclaration {
                cpp_name: "set_callback".to_string(),
                reason: "parameter `cb` type `void (*)(int)` uses a function pointer".to_string(),
            },
        ];

        let output = format_check_summary(&CheckSummary {
            headers: 1,
            records: 1,
            functions: 1,
            enums: 0,
            abi_functions: 3,
            skipped_declarations: &skipped,
        });

        assert!(output.contains(
            "ok with warnings: 1 headers, 1 records, 1 functions, 0 enums, 3 abi functions, 2 skipped declarations"
        ));
        assert!(output.contains("skipped declarations:"));
        assert!(
            output.contains("- Value::operator==: operator declarations are unsupported in v1")
        );
        assert!(output.contains(
            "- set_callback: parameter `cb` type `void (*)(int)` uses a function pointer"
        ));
    }

    #[test]
    fn check_summary_with_many_skips_limits_details() {
        let skipped = (1..=6)
            .map(|index| SkippedDeclaration {
                cpp_name: format!("declaration_{index}"),
                reason: format!("reason {index}"),
            })
            .collect::<Vec<_>>();

        let output = format_check_summary(&CheckSummary {
            headers: 1,
            records: 1,
            functions: 1,
            enums: 0,
            abi_functions: 1,
            skipped_declarations: &skipped,
        });

        assert!(output.contains("- declaration_1: reason 1"));
        assert!(output.contains("- declaration_5: reason 5"));
        assert!(!output.contains("- declaration_6: reason 6"));
        assert!(output.contains("... and 1 more skipped declarations"));
    }
}
