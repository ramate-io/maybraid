//! Load a script file and run each line as a [`super::PlaygroundCommand`] (same as in-game `/`).

use std::path::Path;

use bevy::prelude::*;

use super::PlaygroundCommand;
use clap::Parser;

/// One line per in-game command; empty lines and `#` comments skipped.
#[derive(Debug, Clone, Parser, Component)]
#[command(rename_all = "kebab-case")]
pub struct Script {
	/// Script file path.
	#[arg(long)]
	pub path: std::path::PathBuf,
}

/// Read a script: trim lines, skip empty lines and `#` comments.
pub fn read_script_lines(path: &Path) -> Result<Vec<String>, String> {
	let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	Ok(text
		.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
		.map(str::to_string)
		.collect())
}

/// Parse each line and call [`PlaygroundCommand::react`]; parse / IO failures overwrite `console`.
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn read_script_lines_skips_comments_and_blank() -> anyhow::Result<()> {
		let dir = std::env::temp_dir();
		let path = dir.join("sdf_playground_script_read_test.txt");
		std::fs::write(&path, "\n# c\n  render tapered-cylinder  \n")?;
		let lines = read_script_lines(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
		std::fs::remove_file(&path).ok();
		assert_eq!(lines, vec!["render tapered-cylinder"]);
		Ok(())
	}
}
