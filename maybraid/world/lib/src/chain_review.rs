//! Opt-in in-world fixtures for reviewing PR1604. No substitute pose solver or contacts.

use bevy::ecs::system::RunSystemOnce;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bikanwarda_encounter::chains::{
	BikanwardaChainPhysicsPlugin, ChainAnchor, ChainColliderConfig, ChainEndpoint, ChainLinkId,
	ChainMotion, ChainRoot, ChainTensionMode, ReleasedChain, spawn_chain_colliders,
};
use damage::{Health, HitPayload};
use material_ref::MaterialRefRoot;
use projectiles::{ProjectileVisualCache, spawn_flight};
use world_materials::chain::{
	MythicalChainState, OwnedChainMaterial, SkinnedChainConfig, mythical_chain_ref_for,
	spawn_skinned_chain,
};

#[derive(Clone, Debug, clap::Subcommand)]
pub enum ChainReviewCommand {
	/// Two independent chains near the player, using published mesh/capsule helpers.
	Spawn,
	/// Horizontal endpoints; prescribed taut mode.
	Taut,
	/// Same endpoints; prescribed sag mode.
	Catenary,
	/// Toggle motion/rotation of the anchor parents (not chain joint transforms).
	Move,
	/// Real query projectile through chain A's current middle joint, 10 damage.
	Fire,
	/// Insert the public ReleasedChain marker; do not synthesize retirement behavior.
	Release,
	/// Report actual roots, anchors, joint/capsule transforms and health once.
	Inspect,
	/// Explicit fixture cleanup, distinct from the release behavior under review.
	Clear,
}

#[derive(Component)]
pub struct RequestChainReview(pub ChainReviewCommand);
#[derive(Component)]
struct ReviewOwned;

struct Assembly {
	root: Entity,
	anchor_parent: Entity,
	anchors: [Entity; 2],
	mesh: Entity,
	joints: Vec<Entity>,
	colliders: Vec<Entity>,
	origin: Vec3,
}

#[derive(Resource, Default)]
struct ReviewFixture {
	chains: Vec<Assembly>,
	moving: bool,
	elapsed: f32,
}

pub fn configure(app: &mut App) {
	app.init_resource::<ReviewFixture>();
	if !app.is_plugin_added::<BikanwardaChainPhysicsPlugin>() {
		app.add_plugins(BikanwardaChainPhysicsPlugin);
	}
	app.add_systems(Update, (apply_requests, animate_anchors, draw_actual_pose).chain());
}

fn apply_requests(world: &mut World) {
	let requests: Vec<_> = world
		.query::<(Entity, &RequestChainReview)>()
		.iter(world)
		.map(|(entity, request)| (entity, request.0.clone()))
		.collect();
	for (entity, action) in requests {
		world.despawn(entity);
		if matches!(action, ChainReviewCommand::Spawn | ChainReviewCommand::Clear) {
			let owned: Vec<_> =
				world.query_filtered::<Entity, With<ReviewOwned>>().iter(world).collect();
			for entity in owned {
				if let Ok(entity) = world.get_entity_mut(entity) {
					entity.despawn();
				}
			}
			*world.resource_mut::<ReviewFixture>() = ReviewFixture::default();
		}
		match action {
			ChainReviewCommand::Spawn => match world.run_system_once(spawn_fixture) {
				Ok(chains) => world.resource_mut::<ReviewFixture>().chains = chains,
				Err(error) => warn!("[chain-review] spawn failed: {error}"),
			},
			ChainReviewCommand::Clear => info!("[chain-review] explicit fixture cleanup requested"),
			ChainReviewCommand::Move => {
				let mut fixture = world.resource_mut::<ReviewFixture>();
				fixture.moving = !fixture.moving;
				info!(
					"[chain-review] anchor animation={} (gated by WorldGameplayEnabled)",
					fixture.moving
				);
			}
			ChainReviewCommand::Taut | ChainReviewCommand::Catenary => {
				let mode = if matches!(action, ChainReviewCommand::Taut) {
					ChainTensionMode::Taut
				} else {
					ChainTensionMode::Catenary { sag_ratio: 10 }
				};
				world.resource_scope(|world, fixture: Mut<ReviewFixture>| {
					for chain in &fixture.chains {
						if let Some(mut motion) = world.get_mut::<ChainMotion>(chain.root) {
							motion.mode = mode;
						}
						for (index, anchor) in chain.anchors.iter().enumerate() {
							if let Some(mut pose) = world.get_mut::<Transform>(*anchor) {
								pose.translation =
									Vec3::new(if index == 0 { -1.5 } else { 1.5 }, 1.5, 0.0);
							}
						}
					}
				});
			}
			ChainReviewCommand::Release => {
				world.resource_scope(|world, fixture: Mut<ReviewFixture>| {
					for chain in &fixture.chains {
						world.entity_mut(chain.root).insert(ReleasedChain);
					}
				});
				info!(
					"[chain-review] ReleasedChain inserted; no fixture-side collider/joint retirement"
				);
			}
			ChainReviewCommand::Fire => {
				if let Err(error) = world.run_system_once(fire_fixture_projectile) {
					warn!("[chain-review] projectile failed: {error}");
				}
			}
			ChainReviewCommand::Inspect => inspect(world),
		}
	}
}

fn spawn_fixture(
	mut commands: Commands,
	players: Query<&GlobalTransform, With<world_player::Player>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) -> Vec<Assembly> {
	let Ok(player) = players.single() else {
		warn!("[chain-review] requires one ready world player");
		return Vec::new();
	};
	let mut assemblies = Vec::new();
	for index in 0..2 {
		let origin = player.translation() + Vec3::new(index as f32 * 4.5 - 2.25, 0.0, -6.0);
		let label = if index == 0 { "review-A" } else { "review-B" };
		let config = SkinnedChainConfig { link_count: 12, length_m: 3.0, ..default() };
		let end = config.link_spacing() * 11.0;
		let (root, mesh, joints) = spawn_skinned_chain(
			&mut commands,
			&mut meshes,
			&mut bindposes,
			config,
			Transform::IDENTITY,
			label,
		);
		commands.entity(root).insert((
			ReviewOwned,
			Transform::from_translation(origin),
			ChainRoot {
				link_count: 12,
				rest_length_m: 3.0,
				joint_entities: joints.clone(),
				durability: 100.0,
				max_durability: 100.0,
			},
			ChainMotion::default(),
			Health::from_max(100.0),
		));
		commands.entity(mesh).insert((
			ReviewOwned,
			MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)),
			OwnedChainMaterial,
		));
		for joint in &joints {
			commands.entity(*joint).insert(ReviewOwned);
		}
		let colliders =
			spawn_chain_colliders(&mut commands, root, &joints, ChainColliderConfig::default());
		for collider in &colliders {
			commands.entity(*collider).insert(ReviewOwned);
		}
		let parent = commands
			.spawn((
				Name::new(format!("{label}-anchor-parent")),
				Transform::from_translation(origin),
				Visibility::default(),
				ReviewOwned,
			))
			.id();
		let mut anchors = Vec::new();
		for (endpoint, y, color) in [
			(ChainEndpoint::Start, 0.0, Color::srgb(0.1, 1.0, 0.2)),
			(ChainEndpoint::End, end, Color::srgb(1.0, 0.1, 0.1)),
		] {
			let anchor = commands
				.spawn((
					Name::new(format!("{label}-{endpoint:?}")),
					ChainAnchor { endpoint, chain_root: root },
					Transform::from_xyz(0.0, y, 0.0),
					Mesh3d(meshes.add(Sphere::new(0.13))),
					MeshMaterial3d(materials.add(StandardMaterial {
						base_color: color,
						unlit: true,
						..default()
					})),
					Visibility::default(),
					ChildOf(parent),
					ReviewOwned,
				))
				.id();
			anchors.push(anchor);
		}
		info!(
			"[chain-review] {label}: root={root} mesh={mesh} joints={} capsules={} origin={origin:?}; green=start red=end",
			joints.len(),
			colliders.len()
		);
		assemblies.push(Assembly {
			root,
			anchor_parent: parent,
			anchors: [anchors[0], anchors[1]],
			mesh,
			joints,
			colliders,
			origin,
		});
	}
	assemblies
}

fn animate_anchors(
	gameplay: Res<maybraid_session::WorldGameplayEnabled>,
	time: Res<Time>,
	mut fixture: ResMut<ReviewFixture>,
	mut transforms: Query<&mut Transform>,
) {
	if !fixture.moving || !gameplay.0 {
		return;
	}
	fixture.elapsed += time.delta_secs();
	let t = fixture.elapsed;
	for (index, chain) in fixture.chains.iter().enumerate() {
		if let Ok(mut pose) = transforms.get_mut(chain.anchor_parent) {
			let phase = t + index as f32 * 1.7;
			pose.translation = chain.origin + Vec3::new(phase.sin() * 0.75, phase.cos() * 0.2, 0.0);
			pose.rotation =
				Quat::from_rotation_y(phase.sin() * 0.6) * Quat::from_rotation_z(phase.cos() * 0.2);
		}
	}
}

fn fire_fixture_projectile(
	mut commands: Commands,
	fixture: Res<ReviewFixture>,
	transforms: Query<&GlobalTransform>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut visuals: ResMut<ProjectileVisualCache>,
) {
	let Some(chain) = fixture.chains.first() else {
		return;
	};
	let Some(joint) = chain.joints.get(chain.joints.len() / 2) else {
		return;
	};
	let Ok(target) = transforms.get(*joint) else {
		return;
	};
	let muzzle = target.translation() + Vec3::Z * 2.0;
	let projectile = spawn_flight(
		&mut commands,
		&mut meshes,
		&mut materials,
		&mut visuals,
		muzzle,
		Vec3::NEG_Z,
		0.2,
		0.08,
		4.0,
		5.0,
		2.0,
		2.0,
		Color::srgb(1.0, 0.5, 0.1),
		0.0,
	);
	commands.entity(projectile).insert((HitPayload { amount: 10.0 }, ReviewOwned));
	info!(
		"[chain-review] real projectile={projectile} amount=10 target-joint={joint} muzzle={muzzle:?}; no injected contacts"
	);
}

fn inspect(world: &mut World) {
	world.resource_scope(|world, fixture: Mut<ReviewFixture>| {
        info!("[chain-review] inspect chains={} moving={} overlays=collider-transform-markers(not-shape)", fixture.chains.len(), fixture.moving);
        for chain in &fixture.chains {
            let surviving_joints = chain.joints.iter().filter(|entity| world.entities().contains(**entity)).count();
            let surviving_capsules = chain.colliders.iter().filter(|entity| world.get::<ChainLinkId>(**entity).is_some()).count();
            info!("[chain-review] root={} mesh={} alive={} joints={} capsules={} health={:?} motion={:?} released={}",
                chain.root, chain.mesh, world.entities().contains(chain.mesh), surviving_joints, surviving_capsules,
                world.get::<Health>(chain.root), world.get::<ChainMotion>(chain.root), world.get::<ReleasedChain>(chain.root).is_some());
            for anchor in chain.anchors {
                info!("[chain-review] anchor={anchor} identity={:?} world={:?}", world.get::<ChainAnchor>(anchor),
                    world.get::<GlobalTransform>(anchor).map(GlobalTransform::translation));
            }
            for (index, joint) in chain.joints.iter().enumerate() {
                let collider = chain.colliders[index];
                info!("[chain-review] root={} link={index} joint={joint} joint-world={:?} capsule={collider} capsule-world={:?}",
                    chain.root, world.get::<GlobalTransform>(*joint).map(GlobalTransform::translation),
                    world.get::<GlobalTransform>(collider).map(GlobalTransform::translation));
            }
        }
    });
}

/// Fixed-size transform markers, not a rendering of the Collider shape.
fn draw_actual_pose(
	fixture: Res<ReviewFixture>,
	transforms: Query<&GlobalTransform>,
	mut gizmos: Gizmos,
) {
	for (index, chain) in fixture.chains.iter().enumerate() {
		let color =
			if index == 0 { Color::srgb(0.0, 0.8, 1.0) } else { Color::srgb(1.0, 0.0, 1.0) };
		for collider in &chain.colliders {
			if let Ok(pose) = transforms.get(*collider) {
				let at = pose.translation();
				let axis = pose.rotation() * Vec3::Y * 0.08;
				gizmos.line(at - axis, at + axis, color);
				gizmos.sphere(Isometry3d::from_translation(at - axis), 0.052, color);
				gizmos.sphere(Isometry3d::from_translation(at + axis), 0.052, color);
			}
		}
	}
}

#[cfg(all(test, feature = "chain-review"))]
mod tests {
	use super::*;

	#[test]
	fn chain_review_fixture_has_valid_joint_entities() {
		use bevy::ecs::system::RunSystemOnce;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<Assets<Mesh>>()
			.init_resource::<Assets<SkinnedMeshInverseBindposes>>()
			.init_resource::<Assets<StandardMaterial>>();
		app.world_mut()
			.spawn((world_player::Player, GlobalTransform::IDENTITY));

		let assemblies = app
			.world_mut()
			.run_system_once(spawn_fixture)
			.expect("spawn_fixture system should succeed");

		assert_eq!(assemblies.len(), 2, "Should spawn two chain assemblies");

		for (index, assembly) in assemblies.iter().enumerate() {
			let label = if index == 0 { "review-A" } else { "review-B" };

			assert_eq!(
				assembly.joints.len(),
				12,
				"{label}: joints vector should have 12 entries"
			);

			let root = app.world().entity(assembly.root);
			let chain_root = root
				.get::<ChainRoot>()
				.unwrap_or_else(|| panic!("{label}: ChainRoot component not found on root"));

			assert_eq!(
				chain_root.joint_entities.len(),
				12,
				"{label}: ChainRoot.joint_entities should have 12 entries"
			);

			assert_eq!(
				chain_root.joint_entities, assembly.joints,
				"{label}: ChainRoot.joint_entities should match the joints from spawn"
			);

			let root_transform = root
				.get::<Transform>()
				.unwrap_or_else(|| panic!("{label}: Transform not found on root"));

			let expected_origin = assembly.origin;
			assert_eq!(
				root_transform.translation, expected_origin,
				"{label}: Root translation should match the assembly origin"
			);
		}
	}
}
