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
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use projectiles::ProjectileContact;

	use crate::chains::{
		coalesce_chain_contacts, deactivate_released_chains, spawn_chain_colliders,
		sync_chain_colliders, update_chain_joints, BikanwardaChainPhysicsPlugin, ChainAnchor,
		ChainColliderConfig, ChainEndpoint, ChainLinkId, ChainMotion, ChainRoot,
		ChainTensionMode, ReleasedChain,
	};

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin))
			.init_resource::<crate::chains::ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_systems(
				PostUpdate,
				(
					update_chain_joints,
					sync_chain_colliders.after(update_chain_joints),
				)
					.after(TransformSystems::Propagate),
			)
			.add_systems(Update, (coalesce_chain_contacts, deactivate_released_chains));
		app.finish();
		// Run one update to initialize systems
		app.update();
		app
	}

	#[test]
	fn anchored_chain_follows_socket_transforms() -> Result<()> {
		let mut app = test_app();

		let joint0 =
			app.world_mut().spawn((Name::new("chain-joint-0"), Transform::default())).id();
		let joint1 =
			app.world_mut().spawn((Name::new("chain-joint-1"), Transform::default())).id();
		let joint2 =
			app.world_mut().spawn((Name::new("chain-joint-2"), Transform::default())).id();

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
				Visibility::default(),
			))
			.id();

		app.world_mut().entity_mut(root).add_children(&[joint0, joint1, joint2]);

		// Spawn anchor entities (only set Transform, let propagation handle GlobalTransform)
		let start_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
				Transform::from_xyz(0.0, 0.0, 0.0),
			))
			.id();

		let end_anchor = app
			.world_mut()
			.spawn((
				ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
				Transform::from_xyz(0.0, 3.0, 0.0),
			))
			.id();

		// Update to propagate transforms and run motion system
		app.update();

		// Check that joints are positioned along the chain
		let j0_pos = app.world().get::<Transform>(joint0).unwrap().translation;
		let j1_pos = app.world().get::<Transform>(joint1).unwrap().translation;
		let j2_pos = app.world().get::<Transform>(joint2).unwrap().translation;

		// With taut mode, joints should be evenly spaced along Y
		assert!((j0_pos.y - 0.0).abs() < 0.1, "Joint 0 should be near start");
		assert!((j1_pos.y - 1.5).abs() < 0.1, "Joint 1 should be at midpoint");
		assert!((j2_pos.y - 3.0).abs() < 0.1, "Joint 2 should be near end");

		// Move end anchor (just update Transform, let propagation handle GlobalTransform)
		if let Some(mut transform) = app.world_mut().get_mut::<Transform>(end_anchor) {
			transform.translation = Vec3::new(2.0, 4.0, 0.0);
		}

		app.update();

		// Joints should follow the new anchor position
		let new_j2_pos = app.world().get::<Transform>(joint2).unwrap().translation;
		assert!(
			(new_j2_pos - Vec3::new(2.0, 4.0, 0.0)).length() < 0.1,
			"End joint should follow anchor"
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
		let capsule0 = app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root: root }).id();
		let capsule1 = app.world_mut().spawn(ChainLinkId { link_id: 1, chain_root: root }).id();

		let projectile = Entity::from_bits(999);
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

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Should coalesce to one contact
		if contacts.len() != 1 {
			return Err(anyhow!("Expected 1 coalesced contact, got {}", contacts.len()));
		}

		assert_eq!(contacts[0].projectile, projectile);

		Ok(())
	}

	#[test]
	fn multiple_projectiles_from_same_source_do_not_coalesce() -> Result<()> {
		let mut app = test_app();

		let root = app.world_mut().spawn_empty().id();
		let capsule = app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root: root }).id();

		let projectile_a = Entity::from_bits(999);
		let projectile_b = Entity::from_bits(1000);
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

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Should keep both contacts (different projectiles, even if same source)
		if contacts.len() != 2 {
			return Err(anyhow!("Expected 2 contacts (distinct projectiles), got {}", contacts.len()));
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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
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

		let damage_frame = damage_frame
			.ok_or_else(|| anyhow!("No damage applied after 60 frames (1 second)"))?;

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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
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
		let positioned_colliders = colliders
			.iter()
			.filter(|&&e| app.world().get_entity(e).is_ok())
			.count();
		
		if positioned_colliders < 3 {
			return Err(anyhow!("Expected at least 3 positioned colliders, got {}", positioned_colliders));
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

		// Track health changes frame by frame
		let mut health_history = Vec::new();
		let mut projectile_alive = true;

		// Run for at least 10 frames to ensure the projectile passes through
		for frame in 0..15 {
			app.update();

			let current_health = app.world().entity(root).get::<Health>().map(|h| h.current).unwrap_or(0.0);
			health_history.push(current_health);

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
				ChainRoot {
					link_count: 2,
					rest_length_m: 2.0,
					joint_entities: Vec::new(),
				},
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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
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
		let collider_count = colliders.iter().filter(|&&e| app.world().get_entity(e).is_ok()).count();
		if collider_count != link_count as usize {
			return Err(anyhow!("Expected {} colliders, found {}", link_count, collider_count));
		}

		// Mark chain as released
		app.world_mut().entity_mut(root).insert(ReleasedChain);

		// Run update to retire colliders
		app.update();

		// Verify colliders are despawned
		let remaining_colliders = colliders.iter().filter(|&&e| app.world().get_entity(e).is_ok()).count();
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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Taut,
					active: true,
				},
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
			GlobalTransform::from(Transform::from_translation(end_translation).with_rotation(end_rotation)),
		));

		// Run several updates to let transforms propagate and motion system work
		for _ in 0..5 {
			app.update();
		}

		// Verify joints moved to anchor positions
		let first_joint_pos = app.world().get::<Transform>(joints[0]).unwrap().translation;
		let last_joint_pos = app.world().get::<Transform>(joints[link_count as usize - 1]).unwrap().translation;

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
				ChainRoot {
					link_count,
					rest_length_m: length_m,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 8 },
					active: true,
				},
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
		let last_pos = app.world().get::<Transform>(joints[link_count as usize - 1]).unwrap().translation;

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
				ChainRoot {
					link_count: 5,
					rest_length_m: 4.0,
					joint_entities: joints.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
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
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: joints.clone(),
				},
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
				return Err(anyhow!("Joint should collapse to anchor position when distance is zero"));
			}
		}

		Ok(())
	}
}
