//! Procedural style backends for vegetation IR (unit stick / ball meshes + plane-splay fulfill).
//!
//! Named as style variants until GLBs under `maybraid/art/vegetation/` replace them.

use std::sync::OnceLock;

use bevy::mesh::primitives::{MeshBuilder, Meshable, SphereKind, SphereMeshBuilder};
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;

/// Half-extent of stick kit on \(X/Z\) (\(X = Z \in [-\texttt{STICK\_KIT\_HALF}, \texttt{STICK\_KIT\_HALF}]\)).
pub const STICK_KIT_HALF: f32 = 0.2;

/// Registers shared procedural meshes / materials and fulfills [`PendingPlaneSplay`].
pub struct VegetationProceduralPlugin;

impl Plugin for VegetationProceduralPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_procedural_assets).add_systems(
			Update,
			(
				fulfill_plane_splay,
				crate::update_vegetation_structural_host_levels,
				crate::update_stick_host_levels,
				crate::update_foliage_host_levels,
			),
		);
	}
}

#[derive(Resource, Debug, Clone)]
pub struct VegetationProceduralAssets {
	pub stick_cylinder: Handle<Mesh>,
	pub foliage_ball: Handle<Mesh>,
	pub stick_material: Handle<StandardMaterial>,
	pub foliage_material: Handle<StandardMaterial>,
}

static ASSETS: OnceLock<VegetationProceduralAssets> = OnceLock::new();

impl VegetationProceduralAssets {
	pub fn get() -> &'static VegetationProceduralAssets {
		ASSETS
			.get()
			.expect("VegetationProceduralPlugin must run before vegetation scenes")
	}

	pub fn stick_cylinder() -> Handle<Mesh> {
		Self::get().stick_cylinder.clone()
	}

	pub fn foliage_ball() -> Handle<Mesh> {
		Self::get().foliage_ball.clone()
	}

	pub fn stick_material() -> Handle<StandardMaterial> {
		Self::get().stick_material.clone()
	}

	pub fn foliage_material() -> Handle<StandardMaterial> {
		Self::get().foliage_material.clone()
	}
}

fn init_procedural_assets(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let stick_cylinder = meshes.add(stick_kit_cylinder_mesh());
	let foliage_ball =
		meshes.add(SphereMeshBuilder::new(1.0, SphereKind::Ico { subdivisions: 1 }).build());
	let stick_material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.45, 0.28, 0.14),
		perceptual_roughness: 0.9,
		..default()
	});
	let foliage_material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.22, 0.55, 0.18),
		perceptual_roughness: 0.85,
		cull_mode: None,
		..default()
	});
	let _ = ASSETS.set(VegetationProceduralAssets {
		stick_cylinder,
		foliage_ball,
		stick_material,
		foliage_material,
	});
}

/// Solid cylinder for stick kit space: \(Y \in [0, 1]\), radius [`STICK_KIT_HALF`].
fn stick_kit_cylinder_mesh() -> Mesh {
	let mut mesh = Cylinder::new(STICK_KIT_HALF, 1.0).mesh().build();
	// Bevy's cylinder is centered on Y; shift so base sits at Y = 0.
	if let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
	{
		for p in positions.iter_mut() {
			p[1] += 0.5;
		}
	}
	mesh
}

/// Marker: build plane-splay core + plates under this entity after spawn.
#[derive(Component, Clone, Debug, Default)]
pub struct PendingPlaneSplay {
	pub icosphere_subdivisions: u32,
	pub core_radius: f32,
	pub leaf_disc_radius: f32,
}

#[derive(Component)]
struct PlaneSplayFulfilled;

fn fulfill_plane_splay(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	pending: Query<(Entity, &PendingPlaneSplay), Without<PlaneSplayFulfilled>>,
) {
	let material = VegetationProceduralAssets::foliage_material();
	for (entity, splay) in &pending {
		let subdiv = splay.icosphere_subdivisions.min(4);
		let core_mesh =
			SphereMeshBuilder::new(splay.core_radius, SphereKind::Ico { subdivisions: subdiv })
				.build();
		let plate_mesh = plate_shell_mesh(&core_mesh, splay.leaf_disc_radius);
		let core_handle = meshes.add(core_mesh);
		commands.entity(entity).insert(PlaneSplayFulfilled).with_children(|parent| {
			parent.spawn((
				Mesh3d(core_handle),
				MeshMaterial3d(material.clone()),
				Transform::IDENTITY,
				Visibility::default(),
			));
			if let Some(plate) = plate_mesh {
				parent.spawn((
					Mesh3d(meshes.add(plate)),
					MeshMaterial3d(material.clone()),
					Transform::IDENTITY,
					Visibility::default(),
				));
			}
		});
	}
}

fn tangent_basis(u: Vec3) -> (Vec3, Vec3) {
	let up = if u.y.abs() < 0.92 { Vec3::Y } else { Vec3::Z };
	let mut e1 = up.cross(u);
	if e1.length_squared() < 1e-10 {
		e1 = Vec3::X.cross(u);
	}
	e1 = e1.normalize();
	let e2 = u.cross(e1).normalize();
	(e1, e2)
}

fn plate_shell_mesh(core: &Mesh, leaf_disc_radius: f32) -> Option<Mesh> {
	let positions = core.attribute(Mesh::ATTRIBUTE_POSITION)?;
	let VertexAttributeValues::Float32x3(pos) = positions else {
		return None;
	};
	let indices = core.indices()?;
	let Indices::U32(idx) = indices else {
		return None;
	};

	let mut plates: Option<Mesh> = None;
	for (fi, tri) in idx.chunks_exact(3).enumerate() {
		let a = Vec3::from_array(pos[tri[0] as usize]);
		let b = Vec3::from_array(pos[tri[1] as usize]);
		let c = Vec3::from_array(pos[tri[2] as usize]);
		let centroid = (a + b + c) * (1.0 / 3.0);
		if centroid.length_squared() < 1e-12 {
			continue;
		}
		let radial = centroid.normalize();
		let (e1, e2) = tangent_basis(radial);
		let phi = (fi as f32) * 0.754_877_666_246_693_7 * std::f32::consts::TAU;
		let (cos_p, sin_p) = (phi.cos(), phi.sin());
		let e1r = cos_p * e1 + sin_p * e2;
		let e2r = -sin_p * e1 + cos_p * e2;
		let r = leaf_disc_radius;
		let v0 = r * e1r;
		let ang = std::f32::consts::TAU / 3.0;
		let v1 = r * (ang.cos() * e1r + ang.sin() * e2r);
		let v2 = r * ((2.0 * ang).cos() * e1r + (2.0 * ang).sin() * e2r);
		let piece = Triangle3d::new(v0, v1, v2).mesh().build();
		match &mut plates {
			None => plates = Some(piece),
			Some(acc) => {
				let _ = acc.merge(&piece);
			}
		}
	}
	plates
}
