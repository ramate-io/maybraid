//! Spawn the authored cast after composed height and a terrain trimesh exist.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore, TerrainTrimeshCollider};
use mob_characters::VEGETATION_POI;
use mob_intelligence::MemberOf;
use poi_intelligence::{GlobalPoi, Poi, PoiId};
use routing_intelligence::RoutingIntelligenceUser;

use crate::catalog::{scene_for, PlaygroundCast};
use crate::commands::{RequestBoth, RequestHerd, RequestPack, RequestRebuild};
use crate::playground_player::terrain_collider_ready;
use crate::ui;
use crate::TerrainPresentationDirty;

const FORAGE_COUNT: usize = 8;
const FORAGE_RADIUS: f32 = 90.0;
const FORAGE_ARRIVAL: f32 = 10.0;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaygroundState {
	pub cast: PlaygroundCast,
	pub pois_ready: bool,
	pub mobs_ready: bool,
}

impl Default for PlaygroundState {
	fn default() -> Self {
		Self { cast: PlaygroundCast::Herd, pois_ready: false, mobs_ready: false }
	}
}

/// Authored semantic host spawned by this playground.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlaygroundMobHost;

/// Vegetation POI the herd/pack can journey toward.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlaygroundForagePoi;

pub fn apply_cast_commands(
	mut commands: Commands,
	mut state: ResMut<PlaygroundState>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	hosts: Query<Entity, With<PlaygroundMobHost>>,
	herd: Query<Entity, With<RequestHerd>>,
	pack: Query<Entity, With<RequestPack>>,
	both: Query<Entity, With<RequestBoth>>,
) {
	let mut next = None;
	for entity in &herd {
		next = Some(PlaygroundCast::Herd);
		commands.entity(entity).despawn();
	}
	for entity in &pack {
		next = Some(PlaygroundCast::Pack);
		commands.entity(entity).despawn();
	}
	for entity in &both {
		next = Some(PlaygroundCast::Both);
		commands.entity(entity).despawn();
	}
	let Some(cast) = next else {
		return;
	};
	despawn_hosts(&mut commands, &hosts);
	state.cast = cast;
	state.mobs_ready = false;
	ui::write_status(&mut status, format!("{}: waiting for surface", cast.label()));
}

pub fn apply_rebuild_command(
	mut commands: Commands,
	mut state: ResMut<PlaygroundState>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	hosts: Query<Entity, With<PlaygroundMobHost>>,
	pois: Query<Entity, With<PlaygroundForagePoi>>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	if rebuild.is_empty() {
		return;
	}
	for entity in &rebuild {
		commands.entity(entity).despawn();
	}
	despawn_hosts(&mut commands, &hosts);
	for entity in &pois {
		commands.entity(entity).despawn();
	}
	state.pois_ready = false;
	state.mobs_ready = false;
	dirty.0 = true;
	ui::write_status(&mut status, "rebuild: regenerating terrain");
}

pub fn spawn_forage_pois(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut state: ResMut<PlaygroundState>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
) {
	if state.pois_ready {
		return;
	}
	let center = layout.region_center_xz();
	let mut points = Vec::with_capacity(FORAGE_COUNT);
	for index in 0..FORAGE_COUNT {
		let angle = index as f32 / FORAGE_COUNT as f32 * std::f32::consts::TAU;
		let xz = Vec2::new(
			center.x + angle.cos() * FORAGE_RADIUS,
			center.z + angle.sin() * FORAGE_RADIUS,
		);
		let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
			return;
		};
		points.push((xz, elevation));
	}

	let mesh = meshes.add(Sphere::new(1.2));
	let material = materials.add(Color::srgb(0.95, 0.82, 0.25));
	for (index, (xz, elevation)) in points.into_iter().enumerate() {
		commands.spawn((
			Name::new(format!("forage-{index}")),
			PlaygroundForagePoi,
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material.clone()),
			Transform::from_xyz(xz.x, elevation + 1.2, xz.y),
			Poi::new(PoiId(index as u64 + 1), VEGETATION_POI)
				.with_arrival_radius(FORAGE_ARRIVAL)
				.with_salience(1.0),
			GlobalPoi,
			Visibility::default(),
		));
	}
	state.pois_ready = true;
	info!("spawned {FORAGE_COUNT} forage POIs on composed height");
}

pub fn spawn_playground_mobs(
	mut commands: Commands,
	mut state: ResMut<PlaygroundState>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	terrain_roots: Query<Entity, With<TerrainTrimeshCollider>>,
	children: Query<&Children>,
	colliders: Query<(), With<Collider>>,
) {
	if state.mobs_ready {
		return;
	}
	if !state.pois_ready {
		return;
	}
	if !terrain_collider_ready(&terrain_roots, &children, &colliders) {
		return;
	}

	let center = layout.region_center_xz();
	let mut planned = Vec::new();
	for (kind, offset) in state.cast.placements() {
		let xz = Vec2::new(center.x + offset.x, center.z + offset.y);
		let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
			return;
		};
		planned.push((*kind, xz, elevation));
	}

	for (kind, xz, elevation) in planned {
		let transform = Transform::from_xyz(xz.x, elevation, xz.y);
		let scene = scene_for(kind);
		let members = scene.mob.roster.members.len();
		let host = scene.spawn(&mut commands, transform);
		commands.entity(host).insert(PlaygroundMobHost);
		info!(
			"spawned {:?} members={members} at ({:.1}, {:.1}, {:.1})",
			kind, xz.x, elevation, xz.y
		);
	}
	state.mobs_ready = true;
}

pub fn draw_debug_gizmos(
	mut gizmos: Gizmos,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	hosts: Query<(&maybraid_mobs::MobScene, &Transform, Option<&RoutingIntelligenceUser>)>,
	plants: Query<&GlobalTransform, With<MemberOf>>,
) {
	let hop_colors =
		[Color::srgb(1.0, 0.55, 0.15), Color::srgb(0.95, 0.85, 0.2), Color::srgb(0.25, 0.85, 1.0)];
	for (scene, transform, routing) in &hosts {
		let at = transform.translation;
		let color = kind_color(scene.mob.kind);
		gizmos.line(at, at + Vec3::Y * 18.0, color);
		gizmos.sphere(Isometry3d::from_translation(at + Vec3::Y * 18.0), 1.4, color);
		xz_ring(&mut gizmos, at, scene.mob.intelligence.leash, color.with_alpha(0.55));

		if let Some(terrain_y) = store.composed_height_at(&layout, at.x, at.z) {
			let ground = Vec3::new(at.x, terrain_y, at.z);
			let delta = (at.y - terrain_y).abs();
			let ground_color =
				if delta > 2.0 { Color::srgb(1.0, 0.2, 0.85) } else { Color::srgb(0.4, 0.95, 0.7) };
			gizmos.line(at, ground, ground_color);
			gizmos.sphere(Isometry3d::from_translation(ground), 0.7, ground_color);
		}

		if let Some(routing) = routing {
			if let Some(goal) = routing.destination {
				gizmos.sphere(
					Isometry3d::from_translation(goal + Vec3::Y),
					0.8,
					Color::srgb(1.0, 1.0, 1.0),
				);
			}
			for (index, layer) in routing.plan.layers.iter().enumerate() {
				let hop_color = hop_colors[index.min(hop_colors.len() - 1)];
				let lifted: Vec<Vec3> =
					layer.waypoints.iter().map(|point| *point + Vec3::Y * 1.5).collect();
				if lifted.len() >= 2 {
					gizmos.linestrip(lifted.iter().copied(), hop_color);
				}
				for point in &lifted {
					gizmos.sphere(Isometry3d::from_translation(*point), 0.28, hop_color);
				}
			}
		}
	}

	for plant in &plants {
		let at = plant.translation();
		gizmos.sphere(
			Isometry3d::from_translation(at + Vec3::Y * 0.4),
			0.35,
			Color::srgb(0.2, 0.95, 1.0),
		);
	}
}

fn despawn_hosts(commands: &mut Commands, hosts: &Query<Entity, With<PlaygroundMobHost>>) {
	for entity in hosts {
		commands.entity(entity).despawn();
	}
}

fn kind_color(kind: maybraid_mobs::MobKind) -> Color {
	match kind {
		maybraid_mobs::MobKind::Herd => Color::srgb(0.35, 0.9, 0.45),
		maybraid_mobs::MobKind::Pack => Color::srgb(0.95, 0.55, 0.2),
		maybraid_mobs::MobKind::Raider => Color::srgb(1.0, 0.32, 0.28),
		maybraid_mobs::MobKind::Guard => Color::srgb(0.35, 0.55, 1.0),
		maybraid_mobs::MobKind::Pleb => Color::srgb(0.95, 0.85, 0.3),
		maybraid_mobs::MobKind::Rambles => Color::srgb(0.4, 0.85, 1.0),
		maybraid_mobs::MobKind::Brawler => Color::srgb(0.95, 0.4, 0.85),
	}
}

fn xz_ring(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color) {
	let mut points = Vec::with_capacity(49);
	for index in 0..=48 {
		let angle = index as f32 / 48.0 * std::f32::consts::TAU;
		points.push(Vec3::new(
			center.x + angle.cos() * radius,
			center.y + 0.4,
			center.z + angle.sin() * radius,
		));
	}
	gizmos.linestrip(points, color);
}
