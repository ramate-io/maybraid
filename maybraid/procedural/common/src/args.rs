//! Small parser-backed argument value types shared by procedural front-ends.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitRange {
	pub start: f32,
	pub end: f32,
}

impl UnitRange {
	pub const fn new(start: f32, end: f32) -> Self {
		Self { start, end }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountPair {
	pub first: u32,
	pub second: u32,
}

impl CountPair {
	pub const fn new(first: u32, second: u32) -> Self {
		Self { first, second }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsizeRange {
	pub start: usize,
	pub end: usize,
}

impl UsizeRange {
	pub const fn new(start: usize, end: usize) -> Self {
		Self { start, end }
	}
}

pub fn parse_unit_range(s: &str) -> Result<UnitRange, String> {
	let (start, end) = s
		.split_once("..")
		.ok_or_else(|| format!("expected start..end range, got {s:?}"))?;
	let start = start.trim().parse::<f32>().map_err(|e| e.to_string())?;
	let end = end.trim().parse::<f32>().map_err(|e| e.to_string())?;
	Ok(UnitRange::new(start, end))
}

pub fn parse_count_pair(s: &str) -> Result<CountPair, String> {
	let (first, second) = s
		.split_once('x')
		.ok_or_else(|| format!("expected firstxsecond counts, got {s:?}"))?;
	let first = first.trim().parse::<u32>().map_err(|e| e.to_string())?;
	let second = second.trim().parse::<u32>().map_err(|e| e.to_string())?;
	Ok(CountPair::new(first, second))
}

pub fn parse_usize_range(s: &str) -> Result<UsizeRange, String> {
	let (start, end) = s
		.split_once("..")
		.ok_or_else(|| format!("expected start..end range, got {s:?}"))?;
	let start = start.trim().parse::<usize>().map_err(|e| e.to_string())?;
	let end = end.trim().parse::<usize>().map_err(|e| e.to_string())?;
	Ok(UsizeRange::new(start, end))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_common_arg_shapes() -> anyhow::Result<()> {
		assert_eq!(
			parse_unit_range("0.40..0.95").map_err(|e| anyhow::anyhow!("{e}"))?,
			UnitRange::new(0.40, 0.95)
		);
		assert_eq!(
			parse_count_pair("8x7").map_err(|e| anyhow::anyhow!("{e}"))?,
			CountPair::new(8, 7)
		);
		assert_eq!(
			parse_usize_range("4..6").map_err(|e| anyhow::anyhow!("{e}"))?,
			UsizeRange::new(4, 8)
		);
		Ok(())
	}
}
