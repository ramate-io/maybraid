//! Shared `#RRGGBB` parsing for menu swatches and preview tints.

/// Linear sRGB channels in `0..1` parsed from `#RRGGBB` or `RRGGBB`.
pub fn parse_rrggbb_srgb(hex: &str) -> Option<[f32; 3]> {
	let hex = hex.strip_prefix('#').unwrap_or(hex);
	if hex.len() != 6 {
		return None;
	}
	let channel = |range: core::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok();
	match (channel(0..2), channel(2..4), channel(4..6)) {
		(Some(red), Some(green), Some(blue)) => {
			Some([red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0])
		}
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_hash_prefixed_hex() {
		let [r, g, b] = parse_rrggbb_srgb("#ff8040").expect("parse");
		assert!((r - 1.0).abs() < 1e-5);
		assert!((g - 128.0 / 255.0).abs() < 1e-5);
		assert!((b - 64.0 / 255.0).abs() < 1e-5);
	}

	#[test]
	fn parses_unprefixed_hex() {
		assert!(parse_rrggbb_srgb("00ff00").is_some());
	}

	#[test]
	fn rejects_malformed() {
		assert!(parse_rrggbb_srgb("zzz").is_none());
		assert!(parse_rrggbb_srgb("#12345").is_none());
		assert!(parse_rrggbb_srgb("").is_none());
	}
}
