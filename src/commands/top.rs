pub mod app;
pub mod components;
pub mod events;
pub mod json;
pub mod utils;

use app::run_app;
use json::json;
use utils::calculate_diffs;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser};
use std::{path::PathBuf, thread::sleep, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(last = true, num_args=0..=1, help = "Bcachefs mountpoint to monitor (default: current directory)")]
    mountpoint: Option<PathBuf>,

    #[arg(short, long, default_value = "1")]
    time: u64,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Shows json for fs counters")]
    json: bool,
}

pub fn top(mut argv: Vec<String>) -> Result<()> {
    // The CLI parser here expects the device at position 1.
    if argv.len() > 1 {
        argv.remove(0);
    }
    let cli = Cli::parse_from(argv);

    if cli.json && cli.time > 1 {
        let initial_results = json(&cli.mountpoint).context("Initial stats query failed")?;
        sleep(Duration::from_secs(cli.time));
        let second_results = json(&cli.mountpoint).context("Second stats query failed")?;

        let fs_json = calculate_diffs(&initial_results, &second_results);
        let fs_json_str =
            serde_json::to_string_pretty(&fs_json).context("Failed to pretty-print fs stats")?;
        println!("{fs_json_str}");
    } else if cli.json {
        let fs_json = json(&cli.mountpoint).context("Initial stats query failed")?;
        let fs_json_str =
            serde_json::to_string_pretty(&fs_json).context("Failed to pretty-print fs stats")?;
        println!("{fs_json_str}");
    } else {
        run_app(cli.mountpoint, cli.time)?;
    }

    Ok(())
}
