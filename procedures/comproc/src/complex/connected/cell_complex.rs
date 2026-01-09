pub mod track_enclosed;

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

pub trait Vertex: Hash + Eq + Clone + Debug {
	fn point(&self) -> Vec3;
}
pub trait Edge<O, V: Vertex>: Hash + Eq + Clone + Debug {
	fn vertices(&self, on: &O) -> Vec<V>;
}
pub trait Face<O, V: Vertex, E: Edge<O, V>>: Hash + Eq + Clone + Debug {
	fn edges(&self, on: &O) -> Vec<E>;
}

pub struct CellComplex3d<O, V: Vertex, E: Edge<O, V>, F: Face<O, V, E>> {
	pub v_to_e: HashMap<V, HashSet<E>>,
	pub e_to_f: HashMap<E, HashSet<F>>,
	__phantom_o: PhantomData<O>,
}

impl<O, V: Vertex, E: Edge<O, V>, F: Face<O, V, E>> CellComplex3d<O, V, E, F> {
	pub fn new() -> Self {
		Self { v_to_e: HashMap::new(), e_to_f: HashMap::new(), __phantom_o: PhantomData }
	}

	fn add_edge(&mut self, edge: E, on: &O) {
		for vertex in edge.vertices(on) {
			self.v_to_e.entry(vertex).or_insert(HashSet::new()).insert(edge.clone());
		}
	}

	/// Adds a face to the cell complex, adds edges to the cell complex as well.
	pub fn add_face(&mut self, face: F, on: &O) {
		for edge in face.edges(on) {
			self.e_to_f.entry(edge.clone()).or_insert(HashSet::new()).insert(face.clone());
			self.add_edge(edge, on);
		}
	}

	pub fn unique_edges(&self) -> HashSet<&E> {
		self.e_to_f.keys().collect()
	}

	pub fn unique_faces(&self) -> HashSet<&F> {
		self.e_to_f.values().flat_map(|e| e.iter()).collect()
	}

	pub fn edge_to_incident_faces(&self, edge: &E) -> HashSet<&F> {
		self.e_to_f.get(edge).map(|e| e.iter()).unwrap_or_default().collect()
	}

	/// Gets all the faces that are incident to a given face.
	pub fn face_to_incident_faces(&self, face: &F, on: &O) -> HashSet<&F> {
		let mut incident_faces = HashSet::new();
		for edge in face.edges(on) {
			for face in self.edge_to_incident_faces(&edge) {
				incident_faces.insert(face);
			}
		}
		incident_faces
	}

	pub fn build(&mut self, builder: &mut impl CellComplex3dBuilder<O, V, E, F>, on: &O) {
		let mut last_face = None;
		while let Some(face) = builder.next_face(self, last_face) {
			self.add_face(face.clone(), on);
			last_face = Some(face);
		}
	}
}

pub trait CellComplex3dBuilder<O, V: Vertex, E: Edge<O, V>, F: Face<O, V, E>> {
	fn next_face(&mut self, complex: &CellComplex3d<O, V, E, F>, last_face: Option<F>)
		-> Option<F>;
}
