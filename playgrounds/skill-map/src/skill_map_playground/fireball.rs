use bevy::prelude::*;

#[derive(Component, Debug, Clone, Default)]
pub struct Fireball {
	age: f32,
	max_age: f32,
	radius: f32,
	radius_decay: f32,
	velocity: Vec3,
	velocity_decay: f32,
}

impl Fireball {
	pub fn new(
		max_age: f32,
		radius: f32,
		radius_decay: f32,
		velocity: Vec3,
		velocity_decay: f32,
	) -> Self {
		Self { age: 0.0, max_age, radius, radius_decay, velocity, velocity_decay }
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

	pub fn next(&self, elapsed_time: f32, position: Vec3) -> Option<(Self, Vec3)> {
		if self.age() + elapsed_time > self.max_age() {
			return None;
		}

		let new_position = position + self.velocity * elapsed_time;

		// the velocity decays with the square of the time since birth
		let velocity = self.velocity - self.velocity_decay * elapsed_time * elapsed_time;

		// the radius decays with the square of the time since birth
		let radius = self.radius - self.radius_decay * elapsed_time * elapsed_time;

		Some((
			Self {
				age: self.age + elapsed_time,
				max_age: self.max_age,
				radius,
				radius_decay: self.radius_decay,
				velocity,
				velocity_decay: self.velocity_decay,
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
		query: Query<(Entity, &Fireball, &Transform), With<Fireball>>,
	) {
		for (entity, fireball, transform) in query.iter() {
			if let Some((fireball, position)) =
				fireball.next(time.elapsed_secs(), transform.translation)
			{
				// translate the fireball
				commands.entity(entity).insert(Transform::from_translation(position));

				// update the rendering
				commands
					.entity(entity)
					.insert(Mesh3d(meshes.add(Sphere { radius: fireball.radius(), ..default() })));
				commands.entity(entity).insert(MeshMaterial3d(materials.add(StandardMaterial {
					base_color: Color::srgba(1.0, 0.0, 0.0, 0.5),
					alpha_mode: AlphaMode::AlphaToCoverage,
					..default()
				})));

				// replace the fireball with a new one
				commands.entity(entity).insert(fireball);
			} else {
				// despawn the fireball
				commands.entity(entity).despawn();
			}
		}
	}

	pub fn dispatch_camera_fireball(
		mut commands: Commands,
		dispatch_query: Query<(Entity, &DispatchCameraFireball), Added<DispatchCameraFireball>>,
		camera_query: Query<&Transform, With<Camera3d>>,
	) {
		for (entity, dispatch) in dispatch_query.iter() {
			if let Ok(camera) = camera_query.single() {
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
