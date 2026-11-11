//! Wavefront OBJ text parse / validation (global `v` / `vt` / `vn` indices).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjGroupStats {
	pub name: String,
	pub vertices: usize,
	pub texcoords: usize,
	pub normals: usize,
	pub faces: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjValidationError {
	ZeroFaces { group: String, vertices: usize },
	IndexOutOfRange { group: String, kind: &'static str, index: i64, count: usize },
	MissingTexcoord { group: String },
}

impl fmt::Display for ObjValidationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ZeroFaces { group, vertices } => {
				write!(f, "group {group} has {vertices} vertices and 0 faces")
			}
			Self::IndexOutOfRange { group, kind, index, count } => {
				write!(f, "group {group} face {kind} index {index} out of range (file has {count})")
			}
			Self::MissingTexcoord { group } => {
				write!(
					f,
					"group {group} has a face corner without vt (Procreate requires a UV map)"
				)
			}
		}
	}
}

#[derive(Debug)]
struct ParsedGroup {
	stats: ObjGroupStats,
	face_corners: Vec<FaceCorner>,
}

#[derive(Debug, Clone, Copy)]
struct FaceCorner {
	v: i64,
	vt: Option<i64>,
	vn: Option<i64>,
}

/// Parse `o`/`g` groups and count `v` / `vt` / `vn` / `f` with global 1-based indices.
pub fn parse_obj_groups(text: &str) -> (Vec<ObjGroupStats>, Vec<ObjValidationError>) {
	let mut groups: Vec<ParsedGroup> = Vec::new();
	let mut current: Option<usize> = None;
	let mut file_v = 0usize;
	let mut file_vt = 0usize;
	let mut file_vn = 0usize;
	let mut errors = Vec::new();

	for line in text.lines() {
		let line = line.trim();
		if let Some(name) = line.strip_prefix("o ").or_else(|| {
			if current.is_none() {
				line.strip_prefix("g ")
			} else {
				None
			}
		}) {
			let name = name.trim().to_string();
			if current.is_none_or(|i| groups[i].stats.name != name) {
				groups.push(ParsedGroup {
					stats: ObjGroupStats { name, vertices: 0, texcoords: 0, normals: 0, faces: 0 },
					face_corners: Vec::new(),
				});
				current = Some(groups.len() - 1);
			}
			continue;
		}
		if line.starts_with("v ") {
			file_v += 1;
			if let Some(i) = current {
				groups[i].stats.vertices += 1;
			}
		} else if line.starts_with("vt ") {
			file_vt += 1;
			if let Some(i) = current {
				groups[i].stats.texcoords += 1;
			}
		} else if line.starts_with("vn ") {
			file_vn += 1;
			if let Some(i) = current {
				groups[i].stats.normals += 1;
			}
		} else if let Some(rest) = line.strip_prefix("f ") {
			if let Some(i) = current {
				groups[i].stats.faces += 1;
				for token in rest.split_whitespace() {
					groups[i].face_corners.push(parse_corner(token));
				}
			}
		}
	}

	for group in &groups {
		if group.stats.vertices > 0 && group.stats.faces == 0 {
			errors.push(ObjValidationError::ZeroFaces {
				group: group.stats.name.clone(),
				vertices: group.stats.vertices,
			});
		}
		for corner in &group.face_corners {
			push_range_error(&mut errors, &group.stats.name, "v", corner.v, file_v);
			match corner.vt {
				Some(vt) => push_range_error(&mut errors, &group.stats.name, "vt", vt, file_vt),
				None => {
					let already = errors.iter().any(
						|e| matches!(e, ObjValidationError::MissingTexcoord { group: g } if g == &group.stats.name),
					);
					if !already {
						errors.push(ObjValidationError::MissingTexcoord {
							group: group.stats.name.clone(),
						});
					}
				}
			}
			if let Some(vn) = corner.vn {
				push_range_error(&mut errors, &group.stats.name, "vn", vn, file_vn);
			}
		}
	}

	(groups.into_iter().map(|g| g.stats).collect(), errors)
}

pub fn validate_obj_text(text: &str) -> Result<Vec<ObjGroupStats>, Vec<ObjValidationError>> {
	let (stats, errors) = parse_obj_groups(text);
	if errors.is_empty() {
		Ok(stats)
	} else {
		Err(errors)
	}
}

fn parse_corner(token: &str) -> FaceCorner {
	let mut parts = token.split('/');
	let v = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
	let vt = parts.next().and_then(|s| if s.is_empty() { None } else { s.parse().ok() });
	let vn = parts.next().and_then(|s| if s.is_empty() { None } else { s.parse().ok() });
	FaceCorner { v, vt, vn }
}

fn push_range_error(
	errors: &mut Vec<ObjValidationError>,
	group: &str,
	kind: &'static str,
	index: i64,
	count: usize,
) {
	let ok = if index < 0 {
		let resolved = count as i64 + 1 + index;
		resolved >= 1
	} else {
		index >= 1 && (index as usize) <= count
	};
	if !ok {
		let already = errors.iter().any(|e| {
			matches!(
				e,
				ObjValidationError::IndexOutOfRange { group: g, kind: k, index: i, .. }
					if g == group && *k == kind && *i == index
			)
		});
		if !already {
			errors.push(ObjValidationError::IndexOutOfRange {
				group: group.to_string(),
				kind,
				index,
				count,
			});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rejects_zero_faces() -> anyhow::Result<()> {
		let obj = "o head\nv 0 0 0\nv 1 0 0\nv 0 1 0\n";
		let Err(err) = validate_obj_text(obj) else {
			return Err(anyhow::anyhow!("expected zero-face error"));
		};
		if !err.iter().any(|e| {
			matches!(
				e,
				ObjValidationError::ZeroFaces { group, vertices: 3 } if group == "head"
			)
		}) {
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}

	#[test]
	fn rejects_vt_index_past_texcoord_count() -> anyhow::Result<()> {
		let obj = "\
o body
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
vn 0 0 1
vn 0 0 1
f 1//1 2//2 3//3
o head
v 0 0 1
v 1 0 1
v 0 1 1
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
vn 0 0 1
vn 0 0 1
f 4/4/4 5/5/5 6/6/6
";
		let Err(err) = validate_obj_text(obj) else {
			return Err(anyhow::anyhow!("expected vt oob"));
		};
		if !err.iter().any(|e| {
			matches!(
				e,
				ObjValidationError::IndexOutOfRange { group, kind: "vt", index: 4, count: 3 }
					if group == "head"
			)
		}) {
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}

	#[test]
	fn rejects_faces_without_vt() -> anyhow::Result<()> {
		let obj = "\
o body
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
vn 0 0 1
vn 0 0 1
f 1//1 2//2 3//3
";
		let Err(err) = validate_obj_text(obj) else {
			return Err(anyhow::anyhow!("expected missing vt"));
		};
		if !err
			.iter()
			.any(|e| matches!(e, ObjValidationError::MissingTexcoord { group } if group == "body"))
		{
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}

	#[test]
	fn accepts_separate_v_vt_vn_streams() -> anyhow::Result<()> {
		let obj = "\
o body
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
vn 0 0 1
vn 0 0 1
s off
f 1/1/1 2/2/2 3/3/3
o head
v 0 0 1
v 1 0 1
v 0 1 1
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
vn 0 0 1
vn 0 0 1
s off
f 4/4/4 5/5/5 6/6/6
";
		let stats = validate_obj_text(obj).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		if stats.len() != 2 || stats[1].name != "head" || stats[1].faces != 1 {
			return Err(anyhow::anyhow!("{stats:?}"));
		}
		Ok(())
	}
}
