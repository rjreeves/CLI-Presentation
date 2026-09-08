mod protocol;
mod registry;
mod render;

use clap::{Parser, ValueEnum};
use render::{
    RenderOptions, Renderer,
    chart::{ChartRenderer, ChartType},
    dashboard::DashboardRenderer,
    html::HtmlRenderer,
    map::MapRenderer,
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
}

/// A universal renderer for structured CLI output — see docs/presentation-command.md
#[derive(Parser)]
struct Cli {
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
}

/// Resolves the renderer to use: an explicit `--mode`/positional argument
/// wins outright; otherwise the payload's `schema` is looked up in the
/// built-in registry (docs/presentation-protocol.md §5.2). A schema that is
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
    match registry::default_renderer(schema) {
        Some("table") => Mode::Table,
        Some("dashboard") => Mode::Dashboard,
        Some("chart") => Mode::Chart,
        Some("timeline") => Mode::Timeline,
        Some("map") => Mode::Map,
        Some("html") => Mode::Html,
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
    };
    let rendered = renderer.render(&envelope.data, &options)?;

    match cli.output {
        Some(path) => std::fs::File::create(path)?.write_all(rendered.as_bytes())?,
        None => print!("{rendered}"),
    }

    Ok(())
}
