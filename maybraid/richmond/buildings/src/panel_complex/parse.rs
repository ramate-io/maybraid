//! Compact `FromStr` authoring for [`PanelComplex`].
//!
//! ```text
//! 1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}
//! ```
//!
//! Optional thickness as a 4th tuple component: `1=(0,0,0,0.25)`.

use std::str::FromStr;

use bevy_math::Vec3;

use super::types::{PanelComplex, PanelPoint, PanelPointId};

/// Parse failure for the compact panel-complex syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePanelComplexError(pub String);

impl std::fmt::Display for ParsePanelComplexError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::error::Error for ParsePanelComplexError {}

impl FromStr for PanelComplex {
	type Err = ParsePanelComplexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		parse_panel_complex(s)
	}
}

fn err(msg: impl Into<String>) -> ParsePanelComplexError {
	ParsePanelComplexError(msg.into())
}

fn parse_panel_complex(s: &str) -> Result<PanelComplex, ParsePanelComplexError> {
	let s = s.trim();
	if s.is_empty() {
		return Err(err("empty panel-complex string"));
	}
	let (points_src, tris_src) = match s.split_once("...") {
		Some((a, b)) => (a.trim(), b.trim()),
		None => {
			return Err(err(
				"expected `points ... triangles` with a `...` separator",
			));
		}
	};
	if points_src.is_empty() {
		return Err(err("missing point list before `...`"));
	}
	if tris_src.is_empty() {
		return Err(err("missing triangle list after `...`"));
	}

	let mut complex = PanelComplex::rough_stone();
	for (id, point) in parse_points(points_src)? {
		complex.put_point(id, point);
	}
	for (a, b, c) in parse_triangles(tris_src)? {
		complex.add_triangle(a, b, c);
	}
	Ok(complex)
}

fn parse_points(src: &str) -> Result<Vec<(PanelPointId, PanelPoint)>, ParsePanelComplexError> {
	let mut out = Vec::new();
	let mut rest = src.trim();
	while !rest.is_empty() {
		rest = rest.trim_start();
		let eq = rest
			.find('=')
			.ok_or_else(|| err(format!("expected `id=(x,y,z)` near `{rest}`")))?;
		let id_src = rest[..eq].trim();
		let id: u32 = id_src
			.parse()
			.map_err(|_| err(format!("invalid point id `{id_src}`")))?;
		rest = rest[eq + 1..].trim_start();
		if !rest.starts_with('(') {
			return Err(err(format!("expected `(` after id {id}")));
		}
		let close = rest
			.find(')')
			.ok_or_else(|| err(format!("unclosed `(` for point {id}")))?;
		let inner = &rest[1..close];
		let point = parse_point_tuple(inner, id)?;
		out.push((PanelPointId(id), point));
		rest = rest[close + 1..].trim_start();
		if rest.starts_with(',') {
			rest = rest[1..].trim_start();
		} else if !rest.is_empty() {
			return Err(err(format!(
				"expected `,` between points near `{rest}`"
			)));
		}
	}
	Ok(out)
}

fn parse_point_tuple(inner: &str, id: u32) -> Result<PanelPoint, ParsePanelComplexError> {
	let parts: Vec<_> = inner.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	match parts.as_slice() {
		[x, y, z] => {
			let position = Vec3::new(parse_f32(x, id)?, parse_f32(y, id)?, parse_f32(z, id)?);
			Ok(PanelPoint::at(position))
		}
		[x, y, z, t] => {
			let position = Vec3::new(parse_f32(x, id)?, parse_f32(y, id)?, parse_f32(z, id)?);
			Ok(PanelPoint::new(position, parse_f32(t, id)?))
		}
		_ => Err(err(format!(
			"point {id}: expected (x,y,z) or (x,y,z,thickness), got ({inner})"
		))),
	}
}

fn parse_f32(s: &str, id: u32) -> Result<f32, ParsePanelComplexError> {
	s.parse::<f32>()
		.map_err(|_| err(format!("point {id}: invalid number `{s}`")))
}

fn parse_triangles(
	src: &str,
) -> Result<Vec<(PanelPointId, PanelPointId, PanelPointId)>, ParsePanelComplexError> {
	let mut out = Vec::new();
	let mut rest = src.trim();
	while !rest.is_empty() {
		rest = rest.trim_start();
		if !rest.starts_with('{') {
			return Err(err(format!("expected `{{a,b,c}}` near `{rest}`")));
		}
		let close = rest
			.find('}')
			.ok_or_else(|| err(format!("unclosed `{{` near `{rest}`")))?;
		let inner = &rest[1..close];
		let parts: Vec<_> = inner.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
		if parts.len() != 3 {
			return Err(err(format!(
				"triangle expects three ids, got {{{inner}}}"
			)));
		}
		let a = parse_tri_id(parts[0])?;
		let b = parse_tri_id(parts[1])?;
		let c = parse_tri_id(parts[2])?;
		out.push((a, b, c));
		rest = rest[close + 1..].trim_start();
		if rest.starts_with(',') {
			rest = rest[1..].trim_start();
		} else if !rest.is_empty() {
			return Err(err(format!(
				"expected `,` between triangles near `{rest}`"
			)));
		}
	}
	Ok(out)
}

fn parse_tri_id(s: &str) -> Result<PanelPointId, ParsePanelComplexError> {
	let id: u32 = s
		.parse()
		.map_err(|_| err(format!("invalid triangle point id `{s}`")))?;
	Ok(PanelPointId(id))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_mild_trapezoid() {
		let c: PanelComplex =
			"1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}"
				.parse()
				.expect("parse");
		assert_eq!(c.points().count(), 4);
		assert_eq!(c.triangles().len(), 2);
		assert_eq!(c.shared_edges().len(), 1);
		assert!(c.point(PanelPointId(1)).is_some());
		assert!(c.point(PanelPointId(0)).is_none());
	}

	#[test]
	fn parses_thickness_fourth_component() {
		let c: PanelComplex = "1=(0,0,0,0.2),2=(1,0,0,0.6),3=(0,0,1) ... {1,2,3}"
			.parse()
			.expect("parse");
		assert!((c.point(PanelPointId(1)).unwrap().thickness - 0.2).abs() < 1e-5);
		assert!((c.point(PanelPointId(2)).unwrap().thickness - 0.6).abs() < 1e-5);
	}
}
