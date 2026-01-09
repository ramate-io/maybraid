use super::{CellComplex3d, Edge, Face, Vertex};
use std::collections::HashSet;

pub struct TrackEnclosedFaces<V: Vertex, E: Edge<V>, F: Face<V, E>> {
	// To prevent misuse that would upset the tracking, we keep this private.
	cell_complex: CellComplex3d<V, E, F>,
	// The edges which are incident to at least two faces.
	//
	// Because you can only add to the cell complex, this is safe to fill once the condition is met.
	two_incident_edges: HashSet<E>,
	// All of the faces which are completely made of two incident edges.
	//
	// Because you can only add to the cell complex, this is safe to fill once the condition is met.
	all_two_incident_faces: HashSet<F>,
}

impl<V: Vertex, E: Edge<V>, F: Face<V, E>> TrackEnclosedFaces<V, E, F> {
	/// Creates a new track enclosed faces structure.
	///
	/// Note that because the user can provide a cell complex, you may miss faces.
	/// Later, we implement [SafeTrackEnclosedFaces] which will ensure that all faces are tracked.
	/// However, a user may want to build a cell complex only tracking enclosed faces from
	/// a later point, so this API is public.
	pub fn new(cell_complex: CellComplex3d<V, E, F>) -> Self {
		Self {
			cell_complex,
			two_incident_edges: HashSet::new(),
			all_two_incident_faces: HashSet::new(),
		}
	}

	/// Adds a face to the track enclosed faces builder, tracking enclosed faces as a side effect.
	pub fn add_face(&mut self, face: F) {
		// Add the face to the cell complex.
		self.cell_complex.add_face(face.clone());

		// Compute the incident faces.
		let incident_faces = self.cell_complex.face_to_incident_faces(&face);

		// check if there are at least two incident edges for each incident face
		for incident_face in &incident_faces {
			for edge in incident_face.edges() {
				if self.cell_complex.edge_to_incident_faces(&edge).len() > 2 {
					self.two_incident_edges.insert(edge);
				}
			}
		}

		// For incident face, check if all of the edges are two incident edges.
		for incident_face in incident_faces {
			for edge in incident_face.edges() {
				if !self.two_incident_edges.contains(&edge) {
					break;
				}
			}
			self.all_two_incident_faces.insert(incident_face.clone());
		}
	}
}
