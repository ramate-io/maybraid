//! Render-scene vegetation [`Material`] handles (embedded WGSL from `chico-vegetation-shaders`).

use bevy::prelude::*;

use chico_vegetation_components::{
	VegetationFoliageAssetRoot, VegetationFrondAssetRoot, VegetationProceduralAssets,
};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};

use crate::render::{RenderConfig, RenderSubject};

/// Stable bark / foliage materials reused whenever [`RenderConfig::subject`] is rebuilt from CLI defaults.
#[derive(Resource, Clone)]
pub struct RenderMaterials {
	pub stick: Handle<ChicoStickMaterial>,
	pub conifer_stick: Handle<ChicoStickMaterial>,
	pub leaf: Handle<ChicoLeafMaterial>,
	pub tuft: Handle<StandardMaterial>,
}

fn render_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.13, 0.085, 0.055, 1.0) }
}

fn render_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
}

fn render_conifer_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.18, 0.14, 0.10, 1.0) }
}

fn render_tuft_standard_material() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgb(0.22, 0.62, 0.28),
		double_sided: true,
		..Default::default()
	}
}

/// Replace foliage [`StandardMaterial`] with [`ChicoLeafMaterial`] (discard silhouette).
///
/// Covers:
/// - procedural placeholder (`VegetationProceduralAssets::foliage_material`)
/// - GLB meshes under [`VegetationFoliageAssetRoot`] (layered ball, etc.), once the
///   scene instance has spawned mesh children
///
/// Frond kits ([`VegetationFrondAssetRoot`]) are handled by
/// [`patch_vegetation_frond_solid_material`] (solid green, not the leaf shader).
pub fn patch_vegetation_foliage_leaf_material(
	mut commands: Commands,
	mats: Res<RenderMaterials>,
	procedural: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
	foliage_roots: Query<Entity, With<VegetationFoliageAssetRoot>>,
	children: Query<&Children>,
	glb_meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
	let placeholder = VegetationProceduralAssets::foliage_material();
	let leaf = mats.leaf.clone();

	for (entity, mesh_mat) in &procedural {
		if mesh_mat.id() != placeholder.id() {
			continue;
		}
		commands
			.entity(entity)
			.remove::<MeshMaterial3d<StandardMaterial>>()
			.insert(MeshMaterial3d(leaf.clone()));
	}

	for root in &foliage_roots {
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			if glb_meshes.contains(entity) {
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(leaf.clone()));
			}
			if let Ok(kids) = children.get(entity) {
				stack.extend(kids.iter());
			}
		}
	}
}

/// Keep frond GLBs on solid green [`StandardMaterial`] (`mats.tuft`).
///
/// Matches frond primitives under [`VegetationFrondAssetRoot`] (straight frond segments).
pub fn patch_vegetation_frond_solid_material(
	mut commands: Commands,
	mats: Res<RenderMaterials>,
	frond_roots: Query<Entity, With<VegetationFrondAssetRoot>>,
	children: Query<&Children>,
	glb_meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
	let tuft = mats.tuft.clone();
	for root in &frond_roots {
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			if glb_meshes.contains(entity) {
				commands
					.entity(entity)
					.insert(MeshMaterial3d(tuft.clone()));
			}
			if let Ok(kids) = children.get(entity) {
				stack.extend(kids.iter());
			}
		}
	}
}

pub fn setup_render_materials(
	mut commands: Commands,
	mut stick_assets: ResMut<Assets<ChicoStickMaterial>>,
	mut leaf_assets: ResMut<Assets<ChicoLeafMaterial>>,
	mut standard_assets: ResMut<Assets<StandardMaterial>>,
	mut config: ResMut<RenderConfig>,
) {
	let stick = stick_assets.add(render_stick_colors());
	let conifer_stick = stick_assets.add(render_conifer_stick_colors());
	let leaf = leaf_assets.add(render_leaf_colors());
	let tuft = standard_assets.add(render_tuft_standard_material());

	commands.insert_resource(RenderMaterials {
		stick: stick.clone(),
		conifer_stick: conifer_stick.clone(),
		leaf: leaf.clone(),
		tuft: tuft.clone(),
	});

	attach_render_materials(
		&mut config.subject,
		&stick,
		&conifer_stick,
		&leaf,
		&tuft,
	);
}

fn attach_render_materials(
	subject: &mut RenderSubject,
	stick: &Handle<ChicoStickMaterial>,
	conifer_stick: &Handle<ChicoStickMaterial>,
	leaf: &Handle<ChicoLeafMaterial>,
	tuft: &Handle<StandardMaterial>,
) {
	match subject {
		RenderSubject::SopesBanyan(_tree) => {}
		RenderSubject::LiamsConifer(_tree) => {}
		RenderSubject::FriendsConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::NorthernConifer(_tree) => {}
		RenderSubject::TemperateConifer(_tree) => {}
		RenderSubject::DatePalm(_tree) => {}
		RenderSubject::WaialeaPalm(_tree) => {}
		RenderSubject::PalmBush(_tree) => {}
		RenderSubject::StorybookTree(_tree) => {}
		RenderSubject::PenmarchTorch(_tree) => {}
		RenderSubject::KamakuraTorch(_tree) => {}
		RenderSubject::RorysHeadTrained(_tree) => {}
		RenderSubject::BraidOakTree(_) => {}
		RenderSubject::VaseTree(_) => {}
		RenderSubject::JungleStorybookTree(_) => {}
		RenderSubject::HonuBanyan(_) => {}
		RenderSubject::SucculentTuft(t) => {
			t.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::BladeTuft(t) => {
			t.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TuftPatch(_t) => {}
		RenderSubject::BraidGrass(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TropicalTufts(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::CommonTufts(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::BushScrub(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TropicalUndergrowth(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TropicalThicket(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::JerrysChaparral(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::LevantineScrub(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TallGrass(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::WildGrass(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::MonsterGrass(_) => {}
		RenderSubject::RiverineGreen(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::LowBush(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::HighBush(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::SpottyBushes(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::UnendingJungle(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::JungleLowerMassives(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::JungleMassives(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TemperateLowerMassives(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::PalmShade(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::RiparianMix(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Alpine(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Dryland(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Storytellers(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TradeWinds(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::WanderingAcacia(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Leeward(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::ChristmasTaiga(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::ConiferMassives(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::TemperateMassives(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::RiparianGeneral(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::RollingOaks(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::ForlornSavanna(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Orchard(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Vineyard(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::DateGrove(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::StrangeOasis(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::Shamanhome(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::GoettingenFollow(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::ConiferSapling(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::AridConiferSapling(g) => {
			g.stick_material.mesh = MeshMaterial3d(stick.clone());
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::SpearTuft(t) => {
			t.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::BuddhaHandTuft(t) => {
			t.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::WeepingTuft(t) => {
			t.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::JungleGrowth(g) => {
			g.body_material.mesh = MeshMaterial3d(stick.clone());
			g.foliage_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::HighBushShoots(b) => {
			b.stick_material.mesh = MeshMaterial3d(stick.clone());
			b.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::FrondCrown(c) => {
			c.material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::ModerateLodFrondCrown(c) => {
			c.material.mesh = MeshMaterial3d(tuft.clone());
		}
	}
}

/// CLI parses [`SkippedLeafMeshMaterial`] defaults as empty handles; reattach curated materials before spawning.
pub fn sync_render_material_handles(mut config: ResMut<RenderConfig>, mats: Res<RenderMaterials>) {
	let stick = mats.stick.clone();
	let conifer_stick = mats.conifer_stick.clone();
	let leaf = mats.leaf.clone();
	let tuft = mats.tuft.clone();
	attach_render_materials(&mut config.subject, &stick, &conifer_stick, &leaf, &tuft);
}
