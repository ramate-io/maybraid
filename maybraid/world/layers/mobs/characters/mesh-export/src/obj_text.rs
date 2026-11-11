//! Wavefront OBJ text parse / validation (global `v` / `vt` / `vn` indices).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjGroupStats {
	pub name: String,
	pub vertices: usize,
	pub texcoords: usize,
	pub normals: usize,
	pub faces: usize,
	pub material: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjValidationError {
	ZeroFaces { group: String, vertices: usize },
	IndexOutOfRange { group: String, kind: &'static str, index: i64, count: usize },
	MissingTexcoord { group: String },
	StreamMismatch { group: String, kind: &'static str },
	IndexNotInObject { group: String, kind: &'static str, index: i64 },
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
			Self::StreamMismatch { group, kind } => {
				write!(f, "group {group} face {kind} index does not match the position stream")
			}
			Self::IndexNotInObject { group, kind, index } => {
				write!(f, "group {group} face {kind} index {index} is not in this object's range")
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct ObjFile {
	pub mtllib: Option<String>,
	pub objects: Vec<ObjObject>,
}

#[derive(Debug, Clone)]
pub struct ObjObject {
	pub name: String,
	pub material: Option<String>,
	pub vertices: usize,
	pub texcoords: usize,
	pub normals: usize,
	pub faces: usize,
	pub v_first: i64,
	pub vt_first: i64,
	pub vn_first: i64,
	pub corners: Vec<FaceCorner>,
}

#[derive(Debug, Clone, Copy)]
pub struct FaceCorner {
	pub v: i64,
	pub vt: Option<i64>,
	pub vn: Option<i64>,
}

impl ObjFile {
	pub fn parse(text: &str) -> Self {
		let mut objects: Vec<ObjObject> = Vec::new();
		let mut current: Option<usize> = None;
		let mut file_v = 0i64;
		let mut file_vt = 0i64;
		let mut file_vn = 0i64;
		let mut mtllib = None;

		for line in text.lines() {
			let line = line.trim();
			if let Some(name) = line.strip_prefix("mtllib ") {
				mtllib = Some(name.trim().to_string());
				continue;
			}
			if let Some(name) = line.strip_prefix("usemtl ") {
				if let Some(i) = current {
					objects[i].material = Some(name.trim().to_string());
				}
				continue;
			}
			if let Some(name) = line.strip_prefix("o ").or_else(|| {
				if current.is_none() {
					line.strip_prefix("g ")
				} else {
					None
				}
			}) {
				let name = name.trim().to_string();
				if current.is_none_or(|i| objects[i].name != name) {
					objects.push(ObjObject {
						name,
						material: None,
						vertices: 0,
						texcoords: 0,
						normals: 0,
						faces: 0,
						v_first: 0,
						vt_first: 0,
						vn_first: 0,
						corners: Vec::new(),
					});
					current = Some(objects.len() - 1);
				}
				continue;
			}
			if line.starts_with("v ") {
				file_v += 1;
				if let Some(i) = current {
					if objects[i].vertices == 0 {
						objects[i].v_first = file_v;
					}
					objects[i].vertices += 1;
				}
			} else if line.starts_with("vt ") {
				file_vt += 1;
				if let Some(i) = current {
					if objects[i].texcoords == 0 {
						objects[i].vt_first = file_vt;
					}
					objects[i].texcoords += 1;
				}
			} else if line.starts_with("vn ") {
				file_vn += 1;
				if let Some(i) = current {
					if objects[i].normals == 0 {
						objects[i].vn_first = file_vn;
					}
					objects[i].normals += 1;
				}
			} else if let Some(rest) = line.strip_prefix("f ") {
				if let Some(i) = current {
					objects[i].faces += 1;
					for token in rest.split_whitespace() {
						objects[i].corners.push(parse_corner(token));
					}
				}
			}
		}

		Self { mtllib, objects }
	}

	pub fn stats(&self) -> Vec<ObjGroupStats> {
		self.objects
			.iter()
			.map(|o| ObjGroupStats {
				name: o.name.clone(),
				vertices: o.vertices,
				texcoords: o.texcoords,
				normals: o.normals,
				faces: o.faces,
				material: o.material.clone(),
			})
			.collect()
	}

	pub fn file_counts(&self) -> (usize, usize, usize) {
		let v = self.objects.iter().map(|o| o.vertices).sum();
		let vt = self.objects.iter().map(|o| o.texcoords).sum();
		let vn = self.objects.iter().map(|o| o.normals).sum();
		(v, vt, vn)
	}

	pub fn validation_errors(&self) -> Vec<ObjValidationError> {
		let (file_v, file_vt, file_vn) = self.file_counts();
		let mut errors = Vec::new();
		for object in &self.objects {
			if object.vertices > 0 && object.faces == 0 {
				errors.push(ObjValidationError::ZeroFaces {
					group: object.name.clone(),
					vertices: object.vertices,
				});
			}
			for corner in &object.corners {
				push_range_error(&mut errors, &object.name, "v", corner.v, file_v);
				object.push_object_range(
					&mut errors,
					"v",
					corner.v,
					object.v_first,
					object.vertices,
				);
				match corner.vt {
					Some(vt) => {
						push_range_error(&mut errors, &object.name, "vt", vt, file_vt);
						object.push_object_range(
							&mut errors,
							"vt",
							vt,
							object.vt_first,
							object.texcoords,
						);
						if object.vertices == object.texcoords
							&& object.v_first > 0 && object.vt_first > 0
							&& (vt - object.vt_first) != (corner.v - object.v_first)
						{
							let already = errors.iter().any(|e| {
								matches!(
									e,
									ObjValidationError::StreamMismatch { group, kind: "vt" }
										if group == &object.name
								)
							});
							if !already {
								errors.push(ObjValidationError::StreamMismatch {
									group: object.name.clone(),
									kind: "vt",
								});
							}
						}
					}
					None => {
						let already = errors.iter().any(
							|e| matches!(e, ObjValidationError::MissingTexcoord { group: g } if g == &object.name),
						);
						if !already {
							errors.push(ObjValidationError::MissingTexcoord {
								group: object.name.clone(),
							});
						}
					}
				}
				if let Some(vn) = corner.vn {
					push_range_error(&mut errors, &object.name, "vn", vn, file_vn);
					object.push_object_range(
						&mut errors,
						"vn",
						vn,
						object.vn_first,
						object.normals,
					);
					if object.vertices == object.normals
						&& object.v_first > 0
						&& object.vn_first > 0
						&& (vn - object.vn_first) != (corner.v - object.v_first)
					{
						let already = errors.iter().any(|e| {
							matches!(
								e,
								ObjValidationError::StreamMismatch { group, kind: "vn" }
									if group == &object.name
							)
						});
						if !already {
							errors.push(ObjValidationError::StreamMismatch {
								group: object.name.clone(),
								kind: "vn",
							});
						}
					}
				}
			}
		}
		errors
	}
}

impl ObjObject {
	fn push_object_range(
		&self,
		errors: &mut Vec<ObjValidationError>,
		kind: &'static str,
		index: i64,
		first: i64,
		count: usize,
	) {
		if first <= 0 || count == 0 || index < 1 {
			return;
		}
		let last = first + count as i64 - 1;
		if index < first || index > last {
			let already = errors.iter().any(|e| {
				matches!(
					e,
					ObjValidationError::IndexNotInObject { group, kind: k, index: i }
						if group == &self.name && *k == kind && *i == index
				)
			});
			if !already {
				errors.push(ObjValidationError::IndexNotInObject {
					group: self.name.clone(),
					kind,
					index,
				});
			}
		}
	}
}

/// Parse `o`/`g` groups and count `v` / `vt` / `vn` / `f` with global 1-based indices.
pub fn parse_obj_groups(text: &str) -> (Vec<ObjGroupStats>, Vec<ObjValidationError>) {
	let file = ObjFile::parse(text);
	let errors = file.validation_errors();
	(file.stats(), errors)
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
mtllib x.mtl
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
usemtl body_mat
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
usemtl head_mat
f 4/4/4 5/5/5 6/6/6
";
		let stats = validate_obj_text(obj).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		if stats.len() != 2 || stats[1].name != "head" || stats[1].faces != 1 {
			return Err(anyhow::anyhow!("{stats:?}"));
		}
		if stats[0].material.as_deref() != Some("body_mat")
			|| stats[1].material.as_deref() != Some("head_mat")
		{
			return Err(anyhow::anyhow!("materials {stats:?}"));
		}
		Ok(())
	}

	#[test]
	fn rejects_head_faces_that_reuse_body_vertex_indices() -> anyhow::Result<()> {
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
f 1/1/1 2/2/2 3/3/3
";
		let Err(err) = validate_obj_text(obj) else {
			return Err(anyhow::anyhow!("expected object-local index error"));
		};
		if !err.iter().any(|e| {
			matches!(
				e,
				ObjValidationError::IndexNotInObject { group, kind: "v", index: 1 }
					if group == "head"
			)
		}) {
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}

	#[test]
	fn rejects_vt_stream_that_does_not_track_positions() -> anyhow::Result<()> {
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
f 1/2/1 2/1/2 3/3/3
";
		let Err(err) = validate_obj_text(obj) else {
			return Err(anyhow::anyhow!("expected stream mismatch"));
		};
		if !err.iter().any(
			|e| matches!(e, ObjValidationError::StreamMismatch { group, kind: "vt" } if group == "body"),
		) {
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}
}
