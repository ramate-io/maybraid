//! Comma-separated vector parsing for clap (`--foo 1,2,3`).

use bevy_math::{Vec2, Vec3};

/// Two comma-separated floats (optional ASCII whitespace around commas).
pub fn parse_vec2_csv(s: &str) -> Result<Vec2, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	if parts.len() != 2 {
		return Err(format!("expected two comma-separated numbers (e.g. 1.0,2.0), got {s:?}"));
	}
	let x = parts[0]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[0]))?;
	let y = parts[1]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[1]))?;
	Ok(Vec2::new(x, y))
}

/// Three comma-separated floats (optional ASCII whitespace around commas).
pub fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	if parts.len() != 3 {
		return Err(format!(
			"expected three comma-separated numbers (e.g. 1.0,2.0,3.0), got {s:?}"
		));
	}
	let x = parts[0]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[0]))?;
	let y = parts[1]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[1]))?;
	let z = parts[2]
		.parse::<f32>()
		.map_err(|e| format!("invalid float {:?}: {e}", parts[2]))?;
	Ok(Vec3::new(x, y, z))
}
