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
