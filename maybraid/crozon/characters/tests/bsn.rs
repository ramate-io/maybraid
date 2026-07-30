//! Compile-time checks that every species exposes the data/visual/scene BSN layers.

use bevy::prelude::StandardMaterial;
use crozon_characters::species::{
	braidman::{bsn::Braidman, BraidmanConfig},
	brenal::{bsn::Brenal, BrenalConfig},
	brodler::{bsn::Brodler, BrodlerConfig},
	caole::{bsn::Caole, CaoleConfig},
	claber::{bsn::Claber, ClaberConfig},
	croconot::{bsn::Croconot, CroconotConfig},
	dui::{bsn::Dui, DuiConfig},
	hars::{bsn::Hars, HarsConfig},
	lero::{bsn::Lero, LeroConfig},
	mygr::{bsn::Mygr, MygrConfig},
	spibmom::{bsn::Spibmom, SpibmomConfig},
	wumbus::{bsn::Wumbus, WumbusConfig},
};

#[test]
fn braidman_bsn_scenes_build() {
	let config = BraidmanConfig::default();
	let _root = Braidman::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn brenal_bsn_scenes_build() {
	let config = BrenalConfig::default();
	let _root = Brenal::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn claber_bsn_scenes_build() {
	let config = ClaberConfig::default();
	let _root = Claber::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn caole_bsn_scenes_build() {
	let config = CaoleConfig::default();
	let _root = Caole::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn hars_bsn_scenes_build() {
	let config = HarsConfig::default();
	let _root = Hars::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn croconot_bsn_scenes_build() {
	let config = CroconotConfig::default();
	let _root = Croconot::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn brodler_bsn_scenes_build() {
	let config = BrodlerConfig::default();
	let _root = Brodler::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn mygr_bsn_scenes_build() {
	let config = MygrConfig::default();
	let _root = Mygr::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn dui_bsn_scenes_build() {
	let config = DuiConfig::default();
	let _root = Dui::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn wumbus_bsn_scenes_build() {
	let config = WumbusConfig::default();
	let _root = Wumbus::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn lero_bsn_scenes_build() {
	let config = LeroConfig::default();
	let _root = Lero::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}

#[test]
fn spibmom_bsn_scenes_build() {
	let config = SpibmomConfig::default();
	let _root = Spibmom::from_config(&config);
	let _data = config.data_scene();
	let _visual = config.visual_scene::<StandardMaterial>();
	let _scene = config.scene::<StandardMaterial>();
}
