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

		// Spawn chain root with joints in Taut mode
		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 3, rest_length_m: 3.0 },
				ChainMotion { mode: ChainTensionMode::Taut, active: true },
				Transform::default(),
				Visibility::default(),
			))
			.id();

		let joint0 =
			app.world_mut().spawn((Name::new("chain-joint-0"), Transform::default())).id();
		let joint1 =
			app.world_mut().spawn((Name::new("chain-joint-1"), Transform::default())).id();
		let joint2 =
			app.world_mut().spawn((Name::new("chain-joint-2"), Transform::default())).id();

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

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 2, rest_length_m: 2.0 },
				ChainMotion { mode: ChainTensionMode::Taut, active: false },
				Transform::default(),
				Visibility::default(),
			))
			.id();

		let joint0 = app
			.world_mut()
			.spawn((Name::new("chain-joint-0"), Transform::from_xyz(0.0, 0.0, 0.0)))
			.id();
		let joint1 = app
			.world_mut()
			.spawn((Name::new("chain-joint-1"), Transform::from_xyz(0.0, 2.0, 0.0)))
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

		// Create chain root
		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count, rest_length_m: length_m },
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
			))
			.id();

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

		// Create joints at origin
		let mut joints = Vec::new();
		for _i in 0..link_count {
			let joint = app
				.world_mut()
				.spawn((Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default()))
				.id();
			app.world_mut().entity_mut(root).add_child(joint);
			joints.push(joint);
		}

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

		// Spawn chain root with health
		let root = app
			.world_mut()
			.spawn((
				Name::new("test-chain"),
				ChainRoot { link_count, rest_length_m: length_m },
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				Health { current: initial_health, max: initial_health },
			))
			.id();

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
			app.world_mut().entity_mut(root).add_child(joint);
			joints.push(joint);
		}

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
	fn released_chain_deactivates_motion() -> Result<()> {
		let mut app = test_app();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 2, rest_length_m: 2.0 },
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
	fn catenary_mode_adds_sag() -> Result<()> {
		let mut app = test_app();

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 5, rest_length_m: 4.0 },
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 10 },
					active: true,
				},
				Transform::default(),
				Visibility::default(),
			))
			.id();

		let joints: Vec<_> = (0..5)
			.map(|i| {
				app.world_mut()
					.spawn((Name::new(format!("chain-joint-{}", i)), Transform::default()))
					.id()
			})
			.collect();

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

		let root = app
			.world_mut()
			.spawn((
				ChainRoot { link_count: 3, rest_length_m: 3.0 },
				ChainMotion::default(),
				Transform::default(),
				Visibility::default(),
			))
			.id();

		let joints: Vec<_> = (0..3)
			.map(|i| {
				app.world_mut()
					.spawn((Name::new(format!("chain-joint-{}", i)), Transform::default()))
					.id()
			})
			.collect();

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
