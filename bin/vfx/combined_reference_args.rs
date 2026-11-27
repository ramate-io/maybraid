//! Explicit filesystem boundary for both offline readers. No workstation defaults.
use std::path::PathBuf;
pub struct ReaderArgs {
	pub output: PathBuf,
	pub assets: PathBuf,
}
impl ReaderArgs {
	pub fn parse() -> Result<Self, Box<dyn std::error::Error>> {
		let mut output = None;
		let mut assets = None;
		let mut args = std::env::args().skip(1);
		while let Some(arg) = args.next() {
			match arg.as_str() {
				"--output" => {
					output = Some(PathBuf::from(args.next().ok_or("--output requires PATH")?))
				}
				"--reference-assets" => {
					assets =
						Some(PathBuf::from(args.next().ok_or("--reference-assets requires PATH")?))
				}
				_ => {
					return Err(format!(
						"unknown argument: {arg}; use --output PATH --reference-assets PATH"
					)
					.into());
				}
			}
		}
		// Absolute preserves aliases/spaces while avoiding Bevy's executable-relative asset root.
		let output = std::path::absolute(output.ok_or("--output PATH is required")?)?;
		let assets =
			std::path::absolute(assets.ok_or(
				"--reference-assets PATH is required (supplied by run_reference_export.py)",
			)?)?;
		if !assets.is_dir() {
			return Err(
				format!("reference asset directory does not exist: {}", assets.display()).into()
			);
		}
		std::fs::create_dir_all(&output)?;
		Ok(Self { output, assets })
	}
}
