use super::{CellComplex3d, Edge, Face, Vertex};
use std::collections::HashSet;

pub struct TrackEnclosedFaces<O, V: Vertex, E: Edge<O, V>, F: Face<O, V, E>> {
	// To prevent misuse that would upset the tracking, we keep this private.
	cell_complex: CellComplex3d<O, V, E, F>,
	// The edges which are incident to at least two faces.
	//
	// Because you can only add to the cell complex, this is safe to fill once the condition is met.
	two_incident_edges: HashSet<E>,
	// All of the faces which are completely made of two incident edges, i.e., enclosed.
	//
	// Because you can only add to the cell complex, this is safe to fill once the condition is met.
	enclosed_faces: HashSet<F>,
}

impl<O, V: Vertex, E: Edge<O, V>, F: Face<O, V, E>> TrackEnclosedFaces<O, V, E, F> {
	/// Creates a new track enclosed faces builder.
	///
	/// Note that because the user can provide a cell complex, you may miss faces.
	/// Later, we implement [SafeTrackEnclosedFaces] which will ensure that all faces are tracked.
	/// However, a user may want to build a cell complex only tracking enclosed faces from
	/// a later point, so this API is public.
	pub fn new(cell_complex: CellComplex3d<O, V, E, F>) -> Self {
		Self { cell_complex, two_incident_edges: HashSet::new(), enclosed_faces: HashSet::new() }
	}

	pub fn add_face(&mut self, face: F, on: &O) {
		// add the face to the cell complex
		self.cell_complex.add_face(face.clone(), on);

		// compute the incident faces
		let incident_faces = self.cell_complex.face_to_incident_faces(&face, on);

		// check if there are at least two incident edges for each incident face
		for incident_face in &incident_faces {
			for edge in incident_face.edges(on) {
				if self.cell_complex.edge_to_incident_faces(&edge).len() > 2 {
					self.two_incident_edges.insert(edge);
				}
			}
		}

		// for incident face, check if all of the edges are two incident edges
		for incident_face in incident_faces {
			for edge in incident_face.edges(on) {
				if !self.two_incident_edges.contains(&edge) {
					return;
				}
			}
			self.enclosed_faces.insert(incident_face.clone());
		}
	}
}
