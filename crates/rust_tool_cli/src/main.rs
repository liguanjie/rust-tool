use clap::{Parser, Subcommand, ValueEnum};
use rust_tool_core::{convert_vless_to_yaml, ConvertOptions, OutputMode, TemplateMode};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "rust-tool")]
#[command(about = "Local toolbox CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Convert a vless:// link into Mihomo YAML")]
    ConvertVless {
        #[arg(help = "The vless:// URL")]
        input: String,
        #[arg(long, value_enum, default_value_t = CliOutputMode::FullConfig)]
        mode: CliOutputMode,
        #[arg(long, value_enum, default_value_t = CliTemplateMode::Standard)]
        template: CliTemplateMode,
        #[arg(long, help = "Override the proxy name in YAML")]
        proxy_name: Option<String>,
        #[arg(long, help = "Only output the proxy item")]
        proxy_only: bool,
        #[arg(short, long, help = "Write YAML to a file")]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum CliOutputMode {
    FullConfig,
    ProxyOnly,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliTemplateMode {
    Minimal,
    Standard,
    FullRules,
}

impl fmt::Display for CliOutputMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliOutputMode::FullConfig => formatter.write_str("full-config"),
            CliOutputMode::ProxyOnly => formatter.write_str("proxy-only"),
        }
    }
}

impl fmt::Display for CliTemplateMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliTemplateMode::Minimal => formatter.write_str("minimal"),
            CliTemplateMode::Standard => formatter.write_str("standard"),
            CliTemplateMode::FullRules => formatter.write_str("full-rules"),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ConvertVless {
            input,
            mode,
            template,
            proxy_name,
            proxy_only,
            output,
        } => {
            let output_mode = if proxy_only || matches!(mode, CliOutputMode::ProxyOnly) {
                OutputMode::ProxyOnly
            } else {
                OutputMode::FullConfig
            };

            let template_mode = match template {
                CliTemplateMode::Minimal => TemplateMode::Minimal,
                CliTemplateMode::Standard => TemplateMode::Standard,
                CliTemplateMode::FullRules => TemplateMode::FullRules,
            };

            let yaml = convert_vless_to_yaml(
                &input,
                ConvertOptions {
                    output_mode,
                    template_mode,
                    proxy_name,
                },
            )
                .map_err(|error| error.to_string())?;

            if let Some(path) = output {
                fs::write(&path, yaml).map_err(|error| {
                    format!("写入文件 {} 失败: {error}", path.display())
                })?;
            } else {
                print!("{yaml}");
            }
        }
    }

    Ok(())
}
