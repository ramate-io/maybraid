//! Spawn the authored cast after composed height and a terrain trimesh exist.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore, TerrainTrimeshCollider};
use journeying_intelligence::JourneyingIntelligenceUser;
use meandering_intelligence::MeanderingIntelligenceUser;
use mob_characters::{LOCAL_POI, VEGETATION_POI};
use mob_intelligence::{MemberOf, Mob};
use npc_intelligence::NpcIntelligence;
use poi_intelligence::{GlobalPoi, LocalPoi, Poi, PoiId, PoiIntelligenceUser, PoiKind};
use routing_intelligence::RoutingIntelligenceUser;
use tether_intelligence::{StalkRadii, TetherIntelligenceUser, TetherObjective};

use crate::catalog::{playground_leash, scene_for_placement, PlaygroundCast, JOURNEY_TILE};
use crate::commands::{
	RequestBoth, RequestHars, RequestHarsYlter, RequestHerd, RequestPack, RequestRebuild,
	RequestYlter,
};
use crate::playground_player::terrain_collider_ready;
use crate::ui;
use crate::TerrainPresentationDirty;

const FORAGE_COUNT: usize = 8;
const FORAGE_RADIUS: f32 = 90.0;
const FORAGE_ARRIVAL: f32 = 10.0;
const LOCAL_ID_START: u64 = 101;
const LOCAL_HOST_COUNT: usize = 6;
const LOCAL_HOST_RING: f32 = 22.0;
const LOCAL_FORAGE_COUNT: usize = 2;
const LOCAL_FORAGE_RING: f32 = 12.0;
const LOCAL_KIND_COUNT: usize = 4;
const LOCAL_KIND_RING: f32 = 16.0;
const LOCAL_ARRIVAL: f32 = 3.0;
const TETHER_ADDED_FRAC: f32 = 0.25;
const MEANDER_FRAC: f32 = 0.55;

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

/// Nearby snack the plants can keep circulating through.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlaygroundLocalPoi;

pub fn apply_cast_commands(
	mut commands: Commands,
	mut state: ResMut<PlaygroundState>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	hosts: Query<Entity, With<PlaygroundMobHost>>,
	herd: Query<Entity, With<RequestHerd>>,
	pack: Query<Entity, With<RequestPack>>,
	both: Query<Entity, With<RequestBoth>>,
	hars: Query<Entity, With<RequestHars>>,
	ylter: Query<Entity, With<RequestYlter>>,
	hars_ylter: Query<Entity, With<RequestHarsYlter>>,
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
	for entity in &hars {
		next = Some(PlaygroundCast::Hars);
		commands.entity(entity).despawn();
	}
	for entity in &ylter {
		next = Some(PlaygroundCast::Ylter);
		commands.entity(entity).despawn();
	}
	for entity in &hars_ylter {
		next = Some(PlaygroundCast::HarsYlter);
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
	forage: Query<Entity, With<PlaygroundForagePoi>>,
	locals: Query<Entity, With<PlaygroundLocalPoi>>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	if rebuild.is_empty() {
		return;
	}
	for entity in &rebuild {
		commands.entity(entity).despawn();
	}
	despawn_hosts(&mut commands, &hosts);
	for entity in &forage {
		commands.entity(entity).despawn();
	}
	for entity in &locals {
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
	let mut forage = Vec::with_capacity(FORAGE_COUNT);
	for index in 0..FORAGE_COUNT {
		let xz = forage_xz(center, index);
		let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
			return;
		};
		forage.push((xz, elevation));
	}

	let mut locals = Vec::new();
	for placement in state.cast.placements() {
		let host = Vec2::new(center.x + placement.offset.x, center.z + placement.offset.y);
		for (poi_kind, xz) in host_local_snacks(host, playground_leash(placement.kind)) {
			let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
				return;
			};
			locals.push((poi_kind, xz, elevation));
		}
	}
	for index in 0..FORAGE_COUNT {
		let at = forage_xz(center, index);
		for xz in ring_points(at, LOCAL_FORAGE_RING, LOCAL_FORAGE_COUNT, 0.35) {
			let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
				return;
			};
			locals.push((VEGETATION_POI, xz, elevation));
		}
	}

	let forage_mesh = meshes.add(Sphere::new(1.2));
	let forage_mat = materials.add(Color::srgb(0.95, 0.82, 0.25));
	for (index, (xz, elevation)) in forage.into_iter().enumerate() {
		commands.spawn((
			Name::new(format!("forage-{index}")),
			PlaygroundForagePoi,
			Mesh3d(forage_mesh.clone()),
			MeshMaterial3d(forage_mat.clone()),
			Transform::from_xyz(xz.x, elevation + 1.2, xz.y),
			Poi::new(PoiId(index as u64 + 1), VEGETATION_POI)
				.with_arrival_radius(FORAGE_ARRIVAL)
				.with_salience(1.0),
			GlobalPoi,
			Visibility::default(),
		));
	}

	let local_mesh = meshes.add(Sphere::new(0.7));
	let veg_mat = materials.add(Color::srgb(0.35, 0.72, 0.95));
	let kind_mat = materials.add(Color::srgb(0.55, 0.95, 0.45));
	for (index, (kind, xz, elevation)) in locals.into_iter().enumerate() {
		let material = if kind == LOCAL_POI { kind_mat.clone() } else { veg_mat.clone() };
		commands.spawn((
			Name::new(format!("local-{index}")),
			PlaygroundLocalPoi,
			Mesh3d(local_mesh.clone()),
			MeshMaterial3d(material),
			Transform::from_xyz(xz.x, elevation + 0.8, xz.y),
			Poi::new(PoiId(LOCAL_ID_START + index as u64), kind)
				.with_arrival_radius(LOCAL_ARRIVAL)
				.with_salience(0.85),
			LocalPoi,
			Visibility::default(),
		));
	}
	state.pois_ready = true;
	info!("spawned {FORAGE_COUNT} forage POIs and nearby LocalPoi snacks");
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
	for placement in state.cast.placements() {
		let xz = Vec2::new(center.x + placement.offset.x, center.z + placement.offset.y);
		let Some(elevation) = store.composed_height_at(&layout, xz.x, xz.y) else {
			return;
		};
		planned.push((*placement, xz, elevation));
	}

	for (placement, xz, elevation) in planned {
		let transform = Transform::from_xyz(xz.x, elevation, xz.y);
		let scene = scene_for_placement(placement);
		let members = scene.mob.roster.members.len();
		let host = scene.spawn(&mut commands, transform);
		commands.entity(host).insert(PlaygroundMobHost);
		info!(
			"spawned {:?} species={:?} members={members} at ({:.1}, {:.1}, {:.1})",
			placement.kind, placement.species, xz.x, elevation, xz.y
		);
	}
	state.mobs_ready = true;
}

/// Production journeying defaults skip 256 m tiles; this patch is ~640 m.
pub fn tune_playground_journeying(
	mut hosts: Query<
		&mut JourneyingIntelligenceUser,
		(Added<JourneyingIntelligenceUser>, With<PlaygroundMobHost>),
	>,
) {
	for mut journey in &mut hosts {
		journey.tile_size = JOURNEY_TILE;
		journey.min_tile_distance = 1;
		journey.max_tile_distance = 6;
		journey.tile_probes = 16;
		journey.selection_interval = 0.2;
		journey.linger_secs = 6.0;
		journey.empty_tile_retry_secs = 4.0;
	}
}

/// Personality leashes stay at 24 m; copy the playground host leash onto members.
pub fn widen_playground_member_leashes(
	hosts: Query<&Mob, With<PlaygroundMobHost>>,
	mut members: Query<(
		&MemberOf,
		&mut TetherIntelligenceUser,
		&mut NpcIntelligence,
		Option<&mut MeanderingIntelligenceUser>,
		Option<&mut PoiIntelligenceUser>,
	)>,
) {
	for (membership, mut tether, mut npc, meandering, learner) in &mut members {
		let Ok(mob) = hosts.get(membership.mob) else {
			continue;
		};
		let leash = mob.leash;
		let objective = with_leash(tether.objective, leash);
		if tether.objective != objective {
			tether.objective = objective;
		}
		let added = (leash * TETHER_ADDED_FRAC).max(tether.added_radius);
		if (tether.added_radius - added).abs() > 1e-3 {
			tether.added_radius = added;
		}
		if npc.idle_tether != Some(objective) {
			npc.idle_tether = Some(objective);
		}
		let meander = (leash * MEANDER_FRAC).max(36.0);
		if let Some(mut meandering) = meandering {
			if meandering.radius < meander {
				meandering.radius = meander;
			}
		}
		if let Some(mut learner) = learner {
			if learner.policy.local_radius < meander {
				learner.policy.local_radius = meander;
			}
		}
	}
}

pub fn draw_debug_gizmos(
	mut gizmos: Gizmos,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	hosts: Query<(&maybraid_mobs::MobScene, &Transform, Option<&RoutingIntelligenceUser>)>,
	plants: Query<&GlobalTransform, With<MemberOf>>,
	locals: Query<&GlobalTransform, With<PlaygroundLocalPoi>>,
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

	for local in &locals {
		let at = local.translation();
		gizmos.sphere(Isometry3d::from_translation(at), 0.45, Color::srgb(0.35, 0.72, 0.95));
	}
}

fn forage_xz(center: Vec3, index: usize) -> Vec2 {
	let angle = index as f32 / FORAGE_COUNT as f32 * std::f32::consts::TAU;
	Vec2::new(center.x + angle.cos() * FORAGE_RADIUS, center.z + angle.sin() * FORAGE_RADIUS)
}

fn host_local_snacks(host: Vec2, leash: f32) -> Vec<(PoiKind, Vec2)> {
	let mut snacks = Vec::new();
	let veg_ring = LOCAL_HOST_RING.min(leash * 0.35);
	for xz in ring_points(host, veg_ring, LOCAL_HOST_COUNT, 0.0) {
		snacks.push((VEGETATION_POI, xz));
	}
	let kind_ring = LOCAL_KIND_RING.min(leash * 0.25);
	for xz in ring_points(host, kind_ring, LOCAL_KIND_COUNT, 0.2) {
		snacks.push((LOCAL_POI, xz));
	}
	snacks
}

fn ring_points(center: Vec2, radius: f32, count: usize, phase: f32) -> Vec<Vec2> {
	(0..count)
		.map(|index| {
			let angle = (index as f32 / count.max(1) as f32 + phase) * std::f32::consts::TAU;
			Vec2::new(center.x + angle.cos() * radius, center.y + angle.sin() * radius)
		})
		.collect()
}

fn with_leash(objective: TetherObjective, leash: f32) -> TetherObjective {
	match objective {
		TetherObjective::Tether(subject, _) => TetherObjective::Tether(subject, leash),
		TetherObjective::Stalk(subject, radii) => {
			TetherObjective::Stalk(subject, StalkRadii::new(radii.without(), leash))
		}
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn host_snacks_sit_inside_the_widened_meander() {
		let host = Vec2::new(-40.0, -20.0);
		let leash = playground_leash(maybraid_mobs::MobKind::Herd);
		let meander = (leash * MEANDER_FRAC).max(36.0);
		let snacks = host_local_snacks(host, leash);
		assert!(snacks.iter().any(|(kind, _)| *kind == VEGETATION_POI));
		assert!(snacks.iter().any(|(kind, _)| *kind == LOCAL_POI));
		for (_, xz) in snacks {
			assert!(xz.distance(host) <= meander + LOCAL_ARRIVAL);
		}
	}

	#[test]
	fn widened_grazer_tether_keeps_the_host_subject() {
		let host = Entity::PLACEHOLDER;
		let next = with_leash(TetherObjective::Tether(host, 24.0), 80.0);
		assert_eq!(next, TetherObjective::Tether(host, 80.0));
	}
}
