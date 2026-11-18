//! Integration tests for anchored chain motion and collision ([#1126](https://github.com/ramate-io/maybraid/issues/1126)).
//!
//! Tests cover:
//! - Motion stability and anchoring
//! - Collision identity and capsule-joint sync
//! - Contact deduplication (overlapping capsules)
//! - Health routing through ChainLinkId

#[cfg(test)]
mod tests {
	use anyhow::{anyhow, Result};
	use avian3d::prelude::*;
	use bevy::ecs::message::MessageId;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use damage::{ChainSegment, DamageApplied, DamagePlugin, Health, Hit, HitPayload};
	use encounters::EncounterSignal;
	use projectiles::ProjectileContact;

	use crate::chains::{
		coalesce_chain_contacts, deactivate_released_chains, detect_and_signal_broken_chains,
		spawn_chain_colliders, sync_chain_colliders, update_chain_joints,
		BikanwardaChainPhysicsPlugin, ChainAnchor, ChainBrokenHistory, ChainColliderConfig,
		ChainEndpoint, ChainLinkId, ChainMotion, ChainRoot, ChainTensionMode, ReleasedChain,
	};

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, DamagePlugin))
			.init_resource::<crate::chains::ChainDamageHistory>()
			.init_resource::<ChainBrokenHistory>()
			.add_message::<EncounterSignal>()
			.add_systems(
				PostUpdate,
				(
					update_chain_joints,
					sync_chain_colliders.after(update_chain_joints),
					detect_and_signal_broken_chains.after(damage::DamageSystems::Apply),
				)
					.after(TransformSystems::Propagate),
			)
			.add_systems(
				PostUpdate,
				// Contact deduplication runs in Collect, explicitly BEFORE contacts_to_hits
				coalesce_chain_contacts
					.in_set(damage::DamageSystems::Collect)
					.before(damage::contacts_to_hits),
			)
			.add_systems(Update, deactivate_released_chains);
		app.finish();
		// Run one update to initialize systems
		app.update();
		app
	}

	#[derive(Resource, Default)]
	struct CollectedDamage {
		events: Vec<DamageApplied>,
	}

	#[derive(Resource, Default)]
	struct CollectedContactIds {
		ids: Vec<MessageId<ProjectileContact>>,
	}

	fn collect_damage_applied(
		mut reader: MessageReader<DamageApplied>,
		mut collected: ResMut<CollectedDamage>,
	) {
		collected.events.extend(reader.read().copied());
	}

	fn collect_contact_ids(
		mut reader: MessageReader<ProjectileContact>,
		mut collected: ResMut<CollectedContactIds>,
	) {
		collected.ids.extend(reader.read_with_id().map(|(_, id)| id));
	}

	fn install_persistent_collectors(app: &mut App) {
		app.init_resource::<CollectedDamage>()
			.init_resource::<CollectedContactIds>()
			.add_systems(
				PostUpdate,
				(collect_damage_applied.after(damage::DamageSystems::Apply), collect_contact_ids),
			);
	}

	#[test]
	fn anchored_chain_follows_socket_transforms() -> Result<()> {
		let mut app = test_app();

		let joint0 = app
			.world_mut()
			.spawn((Name::new("chain-joint-0"), Transform::default(), GlobalTransform::default()))
			.id();
		let joint1 = app
			.world_mut()
			.spawn((Name::new("chain-joint-1"), Transform::default(), GlobalTransform::default()))
			.id();
		let joint2 = app
			.world_mut()
			.spawn((Name::new("chain-joint-2"), Transform::default(), GlobalTransform::default()))
			.id();

		// Spawn chain root with joints in Taut mode
		let root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![joint0, joint1, joint2],
				},
				ChainMotion { mode: ChainTensionMode::Taut, active: true },
				Transform::default(),
				GlobalTransform::default(),
				Visibility::default(),
			))
			.id();

		app.world_mut().entity_mut(root).add_children(&[joint0, joint1, joint2]);

		// Spawn anchor entities with both Transform and GlobalTransform
		let start_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::from_xyz(0.0, 0.0, 0.0),
			))
			.id();

		let end_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
				Transform::from_xyz(0.0, 3.0, 0.0),
				GlobalTransform::from_xyz(0.0, 3.0, 0.0),
			))
			.id();

		// Update multiple times to propagate transforms and run motion system
		for _ in 0..5 {
			app.update();
		}

		// Check that joints' WORLD positions (via GlobalTransform) match expected anchor positions
		let j0_world = app.world().get::<GlobalTransform>(joint0).unwrap().translation();
		let j1_world = app.world().get::<GlobalTransform>(joint1).unwrap().translation();
		let j2_world = app.world().get::<GlobalTransform>(joint2).unwrap().translation();

		let epsilon = 0.1;

		// With taut mode, joints should be evenly spaced along Y in world space
		assert!(
			(j0_world.y - 0.0).abs() < epsilon,
			"Joint 0 world Y should be near start (0.0), got {}",
			j0_world.y
		);
		assert!(
			(j1_world.y - 1.5).abs() < epsilon,
			"Joint 1 world Y should be at midpoint (1.5), got {}",
			j1_world.y
		);
		assert!(
			(j2_world.y - 3.0).abs() < epsilon,
			"Joint 2 world Y should be near end (3.0), got {}",
			j2_world.y
		);

		// Move end anchor (update both Transform and GlobalTransform since it has no parent)
		if let Some(mut transform) = app.world_mut().get_mut::<Transform>(end_anchor) {
			transform.translation = Vec3::new(2.0, 4.0, 0.0);
		}
		if let Some(mut global) = app.world_mut().get_mut::<GlobalTransform>(end_anchor) {
			*global = GlobalTransform::from_xyz(2.0, 4.0, 0.0);
		}

		for _ in 0..5 {
			app.update();
		}

		// Joints should follow the new anchor position (check world position via GlobalTransform)
		let new_j2_world = app.world().get::<GlobalTransform>(joint2).unwrap().translation();
		let expected_world_pos = Vec3::new(2.0, 4.0, 0.0);
		assert!(
			(new_j2_world - expected_world_pos).length() < 0.1,
			"End joint world position should follow anchor, expected {:?}, got {:?}",
			expected_world_pos,
			new_j2_world
		);

		Ok(())
	}

	#[test]
	fn capsule_colliders_sync_to_joint_positions() -> Result<()> {
		let mut app = test_app();

		let joint0 = app
			.world_mut()
			.spawn((Name::new("chain-joint-0"), Transform::from_xyz(0.0, 0.0, 0.0)))
			.id();
		let joint1 = app
			.world_mut()
			.spawn((Name::new("chain-joint-1"), Transform::from_xyz(0.0, 2.0, 0.0)))
			.id();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 2,
					rest_length_m: 2.0,
					joint_entities: vec![joint0, joint1],
				},
				ChainMotion { mode: ChainTensionMode::Taut, active: false },
				Transform::default(),
				Visibility::default(),
			))
			.id();

		app.world_mut().entity_mut(root).add_children(&[joint0, joint1]);

		// Spawn colliders
		let config = ChainColliderConfig::default();
		let colliders = app
			.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_chain_colliders(&mut commands, root, &[joint0, joint1], config)
			})
			.unwrap();

		app.update();

		// Check that colliders are at joint positions
		let collider0_pos = app.world().get::<Transform>(colliders[0]).unwrap().translation;
		let collider1_pos = app.world().get::<Transform>(colliders[1]).unwrap().translation;

		assert!(
			(collider0_pos - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-4,
			"Collider 0 should be at joint 0"
		);
		assert!(
			(collider1_pos - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-4,
			"Collider 1 should be at joint 1"
		);

		// Move joint and verify collider follows
		if let Some(mut transform) = app.world_mut().get_mut::<Transform>(joint0) {
			transform.translation = Vec3::new(1.0, 0.0, 0.0);
		}

		app.update();

		let new_collider0_pos = app.world().get::<Transform>(colliders[0]).unwrap().translation;
		assert!(
			(new_collider0_pos - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4,
			"Collider should follow joint"
		);

		Ok(())
	}

	#[test]
	fn chain_link_id_provides_stable_identity() -> Result<()> {
		let mut app = test_app();

		let root = app.world_mut().spawn_empty().id();
		let joints = vec![
			app.world_mut().spawn(Transform::default()).id(),
			app.world_mut().spawn(Transform::default()).id(),
		];

		let config = ChainColliderConfig::default();
		let colliders = app
			.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_chain_colliders(&mut commands, root, &joints, config)
			})
			.unwrap();

		// Verify link IDs
		for (i, collider) in colliders.iter().enumerate() {
			let link_id = app.world().get::<ChainLinkId>(*collider).unwrap();
			assert_eq!(link_id.link_id, i as u8, "Link ID should match index");
			assert_eq!(link_id.chain_root, root, "Chain root should match");
		}

		Ok(())
	}

	#[test]
	fn overlapping_capsule_contacts_deduplicate_by_projectile() -> Result<()> {
		let mut app = test_app();

		let root = app.world_mut().spawn_empty().id();
		let capsule0 = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root: root }, ChainSegment))
			.id();
		let capsule1 = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 1, chain_root: root }, ChainSegment))
			.id();

		let projectile = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let source = Entity::from_bits(100);

		// Same projectile hits two adjacent capsules
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(source),
			target: capsule0,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(source),
			target: capsule1,
			point: Vec3::new(1.1, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let hits: Vec<_> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		if hits.len() != 1 {
			return Err(anyhow!("Expected 1 coalesced Hit, got {}", hits.len()));
		}

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();
		if contacts.len() != 2 {
			return Err(anyhow!(
				"Shared contact queue must stay intact, got {} contacts",
				contacts.len()
			));
		}

		Ok(())
	}

	#[test]
	fn multiple_projectiles_from_same_source_do_not_coalesce() -> Result<()> {
		let mut app = test_app();

		let root = app.world_mut().spawn_empty().id();
		let capsule = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root: root }, ChainSegment))
			.id();

		let projectile_a = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let projectile_b = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let source = Entity::from_bits(100);

		// Two different projectiles from the same source hit the chain
		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_a,
			source: Some(source),
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_b,
			source: Some(source),
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let hits: Vec<_> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		if hits.len() != 2 {
			return Err(anyhow!("Expected 2 Hits (distinct projectiles), got {}", hits.len()));
		}

		Ok(())
	}

	#[test]
	fn motion_system_runs_with_app_update() -> Result<()> {
		let mut app = test_app();

		let link_count = 12;
		let length_m = 3.0;

		// Create joints at origin
		let mut joints = Vec::new();
		for _i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default()))
				.id();
			joints.push(joint);
		}

		// Create chain root
		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Create horizontal anchors at same height
		let _left_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
				Transform::from_xyz(-1.5, 2.5, 0.0),
				GlobalTransform::default(),
			))
			.id();

		let _right_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
				Transform::from_xyz(1.5, 2.5, 0.0),
				GlobalTransform::default(),
			))
			.id();

		// Run several updates to let motion system work
		for _ in 0..5 {
			app.update();
		}

		// Check that midpoint joint moved from origin
		let midpoint_joint = joints[6];
		let joint_transform = app.world().entity(midpoint_joint).get::<Transform>().unwrap();

		// Joint should have moved from (0,0,0) to somewhere near the catenary midpoint
		let moved_distance = joint_transform.translation.length();
		assert!(
			moved_distance > 1.0,
			"Motion system should have moved joint from origin, moved only {moved_distance}m"
		);

		// Midpoint should be below anchor height (sagging)
		assert!(
			joint_transform.translation.y < 2.5,
			"Catenary midpoint should sag below anchor height 2.5, got y={}",
			joint_transform.translation.y
		);

		Ok(())
	}

	#[test]
	fn projectile_collision_coalesces_to_single_damage() -> Result<()> {
		use avian3d::prelude::*;
		use bevy::time::TimeUpdateStrategy;
		use damage::{DamagePlugin, Health, HitPayload};
		use lod_avian::PhysicsInteractionLayer;
		use projectiles::{Flight, ProjectileSource, ProjectilesPlugin};

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		))
		.add_message::<EncounterSignal>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
		app.finish();

		let link_count = 12;
		let length_m = 3.0;
		let initial_health = 50.0; // Low health so we can test death
		let damage_per_hit = 25.0;

		// Spawn joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root with health
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				Health { current: initial_health, max: initial_health },
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Spawn horizontal anchors at same height, horizontally apart
		let _left = app
			.world_mut()
			.spawn((
				Name::new("left-anchor"),
				ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
				Transform::from_xyz(-1.5, 2.5, 0.0),
				GlobalTransform::default(),
			))
			.id();

		let _right = app
			.world_mut()
			.spawn((
				Name::new("right-anchor"),
				ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
				Transform::from_xyz(1.5, 2.5, 0.0),
				GlobalTransform::default(),
			))
			.id();

		// Run updates to let motion system position joints
		app.update();
		let _ = app.world_mut().run_system_once(update_chain_joints);
		app.update();

		// Spawn chain colliders
		let config = ChainColliderConfig::default();
		let colliders = app
			.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_chain_colliders(&mut commands, root, &joints, config)
			})
			.unwrap();

		app.update(); // Apply spawned colliders and sync positions

		// Verify colliders are positioned (they should be spread along the catenary)
		let collider_positions: Vec<Vec3> = colliders
			.iter()
			.filter_map(|&e| app.world().get::<Transform>(e).map(|t| t.translation))
			.collect();
		if collider_positions.len() < 3 {
			return Err(anyhow!("Expected at least 3 positioned colliders"));
		}

		// Find the sag point (lowest y position, should be near the middle)
		let min_y = collider_positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);

		// Spawn projectile aimed through the sag point where 2-3 capsules overlap
		let projectile_start = Vec3::new(-2.0, min_y, 0.0); // Left of chain at sag height
		let projectile_velocity = Vec3::new(15.0, 0.0, 0.0); // Moving right through chain
		let source = app.world_mut().spawn(Name::new("weapon")).id();

		let _projectile = app
			.world_mut()
			.spawn((
				Name::new("test-projectile"),
				Flight::spawn(projectile_start, 100.0, 100.0, 10.0),
				ProjectileSource(source),
				HitPayload { amount: damage_per_hit },
				Transform::from_translation(projectile_start),
				GlobalTransform::default(),
				LinearVelocity(projectile_velocity),
				Collider::sphere(0.05),
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				RigidBody::Dynamic,
				GravityScale(0.0),
			))
			.id();

		// Run until collision happens and damage is applied (max 60 frames = 1 second at 16ms)
		let mut damage_frame = None;
		for frame in 0..60 {
			app.update();

			// Check if health has changed (damage was applied)
			if let Some(health) = app.world().entity(root).get::<Health>() {
				if health.current < initial_health {
					damage_frame = Some(frame);
					break;
				}
			}
		}

		let damage_frame =
			damage_frame.ok_or_else(|| anyhow!("No damage applied after 60 frames (1 second)"))?;

		info!("Damage applied at frame {damage_frame}");

		// Check health was reduced
		let health_after = app.world().entity(root).get::<Health>().unwrap();
		let damage_taken = initial_health - health_after.current;

		if damage_taken <= 0.0 {
			return Err(anyhow!(
				"Expected damage to be applied, but health unchanged (initial: {}, current: {})",
				initial_health,
				health_after.current
			));
		}

		// Verify approximately one hit's worth of damage (allowing for slight variations)
		if (damage_taken - damage_per_hit).abs() > 5.0 {
			return Err(anyhow!(
				"Expected ~{damage_per_hit} damage (one hit), but took {damage_taken} damage. \
				 This suggests contacts were not properly coalesced."
			));
		}

		info!("✓ Collision coalesced correctly: {damage_taken} damage applied (expected ~{damage_per_hit})");

		// Test death scenario: spawn another projectile to finish off the chain
		let _projectile2 = app
			.world_mut()
			.spawn((
				Name::new("finishing-projectile"),
				Flight::spawn(projectile_start, 100.0, 100.0, 10.0),
				ProjectileSource(source),
				HitPayload { amount: damage_per_hit },
				Transform::from_translation(projectile_start),
				GlobalTransform::default(),
				LinearVelocity(projectile_velocity),
				Collider::sphere(0.05),
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				RigidBody::Dynamic,
				GravityScale(0.0),
			))
			.id();

		// Run until second hit depletes health (max 60 frames)
		let mut killed = false;
		for _ in 0..60 {
			app.update();

			// Check if chain is dead
			if let Some(health) = app.world().entity(root).get::<Health>() {
				if health.is_dead() {
					killed = true;
					break;
				}
			}
		}

		if !killed {
			return Err(anyhow!("Second projectile did not kill chain"));
		}

		// Verify death
		let final_health = app.world().entity(root).get::<Health>().unwrap();
		if !final_health.is_dead() {
			return Err(anyhow!(
				"Expected chain to die after two hits, but health is {}",
				final_health.current
			));
		}

		// Check for Died message (it may have been processed already, so we check health.is_dead())
		info!("✓ Chain died after taking lethal damage");

		Ok(())
	}

	#[test]
	fn penetrating_projectile_applies_damage_only_once_across_frames() -> Result<()> {
		use avian3d::prelude::*;
		use bevy::time::TimeUpdateStrategy;
		use damage::{DamagePlugin, Health, HitPayload};
		use lod_avian::PhysicsInteractionLayer;
		use projectiles::{Flight, ProjectileSource, ProjectilesPlugin};

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		))
		.add_message::<EncounterSignal>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
		app.finish();

		let link_count = 12;
		let length_m = 3.0;
		let initial_health = 100.0;
		let damage_per_hit = 10.0;

		// Spawn joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root with health
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain-multiframe"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				Health { current: initial_health, max: initial_health },
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Spawn horizontal anchors
		app.world_mut().spawn((
			Name::new("left-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_xyz(-1.5, 2.5, 0.0),
			GlobalTransform::default(),
		));

		app.world_mut().spawn((
			Name::new("right-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_xyz(1.5, 2.5, 0.0),
			GlobalTransform::default(),
		));

		// Run updates to position joints
		for _ in 0..5 {
			app.update();
		}

		// Spawn chain colliders
		let config = ChainColliderConfig::default();
		let colliders = app
			.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_chain_colliders(&mut commands, root, &joints, config)
			})
			.unwrap();

		// Run several updates to let colliders sync to joint positions
		for _ in 0..5 {
			app.update();
		}

		// Find the chain center position for aiming the projectile
		let chain_center_y = 2.5; // Anchors are at y=2.5, horizontal span
		let chain_center_x = 0.0; // Midpoint between -1.5 and 1.5

		// Verify colliders are positioned
		let positioned_colliders =
			colliders.iter().filter(|&&e| app.world().get_entity(e).is_ok()).count();

		if positioned_colliders < 3 {
			return Err(anyhow!(
				"Expected at least 3 positioned colliders, got {}",
				positioned_colliders
			));
		}

		// Spawn a penetrating projectile aimed at the chain center
		// Use a moderate speed so it takes multiple frames to pass through
		let projectile_start = Vec3::new(-2.0, chain_center_y, 0.0);
		let projectile_velocity = Vec3::new(3.0, 0.0, 0.0); // Moderate speed: 3 m/s
		let source = app.world_mut().spawn(Name::new("weapon")).id();

		let projectile = app
			.world_mut()
			.spawn((
				Name::new("penetrating-projectile"),
				Flight::spawn(projectile_start, 100.0, 500.0, 100.0), // Very high penetration (500)
				ProjectileSource(source),
				HitPayload { amount: damage_per_hit },
				Transform::from_translation(projectile_start),
				GlobalTransform::default(),
				LinearVelocity(projectile_velocity),
				Collider::sphere(0.1), // Larger sphere for better overlap detection
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				RigidBody::Dynamic,
				GravityScale(0.0),
			))
			.id();

		// Track health changes frame by frame and verify damage source
		let mut health_history = Vec::new();
		let mut projectile_alive = true;
		let mut damage_source_verified = false;

		// Run for at least 10 frames to ensure the projectile passes through
		for frame in 0..15 {
			app.update();

			let current_health =
				app.world().entity(root).get::<Health>().map(|h| h.current).unwrap_or(0.0);
			health_history.push(current_health);

			// Check damage records to verify source is set
			if !damage_source_verified {
				let damage_records: Vec<_> =
					app.world_mut().resource_mut::<Messages<DamageApplied>>().drain().collect();

				for record in &damage_records {
					if record.target == root {
						if record.source.is_none() {
							return Err(anyhow!(
								"Damage record has source=null! Expected source={:?}",
								source
							));
						}
						if record.source != Some(source) {
							return Err(anyhow!(
								"Damage record source mismatch: expected {:?}, got {:?}",
								source,
								record.source
							));
						}
						damage_source_verified = true;
						info!("✓ Damage source verified: {:?}", record.source);
					}
				}
			}

			// Check if projectile still exists
			if app.world().get_entity(projectile).is_err() {
				projectile_alive = false;
			}

			// Exit early if we've seen damage and projectile is gone
			if !projectile_alive && current_health < initial_health {
				info!("Projectile despawned at frame {frame}, health: {current_health}");
				break;
			}
		}

		// Check that damage was applied
		let final_health = health_history.last().copied().unwrap_or(initial_health);

		// Verify we actually checked the source
		if !damage_source_verified && final_health < initial_health {
			return Err(anyhow!(
				"Damage was applied but source was never verified in DamageApplied records"
			));
		}
		if final_health >= initial_health {
			return Err(anyhow!("No damage was applied after 15 frames"));
		}

		// Critical check: damage should be applied EXACTLY ONCE
		let total_damage = initial_health - final_health;
		if (total_damage - damage_per_hit).abs() > 1.0 {
			return Err(anyhow!(
				"Penetrating projectile applied damage multiple times across frames!\n\
				 Initial health: {initial_health}, Final health: {final_health}\n\
				 Total damage: {total_damage} (expected {damage_per_hit})\n\
				 Health history: {:?}",
				health_history
			));
		}

		info!(
			"✓ Penetrating projectile correctly applied damage only once: {} → {} (lost {total_damage})",
			initial_health, final_health
		);

		Ok(())
	}

	#[test]
	fn released_chain_deactivates_motion() -> Result<()> {
		let mut app = test_app();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 2, rest_length_m: 2.0, joint_entities: Vec::new() },
				ChainMotion { mode: ChainTensionMode::Taut, active: true },
			))
			.id();

		// Mark as released
		app.world_mut().entity_mut(root).insert(ReleasedChain);

		app.update();

		let motion = app.world().get::<ChainMotion>(root).unwrap();
		if motion.active {
			return Err(anyhow!("Released chain motion should be deactivated"));
		}

		Ok(())
	}

	#[test]
	fn released_chain_retires_colliders_and_blocks_damage() -> Result<()> {
		use avian3d::prelude::*;
		use bevy::time::TimeUpdateStrategy;
		use damage::{DamagePlugin, Health, HitPayload};
		use lod_avian::PhysicsInteractionLayer;
		use projectiles::{Flight, ProjectileSource, ProjectilesPlugin};

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		))
		.add_message::<EncounterSignal>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
		app.finish();

		let link_count = 12;
		let length_m = 3.0;
		let initial_health = 100.0;
		let damage_per_hit = 25.0;

		// Spawn joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root with health
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain-release"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				Health { current: initial_health, max: initial_health },
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Spawn anchors
		app.world_mut().spawn((
			Name::new("left-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_xyz(-1.5, 2.5, 0.0),
			GlobalTransform::default(),
		));

		app.world_mut().spawn((
			Name::new("right-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_xyz(1.5, 2.5, 0.0),
			GlobalTransform::default(),
		));

		// Run updates to position joints
		for _ in 0..3 {
			app.update();
		}

		// Spawn chain colliders
		let config = ChainColliderConfig::default();
		let colliders = app
			.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_chain_colliders(&mut commands, root, &joints, config)
			})
			.unwrap();

		app.update();

		// Verify colliders exist
		let collider_count =
			colliders.iter().filter(|&&e| app.world().get_entity(e).is_ok()).count();
		if collider_count != link_count as usize {
			return Err(anyhow!("Expected {} colliders, found {}", link_count, collider_count));
		}

		// Mark chain as released
		app.world_mut().entity_mut(root).insert(ReleasedChain);

		// Run update to retire colliders
		app.update();

		// Verify colliders are despawned
		let remaining_colliders =
			colliders.iter().filter(|&&e| app.world().get_entity(e).is_ok()).count();
		if remaining_colliders > 0 {
			return Err(anyhow!(
				"Released chain should retire colliders, but {} remain",
				remaining_colliders
			));
		}

		// Spawn projectile AFTER release
		let projectile_start = Vec3::new(-2.0, 2.5, 0.0);
		let projectile_velocity = Vec3::new(15.0, 0.0, 0.0);
		let source = app.world_mut().spawn(Name::new("weapon")).id();

		app.world_mut().spawn((
			Name::new("post-release-projectile"),
			Flight::spawn(projectile_start, 100.0, 100.0, 10.0),
			ProjectileSource(source),
			HitPayload { amount: damage_per_hit },
			Transform::from_translation(projectile_start),
			GlobalTransform::default(),
			LinearVelocity(projectile_velocity),
			Collider::sphere(0.05),
			Sensor,
			PhysicsInteractionLayer::projectile_layers(),
			RigidBody::Dynamic,
			GravityScale(0.0),
		));

		// Run for many frames - projectile should pass through without hitting
		for _ in 0..30 {
			app.update();
		}

		// Verify health unchanged
		let final_health = app.world().entity(root).get::<Health>().unwrap().current;
		if (final_health - initial_health).abs() > 0.1 {
			return Err(anyhow!(
				"Released chain should not take damage, but health changed from {} to {}",
				initial_health,
				final_health
			));
		}

		Ok(())
	}

	#[test]
	fn translated_and_rotated_root_joints_match_anchor_world_positions() -> Result<()> {
		let mut app = test_app();

		let link_count = 8;
		let length_m = 2.5;

		// Create joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root with BOTH translation AND rotation (non-identity transform)
		let root_translation = Vec3::new(1744.55, -351.17, 1031.9);
		let root_rotation = Quat::from_rotation_y(std::f32::consts::PI / 6.0)
			* Quat::from_rotation_x(std::f32::consts::PI / 12.0);
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain-translated-rotated-root"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Taut, active: true },
				Transform::from_translation(root_translation).with_rotation(root_rotation),
				GlobalTransform::default(),
				Visibility::default(),
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Create anchors with world-space positions (NOT relative to root)
		let start_world = Vec3::new(2000.0, -300.0, 1000.0);
		let end_world = Vec3::new(2000.0, -300.0 + length_m, 1000.0);

		app.world_mut().spawn((
			Name::new("world-space-start-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_translation(start_world),
			GlobalTransform::from_translation(start_world),
		));

		app.world_mut().spawn((
			Name::new("world-space-end-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_translation(end_world),
			GlobalTransform::from_translation(end_world),
		));

		// Run several updates to let transforms propagate and motion system work
		for _ in 0..10 {
			app.update();
		}

		// Critical check: joint WORLD positions (via GlobalTransform) must match anchor world positions
		let first_joint_world =
			app.world().get::<GlobalTransform>(joints[0]).unwrap().translation();
		let last_joint_world = app
			.world()
			.get::<GlobalTransform>(joints[link_count as usize - 1])
			.unwrap()
			.translation();

		let epsilon = 0.01; // 1cm tolerance

		let start_error = (first_joint_world - start_world).length();
		if start_error > epsilon {
			return Err(anyhow!(
				"First joint world position mismatch!\n\
				 Expected: {:?}\n\
				 Got:      {:?}\n\
				 Error:    {:.4}m (epsilon: {:.4}m)\n\
				 Root translation: {:?}, rotation: {:?}",
				start_world,
				first_joint_world,
				start_error,
				epsilon,
				root_translation,
				root_rotation
			));
		}

		let end_error = (last_joint_world - end_world).length();
		if end_error > epsilon {
			return Err(anyhow!(
				"Last joint world position mismatch!\n\
				 Expected: {:?}\n\
				 Got:      {:?}\n\
				 Error:    {:.4}m (epsilon: {:.4}m)",
				end_world,
				last_joint_world,
				end_error,
				epsilon
			));
		}

		Ok(())
	}

	#[test]
	fn translated_and_rotated_anchors_drive_chain_motion() -> Result<()> {
		let mut app = test_app();

		let link_count = 8;
		let length_m = 2.5;

		// Create joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain-transformed-anchors"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Taut, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				Visibility::default(),
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Create TRANSLATED start anchor (offset from origin)
		let start_translation = Vec3::new(2.0, 3.0, -1.0);
		app.world_mut().spawn((
			Name::new("translated-start-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_translation(start_translation),
			GlobalTransform::from_translation(start_translation),
		));

		// Create TRANSLATED and ROTATED end anchor (45 degree rotation + offset)
		let end_translation = Vec3::new(4.0, 5.0, 1.0);
		let end_rotation = Quat::from_rotation_y(std::f32::consts::PI / 4.0);
		app.world_mut().spawn((
			Name::new("rotated-end-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_translation(end_translation).with_rotation(end_rotation),
			GlobalTransform::from(
				Transform::from_translation(end_translation).with_rotation(end_rotation),
			),
		));

		// Run several updates to let transforms propagate and motion system work
		for _ in 0..5 {
			app.update();
		}

		// Verify joints moved to anchor positions
		let first_joint_pos = app.world().get::<Transform>(joints[0]).unwrap().translation;
		let last_joint_pos = app
			.world()
			.get::<Transform>(joints[link_count as usize - 1])
			.unwrap()
			.translation;

		// First joint should be at start anchor
		let start_dist = (first_joint_pos - start_translation).length();
		if start_dist > 0.1 {
			return Err(anyhow!(
				"First joint should be at translated start anchor ({}), got {}, distance: {}",
				start_translation,
				first_joint_pos,
				start_dist
			));
		}

		// Last joint should be at end anchor
		let end_dist = (last_joint_pos - end_translation).length();
		if end_dist > 0.1 {
			return Err(anyhow!(
				"Last joint should be at translated+rotated end anchor ({}), got {}, distance: {}",
				end_translation,
				last_joint_pos,
				end_dist
			));
		}

		// Verify joints actually moved from origin
		let moved_from_origin = joints.iter().any(|&joint| {
			let pos = app.world().get::<Transform>(joint).unwrap().translation;
			pos.length() > 1.0
		});

		if !moved_from_origin {
			return Err(anyhow!("Joints should have moved from origin to follow anchors"));
		}

		Ok(())
	}

	#[test]
	fn catenary_mode_with_transformed_anchors() -> Result<()> {
		let mut app = test_app();

		let link_count = 10;
		let length_m = 3.0;

		// Create joints
		let mut joints = Vec::new();
		for i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((
					Name::new(format!("chain-joint-{i}")),
					Transform::from_xyz(0.0, 0.0, 0.0),
					GlobalTransform::default(),
				))
				.id();
			joints.push(joint);
		}

		// Spawn chain root with catenary mode
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain-catenary-transformed"),
				ChainRoot { link_count, rest_length_m: length_m, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 8 }, active: true },
				Transform::from_xyz(0.0, 0.0, 0.0),
				Visibility::default(),
			))
			.id();

		for joint in &joints {
			app.world_mut().entity_mut(root).add_child(*joint);
		}

		// Horizontal span with vertical offset
		let left_pos = Vec3::new(-2.0, 5.0, 0.0);
		let right_pos = Vec3::new(2.0, 5.0, 0.0);

		app.world_mut().spawn((
			Name::new("left-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_translation(left_pos),
			GlobalTransform::from_translation(left_pos),
		));

		app.world_mut().spawn((
			Name::new("right-anchor"),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_translation(right_pos),
			GlobalTransform::from_translation(right_pos),
		));

		// Run updates
		for _ in 0..5 {
			app.update();
		}

		// Verify catenary sag: middle joint should be below the anchor line
		let mid_joint = joints[link_count as usize / 2];
		let mid_pos = app.world().get::<Transform>(mid_joint).unwrap().translation;

		if mid_pos.y >= left_pos.y {
			return Err(anyhow!(
				"Catenary middle joint should sag below horizontal anchor line (y < {}), got y={}",
				left_pos.y,
				mid_pos.y
			));
		}

		// Verify endpoints are at anchors
		let first_pos = app.world().get::<Transform>(joints[0]).unwrap().translation;
		let last_pos = app
			.world()
			.get::<Transform>(joints[link_count as usize - 1])
			.unwrap()
			.translation;

		if (first_pos - left_pos).length() > 0.1 {
			return Err(anyhow!("Start joint should be at left anchor"));
		}

		if (last_pos - right_pos).length() > 0.1 {
			return Err(anyhow!("End joint should be at right anchor"));
		}

		Ok(())
	}

	#[test]
	fn catenary_mode_adds_sag() -> Result<()> {
		let mut app = test_app();

		let joints: Vec<_> = (0..5)
			.map(|i| {
				app.world_mut()
					.spawn((Name::new(format!("chain-joint-{}", i)), Transform::default()))
					.id()
			})
			.collect();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 5, rest_length_m: 4.0, joint_entities: joints.clone() },
				ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
				Transform::default(),
				Visibility::default(),
			))
			.id();

		app.world_mut().entity_mut(root).add_children(&joints);

		// Horizontal anchors
		app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_xyz(0.0, 5.0, 0.0),
			GlobalTransform::default(),
		));
		app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_xyz(4.0, 5.0, 0.0),
			GlobalTransform::default(),
		));

		app.update();

		// Middle joint should sag below the line
		let mid_pos = app.world().get::<Transform>(joints[2]).unwrap().translation;
		if mid_pos.y >= 5.0 {
			return Err(anyhow!("Middle joint should sag below horizontal line (y < 5.0)"));
		}

		Ok(())
	}

	#[test]
	fn motion_stability_with_zero_distance_anchors() -> Result<()> {
		let mut app = test_app();

		let joints: Vec<_> = (0..3)
			.map(|i| {
				app.world_mut()
					.spawn((Name::new(format!("chain-joint-{}", i)), Transform::default()))
					.id()
			})
			.collect();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 3, rest_length_m: 3.0, joint_entities: joints.clone() },
				ChainMotion::default(),
				Transform::default(),
				Visibility::default(),
			))
			.id();

		app.world_mut().entity_mut(root).add_children(&joints);

		// Both anchors at the same position (collapsed chain)
		let pos = Vec3::new(2.0, 3.0, 1.0);
		app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::from_translation(pos),
			GlobalTransform::from_translation(pos),
		));
		app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::from_translation(pos),
			GlobalTransform::from_translation(pos),
		));

		app.update();

		// All joints should collapse to the anchor position
		for joint in joints {
			let j_pos = app.world().get::<Transform>(joint).unwrap().translation;
			if (j_pos - pos).length() > 1e-4 {
				return Err(anyhow!(
					"Joint should collapse to anchor position when distance is zero"
				));
			}
		}

		Ok(())
	}

	/// Regression test for #1619 comment 6986265628: non-chain projectile contacts
	/// must not be re-emitted with fresh message IDs every frame.
	///
	/// Before the fix, ordinary projectile hits were drained and rewritten every update,
	/// causing the same contact to appear with 256+ distinct message IDs while the
	/// projectile remained alive, applying damage repeatedly (1000→975→950→925 HP).
	#[test]
	fn non_chain_projectile_contact_applies_once() -> Result<()> {
		let mut app = test_app();
		install_persistent_collectors(&mut app);

		// Boss entity with Health (not a chain capsule)
		let boss = app
			.world_mut()
			.spawn((
				Name::new("boss"),
				Health::from_max(1000.0),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Projectile entity (mock, just needs to exist)
		let projectile = app
			.world_mut()
			.spawn((
				Name::new("projectile"),
				projectiles::Flight::spawn(Vec3::ZERO, 100.0, 100.0, 10.0),
				HitPayload { amount: 25.0 },
			))
			.id();

		// Write one ProjectileContact (ordinary, non-chain hit)
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::PLACEHOLDER),
			target: boss,
			point: Vec3::ZERO,
			normal: Vec3::Y,
		});

		for _ in 0..15 {
			app.update();
		}

		let ids_before_despawn = app.world().resource::<CollectedContactIds>().ids.len();
		app.world_mut().entity_mut(projectile).despawn();
		for _ in 0..5 {
			app.update();
		}

		let applies: Vec<_> = app
			.world()
			.resource::<CollectedDamage>()
			.events
			.iter()
			.copied()
			.filter(|event| event.target == boss)
			.collect();
		if applies.len() != 1 {
			return Err(anyhow!(
				"Non-chain projectile contact was applied {} times instead of once: {applies:?}",
				applies.len()
			));
		}

		let ids_after_despawn = app.world().resource::<CollectedContactIds>().ids.len();
		if ids_after_despawn != ids_before_despawn {
			return Err(anyhow!(
				"Ordinary shot emitted fresh contact IDs after later updates (before despawn {ids_before_despawn}, after {ids_after_despawn})"
			));
		}
		if ids_before_despawn != 1 {
			return Err(anyhow!(
				"Ordinary shot should keep a single contact ID, got {ids_before_despawn}"
			));
		}

		let health = app.world().get::<Health>(boss).unwrap();
		let expected = 1000.0 - 25.0;
		if (health.current - expected).abs() > 0.1 {
			return Err(anyhow!(
				"Boss health should be {} (1000 - 25), got {}",
				expected,
				health.current
			));
		}

		Ok(())
	}

	/// Regression test for #1619: projectile crossing multiple capsules of the same chain
	/// should coalesce to one damage apply per interaction.
	#[test]
	fn chain_projectile_crosses_multiple_capsules_coalesces() -> Result<()> {
		let mut app = test_app();
		install_persistent_collectors(&mut app);

		// Chain root with Health
		let chain_root = app
			.world_mut()
			.spawn((
				Name::new("chain-root"),
				Health::from_max(100.0),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Two adjacent capsules on the same chain
		let capsule_a = app
			.world_mut()
			.spawn((
				Name::new("capsule-a"),
				ChainLinkId { link_id: 0, chain_root },
				ChainSegment,
				ChildOf(chain_root),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		let capsule_b = app
			.world_mut()
			.spawn((
				Name::new("capsule-b"),
				ChainLinkId { link_id: 1, chain_root },
				ChainSegment,
				ChildOf(chain_root),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Projectile entity
		let projectile = app
			.world_mut()
			.spawn((
				Name::new("projectile"),
				projectiles::Flight::spawn(Vec3::ZERO, 100.0, 100.0, 10.0),
				HitPayload { amount: 10.0 },
			))
			.id();

		// Projectile hits both capsules (same frame, multiple contacts)
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::PLACEHOLDER),
			target: capsule_a,
			point: Vec3::new(0.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::PLACEHOLDER),
			target: capsule_b,
			point: Vec3::new(0.1, 0.0, 0.0),
			normal: Vec3::Y,
		});

		for _ in 0..10 {
			app.update();
		}

		let applies: Vec<_> = app
			.world()
			.resource::<CollectedDamage>()
			.events
			.iter()
			.copied()
			.filter(|event| event.target == chain_root)
			.collect();
		if applies.len() != 1 {
			return Err(anyhow!(
				"Chain contacts should coalesce to 1 damage apply, got {}: {applies:?}",
				applies.len()
			));
		}

		let health = app.world().get::<Health>(chain_root).unwrap();
		let expected = 100.0 - 10.0;
		if (health.current - expected).abs() > 0.1 {
			return Err(anyhow!(
				"Chain root health should be {} (100 - 10), got {}",
				expected,
				health.current
			));
		}

		Ok(())
	}
}
