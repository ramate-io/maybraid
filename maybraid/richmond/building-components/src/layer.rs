//! Provenance layers for [`crate::BuildingComponents`] node maps.
//!
//! A [`Layer`] is **not** a node-type tag (panel vs partition vs furniture). Domain
//! type stays on the trait method (`panel_nodes_for_level`, …). Layer is a
//! provenance record so higher-order buildings can decide what to *do* with the
//! geometry in that bucket (e.g. treat `"closet"` partitions differently from
//! `"envelope"` ones).

use std::collections::HashMap;
use std::fmt;

/// Provenance label for a bucket of authored geometry.
///
/// Orthogonal to node type: the same name can appear under panels, partitions,
/// furniture, etc. Higher-order concepts read the label to interpret or route
/// that geometry. Prefer [`Layers::free`] until a provenance name is meaningful.
///
/// Examples: `"closet"`, `"envelope"`, `"lantern"`, `"fill"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Layer(pub String);

impl Layer {
	pub fn new(name: impl Into<String>) -> Self {
		Self(name.into())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<&str> for Layer {
	fn from(value: &str) -> Self {
		Self::new(value)
	}
}

impl From<String> for Layer {
	fn from(value: String) -> Self {
		Self(value)
	}
}

impl AsRef<str> for Layer {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl fmt::Display for Layer {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Nodes with optional provenance labels.
///
/// - [`Self::free`] — no provenance yet; fine while higher-order policy is soft
/// - [`Self::labeled`] — nodes tagged with a [`Layer`] provenance name
///
/// `T` is always a concrete domain node type from one trait method; layer keys
/// do not encode that type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layers<T> {
	pub free: Vec<T>,
	pub labeled: HashMap<Layer, Vec<T>>,
}

impl<T> Default for Layers<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Layers<T> {
	pub fn new() -> Self {
		Self { free: Vec::new(), labeled: HashMap::new() }
	}

	/// All nodes unlabeled.
	pub fn from_free(nodes: Vec<T>) -> Self {
		Self { free: nodes, labeled: HashMap::new() }
	}

	/// All nodes under a single label.
	pub fn from_labeled(layer: impl Into<Layer>, nodes: Vec<T>) -> Self {
		let mut layers = Self::new();
		layers.extend_labeled(layer, nodes);
		layers
	}

	pub fn is_empty(&self) -> bool {
		self.free.is_empty() && self.labeled.values().all(|v| v.is_empty())
	}

	pub fn len(&self) -> usize {
		self.free.len() + self.labeled.values().map(|v| v.len()).sum::<usize>()
	}

	pub fn push_free(&mut self, node: T) -> &mut Self {
		self.free.push(node);
		self
	}

	pub fn extend_free(&mut self, nodes: impl IntoIterator<Item = T>) -> &mut Self {
		self.free.extend(nodes);
		self
	}

	pub fn with_free(mut self, nodes: impl IntoIterator<Item = T>) -> Self {
		self.extend_free(nodes);
		self
	}

	pub fn push_labeled(&mut self, layer: impl Into<Layer>, node: T) -> &mut Self {
		self.labeled.entry(layer.into()).or_default().push(node);
		self
	}

	pub fn extend_labeled(
		&mut self,
		layer: impl Into<Layer>,
		nodes: impl IntoIterator<Item = T>,
	) -> &mut Self {
		self.labeled.entry(layer.into()).or_default().extend(nodes);
		self
	}

	pub fn with_labeled(
		mut self,
		layer: impl Into<Layer>,
		nodes: impl IntoIterator<Item = T>,
	) -> Self {
		self.extend_labeled(layer, nodes);
		self
	}

	/// Merge another [`Layers`], appending free lists and same-named labels.
	pub fn extend(&mut self, other: Layers<T>) -> &mut Self {
		self.free.extend(other.free);
		for (layer, mut nodes) in other.labeled {
			self.labeled.entry(layer).or_default().append(&mut nodes);
		}
		self
	}

	pub fn extended(mut self, other: Layers<T>) -> Self {
		self.extend(other);
		self
	}

	/// Flatten free then labeled (labels sorted by name) into one list.
	pub fn flatten(self) -> Vec<T> {
		let mut out = self.free;
		let mut entries: Vec<_> = self.labeled.into_iter().collect();
		entries.sort_by(|a, b| a.0.cmp(&b.0));
		for (_, nodes) in entries {
			out.extend(nodes);
		}
		out
	}
}

impl<T> FromIterator<T> for Layers<T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		Self::from_free(iter.into_iter().collect())
	}
}

impl<T> Extend<T> for Layers<T> {
	fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
		self.extend_free(iter);
	}
}
