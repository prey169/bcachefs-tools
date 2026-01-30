use super::json::FsJsonOutput;

use anyhow::{Context, Result};
use std::{env, path::PathBuf};

pub fn handle_mountpoint(mountpoint: &Option<PathBuf>) -> Result<PathBuf> {
    let mountpoint: PathBuf = if let Some(p) = mountpoint {
        p.to_path_buf()
    } else {
        env::current_dir().context("Failed to get current working directory")?
    };
    Ok(mountpoint)
}

pub fn calculate_diffs(prev: &FsJsonOutput, curr: &FsJsonOutput) -> FsJsonOutput {
    let mut result = FsJsonOutput::default();
    result.time = curr.time;
    result.uuid = curr.uuid.clone();

    for (key, value) in &curr.counters {
        let diff = value - prev.counters[key];
        result.counters.insert(key.clone(), diff);
    }
    result
}
