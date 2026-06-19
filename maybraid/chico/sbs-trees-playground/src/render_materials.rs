//! Render-scene vegetation [`Material`] handles (embedded WGSL from `chico-vegetation-shaders`).

use bevy::prelude::*;

use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};

use crate::render::{
	RenderBraidOakTree, RenderConfig, RenderHonuBanyan, RenderJungleStorybookTree, RenderSubject,
	RenderVaseTree,
};

/// Stable bark / foliage materials reused whenever [`RenderConfig::subject`] is rebuilt from CLI defaults.
#[derive(Resource, Clone)]
pub struct RenderMaterials {
	pub stick: Handle<ChicoStickMaterial>,
	pub conifer_stick: Handle<ChicoStickMaterial>,
	pub leaf: Handle<ChicoLeafMaterial>,
	pub northern_leaf: Handle<ChicoLeafMaterial>,
	pub jungle_inner_leaf: Handle<ChicoLeafMaterial>,
	pub jungle_outer_leaf: Handle<ChicoLeafMaterial>,
	pub braid_inner_leaf: Handle<ChicoLeafMaterial>,
	pub braid_outer_leaf: Handle<ChicoLeafMaterial>,
	pub jungle_stick: Handle<ChicoStickMaterial>,
	pub tuft: Handle<StandardMaterial>,
}

fn render_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.13, 0.085, 0.055, 1.0) }
}

fn render_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
}

fn render_northern_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.14, 0.38, 0.34, 1.0) }
}

fn render_jungle_inner_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.12, 0.28, 0.16, 1.0) }
}

fn render_jungle_outer_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.18, 0.58, 0.32, 1.0) }
}

fn render_braid_inner_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.14, 0.32, 0.18, 1.0) }
}

fn render_braid_outer_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.20, 0.52, 0.28, 1.0) }
}

fn render_jungle_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.09, 0.06, 0.04, 1.0) }
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
	let northern_leaf = leaf_assets.add(render_northern_leaf_colors());
	let jungle_inner_leaf = leaf_assets.add(render_jungle_inner_leaf_colors());
	let jungle_outer_leaf = leaf_assets.add(render_jungle_outer_leaf_colors());
	let braid_inner_leaf = leaf_assets.add(render_braid_inner_leaf_colors());
	let braid_outer_leaf = leaf_assets.add(render_braid_outer_leaf_colors());
	let jungle_stick = stick_assets.add(render_jungle_stick_colors());
	let tuft = standard_assets.add(render_tuft_standard_material());

	commands.insert_resource(RenderMaterials {
		stick: stick.clone(),
		conifer_stick: conifer_stick.clone(),
		leaf: leaf.clone(),
		northern_leaf: northern_leaf.clone(),
		jungle_inner_leaf: jungle_inner_leaf.clone(),
		jungle_outer_leaf: jungle_outer_leaf.clone(),
		braid_inner_leaf: braid_inner_leaf.clone(),
		braid_outer_leaf: braid_outer_leaf.clone(),
		jungle_stick: jungle_stick.clone(),
		tuft: tuft.clone(),
	});

	let mats_snapshot = RenderMaterials {
		stick: stick.clone(),
		conifer_stick: conifer_stick.clone(),
		leaf: leaf.clone(),
		northern_leaf: northern_leaf.clone(),
		jungle_inner_leaf: jungle_inner_leaf.clone(),
		jungle_outer_leaf: jungle_outer_leaf.clone(),
		braid_inner_leaf: braid_inner_leaf.clone(),
		braid_outer_leaf: braid_outer_leaf.clone(),
		jungle_stick: jungle_stick.clone(),
		tuft: tuft.clone(),
	};
	attach_render_materials(
		&mut config.subject,
		&stick,
		&conifer_stick,
		&leaf,
		&northern_leaf,
		&tuft,
	);
	if let RenderSubject::JungleStorybookTree(tree) = &mut config.subject {
		attach_jungle_storybook_materials(tree, &mats_snapshot);
	}
	if let RenderSubject::HonuBanyan(tree) = &mut config.subject {
		attach_honu_banyan_materials(tree, &mats_snapshot);
	}
	if let RenderSubject::TropicalThicket(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats_snapshot);
	}
	if let RenderSubject::UnendingJungle(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats_snapshot);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats_snapshot.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh =
			MeshMaterial3d(mats_snapshot.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats_snapshot);
	}
	if let RenderSubject::JungleLowerMassives(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats_snapshot);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats_snapshot.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh =
			MeshMaterial3d(mats_snapshot.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats_snapshot);
	}
	if let RenderSubject::JungleMassives(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats_snapshot);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats_snapshot.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh =
			MeshMaterial3d(mats_snapshot.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats_snapshot);
	}
	if let RenderSubject::Shamanhome(grove) = &mut config.subject {
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats_snapshot.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh =
			MeshMaterial3d(mats_snapshot.jungle_inner_leaf.clone());
	}
	if let RenderSubject::BraidOakTree(tree) = &mut config.subject {
		attach_braid_oak_materials(tree, &mats_snapshot);
	}
	if let RenderSubject::VaseTree(tree) = &mut config.subject {
		attach_vase_tree_materials(tree, &mats_snapshot);
	}
}

pub fn attach_vase_tree_materials(tree: &mut RenderVaseTree, mats: &RenderMaterials) {
	tree.stick_material.mesh = MeshMaterial3d(mats.stick.clone());
	tree.inner_leaf_material.mesh = MeshMaterial3d(mats.braid_inner_leaf.clone());
	tree.outer_leaf_material.mesh = MeshMaterial3d(mats.braid_outer_leaf.clone());
}

pub fn attach_braid_oak_materials(tree: &mut RenderBraidOakTree, mats: &RenderMaterials) {
	tree.stick_material.mesh = MeshMaterial3d(mats.stick.clone());
	tree.inner_leaf_material.mesh = MeshMaterial3d(mats.braid_inner_leaf.clone());
	tree.outer_leaf_material.mesh = MeshMaterial3d(mats.braid_outer_leaf.clone());
}

pub fn attach_jungle_storybook_materials(
	tree: &mut RenderJungleStorybookTree,
	mats: &RenderMaterials,
) {
	tree.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
	tree.inner_leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
	tree.outer_leaf_material.mesh = MeshMaterial3d(mats.jungle_outer_leaf.clone());
	tree.growth_body_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
	tree.growth_foliage_material.mesh = MeshMaterial3d(mats.tuft.clone());
}

pub fn attach_honu_banyan_materials(tree: &mut RenderHonuBanyan, mats: &RenderMaterials) {
	tree.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
	tree.inner_leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
	tree.outer_leaf_material.mesh = MeshMaterial3d(mats.jungle_outer_leaf.clone());
	tree.growth_body_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
	tree.growth_foliage_material.mesh = MeshMaterial3d(mats.tuft.clone());
}

fn attach_render_materials(
	subject: &mut RenderSubject,
	stick: &Handle<ChicoStickMaterial>,
	conifer_stick: &Handle<ChicoStickMaterial>,
	leaf: &Handle<ChicoLeafMaterial>,
	northern_leaf: &Handle<ChicoLeafMaterial>,
	tuft: &Handle<StandardMaterial>,
) {
	match subject {
		RenderSubject::SopesBanyan(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::LiamsConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::FriendsConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::NorthernConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(northern_leaf.clone());
		}
		RenderSubject::TemperateConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::DatePalm(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::WaialeaPalm(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::PalmBush(tree) => {
			tree.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
		RenderSubject::StorybookTree(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::PenmarchTorch(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::KamakuraTorch(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
		RenderSubject::RorysHeadTrained(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick.clone());
			tree.leaf_material.mesh = MeshMaterial3d(leaf.clone());
		}
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
		RenderSubject::TuftPatch(t) => {
			t.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
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
		RenderSubject::MonsterGrass(g) => {
			g.leaf_material.mesh = MeshMaterial3d(tuft.clone());
		}
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
	let northern_leaf = mats.northern_leaf.clone();
	let tuft = mats.tuft.clone();
	attach_render_materials(
		&mut config.subject,
		&stick,
		&conifer_stick,
		&leaf,
		&northern_leaf,
		&tuft,
	);
	if let RenderSubject::JungleStorybookTree(tree) = &mut config.subject {
		attach_jungle_storybook_materials(tree, &mats);
	}
	if let RenderSubject::HonuBanyan(tree) = &mut config.subject {
		attach_honu_banyan_materials(tree, &mats);
	}
	if let RenderSubject::TropicalThicket(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats);
	}
	if let RenderSubject::UnendingJungle(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats);
	}
	if let RenderSubject::JungleLowerMassives(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats);
	}
	if let RenderSubject::JungleMassives(grove) = &mut config.subject {
		attach_honu_banyan_materials(&mut grove.honu_template, &mats);
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
		attach_jungle_storybook_materials(&mut grove.jungle_storybook_template, &mats);
	}
	if let RenderSubject::Shamanhome(grove) = &mut config.subject {
		grove.sope_template.stick_material.mesh = MeshMaterial3d(mats.jungle_stick.clone());
		grove.sope_template.leaf_material.mesh = MeshMaterial3d(mats.jungle_inner_leaf.clone());
	}
	if let RenderSubject::VaseTree(tree) = &mut config.subject {
		attach_vase_tree_materials(tree, &mats);
	}
	if let RenderSubject::BraidOakTree(tree) = &mut config.subject {
		attach_braid_oak_materials(tree, &mats);
	}
}
