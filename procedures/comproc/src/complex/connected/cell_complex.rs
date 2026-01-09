pub mod track_enclosed;

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

pub trait Vertex: Hash + Eq + Clone + Debug {
	fn point(&self) -> Vec3;
}
pub trait Edge<V: Vertex>: Hash + Eq + Clone + Debug {
	fn vertices(&self) -> Vec<V>;
}
pub trait Face<V: Vertex, E: Edge<V>>: Hash + Eq + Clone + Debug {
	fn edges(&self) -> Vec<E>;
}

pub struct CellComplex3d<V: Vertex, E: Edge<V>, F: Face<V, E>> {
	pub v_to_e: HashMap<V, HashSet<E>>,
	pub e_to_f: HashMap<E, HashSet<F>>,
}

impl<V: Vertex, E: Edge<V>, F: Face<V, E>> CellComplex3d<V, E, F> {
	pub fn new() -> Self {
		Self { v_to_e: HashMap::new(), e_to_f: HashMap::new() }
	}

	fn add_edge(&mut self, edge: E) {
		for vertex in edge.vertices() {
			self.v_to_e.entry(vertex).or_insert(HashSet::new()).insert(edge.clone());
		}
	}

	/// Adds a face to the cell complex, adds edges to the cell complex as well.
	pub fn add_face(&mut self, face: F) {
		for edge in face.edges() {
			self.e_to_f.entry(edge.clone()).or_insert(HashSet::new()).insert(face.clone());
			self.add_edge(edge);
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
	pub fn face_to_incident_faces(&self, face: &F) -> HashSet<&F> {
		let mut incident_faces = HashSet::new();
		for edge in face.edges() {
			for face in self.edge_to_incident_faces(&edge) {
				incident_faces.insert(face);
			}
		}
		incident_faces
	}

	/// Computes those faces which are two incident on all edges
	pub fn two_incident_faces(&self) -> HashSet<&F> {
		let mut two_incident_faces = HashSet::new();
		for face in self.unique_faces() {
			let mut all_two_incident = true;
			for edge in face.edges() {
				if self.edge_to_incident_faces(&edge).len() < 2 {
					all_two_incident = false;
					break;
				}
			}
			if all_two_incident {
				two_incident_faces.insert(face);
			}
		}
		two_incident_faces
	}

	/// Computes those faces which are only incident to two-incident faces.
	pub fn hyper_two_incident_faces(&self) -> HashSet<&F> {
		let mut hyper_two_incident_faces = HashSet::new();
		let two_incident_faces = self.two_incident_faces();
		for incident_face in &two_incident_faces {
			for incident_face in self.face_to_incident_faces(incident_face) {
				if two_incident_faces.contains(incident_face) {
					hyper_two_incident_faces.insert(incident_face);
				}
			}
		}
		hyper_two_incident_faces
	}

	pub fn build(&mut self, builder: &mut impl CellComplex3dBuilder<V, E, F>) {
		let mut last_face = None;
		while let Some(face) = builder.next_face(self, last_face) {
			self.add_face(face.clone());
			last_face = Some(face);
		}
	}
}

pub trait CellComplex3dBuilder<V: Vertex, E: Edge<V>, F: Face<V, E>> {
	fn next_face(&mut self, complex: &CellComplex3d<V, E, F>, last_face: Option<F>) -> Option<F>;
}
