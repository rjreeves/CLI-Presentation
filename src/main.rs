mod protocol;
mod registry;
mod render;

use clap::{Parser, Subcommand, ValueEnum};
use render::{
    RenderOptions, Renderer,
    ai_summary::AiSummaryRenderer,
    chart::{ChartRenderer, ChartType},
    dashboard::DashboardRenderer,
    explain::{ExplainLevel, ExplainRenderer},
    html::HtmlRenderer,
    map::MapRenderer,
    md::MarkdownRenderer,
    table::TableRenderer,
    timeline::TimelineRenderer,
};
use std::error::Error;
use std::io::{IsTerminal, Read, Write};

#[derive(Copy, Clone, ValueEnum)]
enum Mode {
    Table,
    Dashboard,
    Chart,
    Timeline,
    Map,
    Html,
    Md,
    Explain,
    AiSummary,
}

#[derive(Subcommand)]
enum Command {
    /// Register a schema in the user registry (~/.presentation/registry.json)
    /// — see docs/schema-registry.md §4.
    Register {
        /// Schema name, e.g. `myapp.metrics`.
        #[arg(long)]
        schema: String,
        /// Default renderer for this schema.
        #[arg(long)]
        renderer: String,
        /// Path to a field-definitions JSON file (see schema-registry.md §10).
        #[arg(long)]
        fields: Option<String>,
    },
    /// Validate a PPS payload from stdin against the schema registry — see
    /// docs/schema-registry.md §5.
    Validate,
}

/// A universal renderer for structured CLI output — see docs/presentation-command.md
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Renderer mode. If omitted, selected from the payload's `schema` field
    /// (see docs/presentation-protocol.md §5.2), falling back to `table`.
    #[arg(value_enum)]
    mode: Option<Mode>,

    /// Sort rows by this field.
    #[arg(long)]
    sort: Option<String>,

    /// Filter rows: `field=value`.
    #[arg(long)]
    filter: Option<String>,

    /// Terminal width for layout.
    #[arg(long)]
    width: Option<usize>,

    /// Write rendered output to a file instead of stdout.
    #[arg(long)]
    output: Option<String>,

    /// Chart style, for `chart` mode.
    #[arg(long, value_enum, default_value = "bar")]
    chart_type: ChartType,

    /// AI verbosity, for `explain`/`ai-summary` modes.
    #[arg(long, value_enum, default_value = "brief")]
    explain_level: ExplainLevel,
}

/// Resolves the renderer to use: an explicit `--mode`/positional argument
/// wins outright; otherwise the payload's `schema` is looked up in the
/// merged registry (user-registered schemas take precedence over
/// built-ins — see registry::resolve_renderer). A schema that is
/// unrecognized, or that maps to a renderer this build doesn't implement,
/// warns on stderr and falls back to `table`, per docs/schema-registry.md
/// §9.
fn resolve_mode(cli_mode: Option<Mode>, schema: Option<&str>) -> Mode {
    if let Some(mode) = cli_mode {
        return mode;
    }
    let Some(schema) = schema else {
        return Mode::Table;
    };
    match registry::resolve_renderer(schema).as_deref() {
        Some("table") => Mode::Table,
        Some("dashboard") => Mode::Dashboard,
        Some("chart") => Mode::Chart,
        Some("timeline") => Mode::Timeline,
        Some("map") => Mode::Map,
        Some("html") => Mode::Html,
        Some("md") => Mode::Md,
        Some("explain") => Mode::Explain,
        Some("ai-summary") => Mode::AiSummary,
        Some(other) => {
            eprintln!(
                "presentation: renderer '{other}' for schema '{schema}' is not implemented yet; falling back to table"
            );
            Mode::Table
        }
        None => {
            eprintln!("presentation: unknown schema '{schema}'; falling back to table");
            Mode::Table
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        return match command {
            Command::Register {
                schema,
                renderer,
                fields,
            } => registry::register(&schema, &renderer, fields.as_deref()),
            Command::Validate => {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                registry::validate(&input)
            }
        };
    }

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let envelope = protocol::parse(&input)?;

    let filter = cli
        .filter
        .as_ref()
        .and_then(|f| f.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()));

    let options = RenderOptions {
        sort: cli.sort,
        filter,
        width: cli.width,
        color: cli.output.is_none() && std::io::stdout().is_terminal(),
    };

    let mode = resolve_mode(cli.mode, envelope.schema.as_deref());
    let renderer: Box<dyn Renderer> = match mode {
        Mode::Table => Box::new(TableRenderer),
        Mode::Dashboard => Box::new(DashboardRenderer),
        Mode::Chart => Box::new(ChartRenderer {
            chart_type: cli.chart_type,
        }),
        Mode::Timeline => Box::new(TimelineRenderer),
        Mode::Map => Box::new(MapRenderer),
        Mode::Html => Box::new(HtmlRenderer),
        Mode::Md => Box::new(MarkdownRenderer),
        Mode::Explain => Box::new(ExplainRenderer {
            level: cli.explain_level,
        }),
        Mode::AiSummary => Box::new(AiSummaryRenderer {
            level: cli.explain_level,
        }),
    };
    let rendered = renderer.render(&envelope.data, &options)?;

    match cli.output {
        Some(path) => std::fs::File::create(path)?.write_all(rendered.as_bytes())?,
        None => print!("{rendered}"),
    }

    Ok(())
}
