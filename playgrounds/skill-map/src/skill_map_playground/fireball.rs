use crate::camera::CameraController;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Default)]
pub struct Fireball {
	age: f32,
	max_age: f32,
	radius: f32,
	radius_decay: f32,
	velocity: Vec3,
	drag: f32,
}

impl Fireball {
	pub fn new(max_age: f32, radius: f32, radius_decay: f32, velocity: Vec3, drag: f32) -> Self {
		Self { age: 0.0, max_age, radius, radius_decay, velocity, drag }
	}

	pub fn age(&self) -> f32 {
		self.age
	}

	pub fn max_age(&self) -> f32 {
		self.max_age
	}

	pub fn velocity(&self) -> Vec3 {
		self.velocity
	}

	pub fn radius(&self) -> f32 {
		self.radius
	}

	pub fn next(&self, dt: f32, position: Vec3) -> Option<(Self, Vec3)> {
		let age = self.age + dt;
		if age > self.max_age {
			return None;
		}

		// Gravity acceleration (m/s^2)
		// If you want "bend toward -Vec3::Y", use negative Y gravity.
		let gravity_strength = 9.81;
		let gravity = -Vec3::Y * gravity_strength;

		// 1) Accelerate velocity by gravity
		let mut velocity = self.velocity + gravity * dt;

		// 2) Apply drag (linear air resistance, stable & framerate independent)
		// velocity_decay is in 1/sec
		let drag = (-self.drag * dt).exp();
		velocity *= drag;

		// 3) Integrate position using updated velocity (semi-implicit Euler)
		let new_position = position + velocity * dt;

		// Radius decay (optional)
		let radius = (self.radius - self.radius_decay * dt).max(0.0);

		Some((
			Self {
				age,
				max_age: self.max_age,
				radius,
				radius_decay: self.radius_decay,
				velocity,
				drag: self.drag,
			},
			new_position,
		))
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct DispatchCameraFireball(pub Fireball);

pub struct FireballPlugin;

impl FireballPlugin {
	pub fn render_fireball(
		mut commands: Commands,
		mut meshes: ResMut<Assets<Mesh>>,
		mut materials: ResMut<Assets<StandardMaterial>>,
		time: Res<Time>,
		query: Query<(Entity, &Fireball, &Transform)>,
	) {
		for (entity, fireball, transform) in query.iter() {
			log::info!("Rendering fireball");
			if let Some((fireball, position)) =
				fireball.next(time.delta_secs(), transform.translation)
			{
				// translate the fireball
				log::info!("Translating fireball to: {:?}", position);
				commands.entity(entity).insert(Transform::from_translation(position));

				// update the rendering
				commands
					.entity(entity)
					.insert(Mesh3d(meshes.add(Sphere { radius: fireball.radius(), ..default() })));
				commands.entity(entity).insert(MeshMaterial3d(materials.add(StandardMaterial {
					base_color: Color::srgba(1.0, 0.0, 0.0, 0.9),
					alpha_mode: AlphaMode::AlphaToCoverage,
					..default()
				})));

				// replace the fireball with a new one
				commands.entity(entity).insert(fireball);

				// Make sure the render layer is 0
				commands.entity(entity).insert(RenderLayers::layer(0));
			} else {
				// despawn the fireball
				commands.entity(entity).despawn();
			}
		}
	}

	pub fn dispatch_camera_fireball(
		mut commands: Commands,
		dispatch_query: Query<(Entity, &DispatchCameraFireball), Added<DispatchCameraFireball>>,
		camera_query: Query<&Transform, (With<Camera3d>, With<CameraController>)>,
	) {
		for (entity, dispatch) in dispatch_query.iter() {
			if let Ok(camera) = camera_query.single() {
				log::info!("Dispatching camera fireball");
				let mut fireball = dispatch.0.clone();

				// the velocity magnitude of the fireball is the length of the given fireball velocity vector
				let velocity_magnitude = fireball.velocity().length();

				// the vectore is the direction in which the camera is looking
				let direction = camera.forward();
				let velocity = direction * velocity_magnitude;
				fireball.velocity = velocity;

				commands.entity(entity).insert(fireball);
				commands.entity(entity).insert(Transform::from_translation(camera.translation));
			}
		}
	}
}

impl Plugin for FireballPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, Self::render_fireball);
		app.add_systems(Update, Self::dispatch_camera_fireball);
	}
}
