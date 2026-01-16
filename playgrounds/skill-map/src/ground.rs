use crate::checkerboard_material::CheckerboardMaterial;
use bevy::prelude::*;

#[derive(Resource)]
pub struct CheckerSize {
	/// Checker size in meters (powers of 10: 1, 10, 100, 1000, etc.)
	pub size_meters: f32,
	/// Exponent for power of 10 (0 = 1m, 1 = 10m, 2 = 100m, etc.)
	pub exponent: i32,
}

impl Default for CheckerSize {
	fn default() -> Self {
		Self {
			size_meters: 10.0, // Default: 1 km
			exponent: 1,       // 10^1 = 10 meters
		}
	}
}

impl CheckerSize {
	pub fn increase(&mut self) {
		self.exponent += 1;
		self.size_meters = 10.0_f32.powi(self.exponent);
	}

	pub fn decrease(&mut self) {
		if self.exponent > -3 {
			// Don't go below 0.001m (1mm)
			self.exponent -= 1;
			self.size_meters = 10.0_f32.powi(self.exponent);
		}
	}

	pub fn size_m(&self) -> f32 {
		self.size_meters
	}
}

#[derive(Component)]
pub struct CheckeredGround;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
	checker_size: Res<CheckerSize>,
) {
	// Create a large ground plane (1km x 1km)
	let size = 1000.0; // 1km x 1km ground plane
	let mesh = meshes.add(Plane3d::default().mesh().size(size, size));

	// Create a checkered material
	let material = materials.add(CheckerboardMaterial {
		checker_size_m: checker_size.size_m(),
		color1: Color::srgb(0.9, 0.9, 0.9).into(), // Light gray
		color2: Color::srgb(0.7, 0.7, 0.7).into(), // Dark gray
	});

	commands.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
		CheckeredGround,
	));
}

pub fn update_checker_size(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut checker_size: ResMut<CheckerSize>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
	ground_query: Query<&MeshMaterial3d<CheckerboardMaterial>, With<CheckeredGround>>,
) {
	let mut changed = false;

	if keyboard_input.just_pressed(KeyCode::Equal) {
		checker_size.increase();
		changed = true;
		log::info!("Checker size increased to {} meters", checker_size.size_meters);
	}

	if keyboard_input.just_pressed(KeyCode::Minus) {
		checker_size.decrease();
		changed = true;
		log::info!("Checker size decreased to {} meters", checker_size.size_meters);
	}

	if changed {
		// Update the material with new checker size
		if let Ok(mesh_material) = ground_query.single() {
			if let Some(material) = materials.get_mut(&mesh_material.0) {
				material.checker_size_m = checker_size.size_m();
			}
		}
	}
}
