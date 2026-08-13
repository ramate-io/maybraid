//! Render-scene vegetation [`Material`] handles for CLI [`RenderSubject`] rebuilds.
//!
//! LOD vegetation components use [`crate::chico_material_lib::ChicoMaterialRefPlugin`] instead.

use bevy::prelude::*;

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
	_conifer_stick: &Handle<ChicoStickMaterial>,
	_leaf: &Handle<ChicoLeafMaterial>,
	tuft: &Handle<StandardMaterial>,
) {
	match subject {
		RenderSubject::SopesBanyan(_tree) => {}
		RenderSubject::LiamsConifer(_tree) => {}
		RenderSubject::FriendsConifer(_) => {}
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
		RenderSubject::BraidGrass(_) => {}
		RenderSubject::TropicalTufts(_) => {}
		RenderSubject::CommonTufts(_) => {}
		RenderSubject::BushScrub(_) => {}
		RenderSubject::TropicalUndergrowth(_) => {}
		RenderSubject::TropicalThicket(_) => {}
		RenderSubject::JerrysChaparral(_) => {}
		RenderSubject::LevantineScrub(_) => {}
		RenderSubject::TallGrass(_) => {}
		RenderSubject::WildGrass(_) => {}
		RenderSubject::MonsterGrass(_) => {}
		RenderSubject::RiverineGreen(_) => {}
		RenderSubject::LowBush(_) => {}
		RenderSubject::HighBush(_) => {}
		RenderSubject::SpottyBushes(_) => {}
		RenderSubject::UnendingJungle(_) => {}
		RenderSubject::JungleLowerMassives(_) => {}
		RenderSubject::JungleMassives(_) => {}
		RenderSubject::TemperateLowerMassives(_) => {}
		RenderSubject::PalmShade(_) => {}
		RenderSubject::RiparianMix(_) => {}
		RenderSubject::Alpine(_) => {}
		RenderSubject::Dryland(_) => {}
		RenderSubject::Storytellers(_) => {}
		RenderSubject::TradeWinds(_) => {}
		RenderSubject::WanderingAcacia(_) => {}
		RenderSubject::Leeward(_) => {}
		RenderSubject::ChristmasTaiga(_) => {}
		RenderSubject::ConiferMassives(_) => {}
		RenderSubject::TemperateMassives(_) => {}
		RenderSubject::RiparianGeneral(_) => {}
		RenderSubject::RollingOaks(_) => {}
		RenderSubject::ForlornSavanna(_) => {}
		RenderSubject::Orchard(_) => {}
		RenderSubject::Vineyard(_) => {}
		RenderSubject::DateGrove(_) => {}
		RenderSubject::StrangeOasis(_) => {}
		RenderSubject::Shamanhome(_) => {}
		RenderSubject::GoettingenFollow(_) => {}
		RenderSubject::ConiferSapling(_) => {}
		RenderSubject::AridConiferSapling(_) => {}
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
