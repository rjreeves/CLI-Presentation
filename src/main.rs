mod protocol;
mod render;

use clap::{Parser, ValueEnum};
use render::{
    RenderOptions, Renderer,
    chart::{ChartRenderer, ChartType},
    dashboard::DashboardRenderer,
    table::TableRenderer,
};
use std::error::Error;
use std::io::{IsTerminal, Read, Write};

#[derive(Copy, Clone, ValueEnum)]
enum Mode {
    Table,
    Dashboard,
    Chart,
}

/// A universal renderer for structured CLI output — see docs/presentation-command.md
#[derive(Parser)]
struct Cli {
    /// Renderer mode. Defaults to `table` (schema-based auto-selection is not yet implemented).
    #[arg(value_enum, default_value = "table")]
    mode: Mode,

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

    let renderer: Box<dyn Renderer> = match cli.mode {
        Mode::Table => Box::new(TableRenderer),
        Mode::Dashboard => Box::new(DashboardRenderer),
        Mode::Chart => Box::new(ChartRenderer {
            chart_type: cli.chart_type,
        }),
    };
    let rendered = renderer.render(&envelope.data, &options)?;

    match cli.output {
        Some(path) => std::fs::File::create(path)?.write_all(rendered.as_bytes())?,
        None => print!("{rendered}"),
    }

    Ok(())
}
