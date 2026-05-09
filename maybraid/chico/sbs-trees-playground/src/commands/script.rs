use std::path::Path;

use bevy::prelude::*;

use super::PlaygroundCommand;
use clap::Parser;

#[derive(Debug, Clone, Parser, Component)]
#[command(rename_all = "kebab-case")]
pub struct Script {
	#[arg(long)]
	pub path: std::path::PathBuf,
}

pub fn read_script_lines(path: &Path) -> Result<Vec<String>, String> {
	let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	Ok(text
		.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
		.map(str::to_string)
		.collect())
}

pub(crate) fn run_script_file(path: &Path, commands: &mut Commands, console: &mut String) {
	let lines = match read_script_lines(path) {
		Ok(l) => l,
		Err(e) => {
			*console = e;
			return;
		}
	};
	for (idx, line) in lines.iter().enumerate() {
		match PlaygroundCommand::parse_line(line) {
			Ok(cmd) => cmd.react(commands, console),
			Err(e) => {
				*console = format!("{} line {}: {e}", path.display(), idx + 1);
			}
		}
	}
}
