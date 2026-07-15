use character_ui_menu::{
	CameraFocus, LabelOption, ListValues, MenuComponent, MenuNode, SingleSelect,
};
use crozon_characters::{
	species::{
		braidman::BraidmanConfig, brenal::BrenalConfig, caole::CaoleConfig, hars::HarsConfig, claber::ClaberConfig, croconot::CroconotConfig, brodler::BrodlerConfig, dui::DuiConfig,
		chupri::ChupriConfig, lidder::LidderConfig, lero::LeroConfig, mygr::MygrConfig, spibmom::SpibmomConfig, sonyak::SonyakConfig, wumbus::WumbusConfig, ylter::YilterConfig,
	},
	ConceptAnimation,
};

use crate::{
	characters::{
		braidman::BraidmanMenu,
		brenal::{BrenalAnimationClip, BrenalMenu},
		caole::{CaoleAnimationClip, CaoleMenu},
		hars::{HarsAnimationClip, HarsMenu},
		claber::{ClaberAnimationClip, ClaberMenu},
		croconot::{CroconotAnimationClip, CroconotMenu},
		brodler::BrodlerMenu,
		dui::DuiMenu,
		chupri::ChupriMenu,
		lidder::LidderMenu,
		lero::LeroMenu,
		mygr::MygrMenu,
		spibmom::SpibmomMenu,
		sonyak::{SonyakAnimationClip, SonyakMenu},
		wumbus::WumbusMenu,
		ylter::{YilterAnimationClip, YilterMenu},
	},
	cycle_value,
	event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConceptSpecies {
	Braidman,
	Brenal,
	Caole,
	Hars,
	Yilter,
	Sonyak,
	Claber,
	Croconot,
	Brodler,
	Mygr,
	Dui,
	Lidder,
	Chupri,
	Wumbus,
	Lero,
	Spibmom,
}

impl ConceptSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Brenal => "brenal",
			Self::Caole => "caole",
			Self::Hars => "hars",
			Self::Yilter => "ylter",
			Self::Sonyak => "sonyak",
			Self::Claber => "claber",
			Self::Croconot => "croconot",
			Self::Brodler => "brodler",
			Self::Mygr => "mygr",
			Self::Dui => "dui",
			Self::Lidder => "lidder",
			Self::Chupri => "chupri",
			Self::Wumbus => "wumbus",
			Self::Lero => "lero",
			Self::Spibmom => "spibmom",
		}
	}
}

impl ListValues for ConceptSpecies {
	fn values() -> &'static [Self] {
		&[
			Self::Braidman,
			Self::Brenal,
			Self::Caole,
			Self::Hars,
			Self::Yilter,
			Self::Sonyak,
			Self::Claber,
			Self::Croconot,
			Self::Brodler,
			Self::Mygr,
			Self::Dui,
			Self::Lidder,
			Self::Chupri,
			Self::Wumbus,
			Self::Lero,
			Self::Spibmom,
		]
	}
}

impl LabelOption for ConceptSpecies {
	fn label(&self) -> &'static str {
		match *self {
			Self::Braidman => "braidman",
			Self::Brenal => "brenal",
			Self::Caole => "caole",
			Self::Hars => "hars",
			Self::Yilter => "ylter",
			Self::Sonyak => "sonyak",
			Self::Claber => "claber",
			Self::Croconot => "croconot",
			Self::Brodler => "brodler",
			Self::Mygr => "mygr",
			Self::Dui => "dui",
			Self::Lidder => "lidder",
			Self::Chupri => "chupri",
			Self::Wumbus => "wumbus",
			Self::Lero => "lero",
			Self::Spibmom => "spibmom",
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMenu {
	pub species: SingleSelect<ConceptSpecies>,
	pub braidman: BraidmanMenu,
	pub brenal: BrenalMenu,
	pub caole: CaoleMenu,
	pub hars: HarsMenu,
	pub ylter: YilterMenu,
	pub sonyak: SonyakMenu,
	pub claber: ClaberMenu,
	pub croconot: CroconotMenu,
	pub brodler: BrodlerMenu,
	pub mygr: MygrMenu,
	pub dui: DuiMenu,
	pub lidder: LidderMenu,
	pub chupri: ChupriMenu,
	pub wumbus: WumbusMenu,
	pub lero: LeroMenu,
	pub spibmom: SpibmomMenu,
}

impl CharacterMenu {
	pub fn from_braidman(config: &BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Braidman),
			braidman: BraidmanMenu::from(config).with_animation(animation),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_brenal(config: &BrenalConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Brenal),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::from(config).with_animation(animation),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_caole(config: &CaoleConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Caole),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::from(config).with_animation(animation),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_hars(config: &HarsConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Hars),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::from(config).with_animation(animation),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_ylter(config: &YilterConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Yilter),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::from(config).with_animation(animation),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_sonyak(config: &SonyakConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Sonyak),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::from(config).with_animation(animation),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}


	pub fn from_croconot(config: &CroconotConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Croconot),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::from(config).with_animation(animation),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_claber(config: &ClaberConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Claber),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::from(config).with_animation(animation),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_brodler(config: &BrodlerConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Brodler),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::from(config).with_animation(animation),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_mygr(config: &MygrConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Mygr),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::from(config).with_animation(animation),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_dui(config: &DuiConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Dui),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::from(config).with_animation(animation),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}


	pub fn from_lidder(config: &LidderConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Lidder),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::from(config).with_animation(animation),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_chupri(config: &ChupriConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Chupri),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::from(config).with_animation(animation),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_wumbus(config: &WumbusConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Wumbus),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::from(config).with_animation(animation),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_lero(config: &LeroConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Lero),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::from(config).with_animation(animation),
			spibmom: SpibmomMenu::default(),
		}
	}

	pub fn from_spibmom(config: &SpibmomConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Spibmom),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::from(config).with_animation(animation),
		}
	}

	/// Lowers the currently selected species menu; the other species' state is
	/// retained but not part of the tree.
	fn species_node(&self) -> MenuNode<MenuEvent> {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.menu_node(),
			ConceptSpecies::Brenal => self.brenal.menu_node(),
			ConceptSpecies::Caole => self.caole.menu_node(),
			ConceptSpecies::Hars => self.hars.menu_node(),
			ConceptSpecies::Yilter => self.ylter.menu_node(),
			ConceptSpecies::Sonyak => self.sonyak.menu_node(),
			ConceptSpecies::Claber => self.claber.menu_node(),
			ConceptSpecies::Croconot => self.croconot.menu_node(),
			ConceptSpecies::Brodler => self.brodler.menu_node(),
			ConceptSpecies::Mygr => self.mygr.menu_node(),
			ConceptSpecies::Dui => self.dui.menu_node(),
			ConceptSpecies::Lidder => self.lidder.menu_node(),
			ConceptSpecies::Chupri => self.chupri.menu_node(),
			ConceptSpecies::Wumbus => self.wumbus.menu_node(),
			ConceptSpecies::Lero => self.lero.menu_node(),
			ConceptSpecies::Spibmom => self.spibmom.menu_node(),
		}
	}

	pub fn animation(&self) -> ConceptAnimation {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.animation(),
			ConceptSpecies::Brenal => self.brenal.animation(),
			ConceptSpecies::Caole => self.caole.animation(),
			ConceptSpecies::Hars => self.hars.animation(),
			ConceptSpecies::Yilter => self.ylter.animation(),
			ConceptSpecies::Sonyak => self.sonyak.animation(),
			ConceptSpecies::Claber => self.claber.animation(),
			ConceptSpecies::Croconot => self.croconot.animation(),
			ConceptSpecies::Brodler => self.brodler.animation(),
			ConceptSpecies::Mygr => self.mygr.animation(),
			ConceptSpecies::Dui => self.dui.animation(),
			ConceptSpecies::Lidder => self.lidder.animation(),
			ConceptSpecies::Chupri => self.chupri.animation(),
			ConceptSpecies::Wumbus => self.wumbus.animation(),
			ConceptSpecies::Lero => self.lero.animation(),
			ConceptSpecies::Spibmom => self.spibmom.animation(),
		}
	}

	pub fn braidman_config(&self) -> BraidmanConfig {
		BraidmanConfig::from(&self.braidman)
	}

	pub fn brenal_config(&self) -> BrenalConfig {
		BrenalConfig::from(&self.brenal)
	}

	pub fn caole_config(&self) -> CaoleConfig {
		CaoleConfig::from(&self.caole)
	}

	pub fn hars_config(&self) -> HarsConfig {
		HarsConfig::from(&self.hars)
	}

	pub fn ylter_config(&self) -> YilterConfig {
		YilterConfig::from(&self.ylter)
	}

	pub fn sonyak_config(&self) -> SonyakConfig {
		SonyakConfig::from(&self.sonyak)
	}

	pub fn claber_config(&self) -> ClaberConfig {
		ClaberConfig::from(&self.claber)
	}

	pub fn croconot_config(&self) -> CroconotConfig {
		CroconotConfig::from(&self.croconot)
	}

	pub fn brodler_config(&self) -> BrodlerConfig {
		BrodlerConfig::from(&self.brodler)
	}

	pub fn mygr_config(&self) -> MygrConfig {
		MygrConfig::from(&self.mygr)
	}

	pub fn dui_config(&self) -> DuiConfig {
		DuiConfig::from(&self.dui)
	}

	pub fn lidder_config(&self) -> LidderConfig {
		LidderConfig::from(&self.lidder)
	}

	pub fn chupri_config(&self) -> ChupriConfig {
		ChupriConfig::from(&self.chupri)
	}

	pub fn wumbus_config(&self) -> WumbusConfig {
		WumbusConfig::from(&self.wumbus)
	}

	pub fn lero_config(&self) -> LeroConfig {
		LeroConfig::from(&self.lero)
	}

	pub fn spibmom_config(&self) -> SpibmomConfig {
		SpibmomConfig::from(&self.spibmom)
	}

	pub fn apply(&mut self, event: MenuEvent) -> bool {
		match event {
			MenuEvent::SetSpecies(species) => {
				if self.species.value == species {
					return false;
				}
				self.species.value = species;
				return true;
			}
			MenuEvent::ToggleSection(_) => return false,
			_ => {}
		}
		match self.species.value {
			ConceptSpecies::Braidman => self.apply_braidman(event),
			ConceptSpecies::Brenal => self.apply_brenal(event),
			ConceptSpecies::Caole => self.apply_caole(event),
			ConceptSpecies::Hars => self.apply_hars(event),
			ConceptSpecies::Yilter => self.apply_ylter(event),
			ConceptSpecies::Sonyak => self.apply_sonyak(event),
			ConceptSpecies::Claber => self.apply_claber(event),
			ConceptSpecies::Croconot => self.apply_croconot(event),
			ConceptSpecies::Brodler => self.apply_brodler(event),
			ConceptSpecies::Mygr => self.apply_mygr(event),
			ConceptSpecies::Dui => self.apply_dui(event),
			ConceptSpecies::Lidder => self.apply_lidder(event),
			ConceptSpecies::Chupri => self.apply_chupri(event),
			ConceptSpecies::Wumbus => self.apply_wumbus(event),
			ConceptSpecies::Lero => self.apply_lero(event),
			ConceptSpecies::Spibmom => self.apply_spibmom(event),
		}
	}

	pub fn camera_focus_for_event(&self, event: MenuEvent) -> Option<CameraFocus> {
		let field = match event {
			MenuEvent::SetAsset(field, _) | MenuEvent::Cycle(field, _) => Some(field),
			MenuEvent::ToggleClothing(clothing) => Some(CharacterField::Clothing(clothing)),
			_ => None,
		}?;
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.camera_focus_for_field(field),
			ConceptSpecies::Brenal => self.brenal.camera_focus_for_field(field),
			ConceptSpecies::Caole => self.caole.camera_focus_for_field(field),
			ConceptSpecies::Hars => self.hars.camera_focus_for_field(field),
			ConceptSpecies::Yilter => self.ylter.camera_focus_for_field(field),
			ConceptSpecies::Sonyak => self.sonyak.camera_focus_for_field(field),
			ConceptSpecies::Claber => self.claber.camera_focus_for_field(field),
			ConceptSpecies::Croconot => self.croconot.camera_focus_for_field(field),
			ConceptSpecies::Brodler => self.brodler.camera_focus_for_field(field),
			ConceptSpecies::Mygr => self.mygr.camera_focus_for_field(field),
			ConceptSpecies::Dui => self.dui.camera_focus_for_field(field),
			ConceptSpecies::Lidder => self.lidder.camera_focus_for_field(field),
			ConceptSpecies::Chupri => self.chupri.camera_focus_for_field(field),
			ConceptSpecies::Wumbus => self.wumbus.camera_focus_for_field(field),
			ConceptSpecies::Lero => self.lero.camera_focus_for_field(field),
			ConceptSpecies::Spibmom => self.spibmom.camera_focus_for_field(field),
		}
	}

	fn apply_braidman(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.braidman;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BodyMesh, AssetValue::Body(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::HeadMesh, AssetValue::Head(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_braidman_slider(menu, field, delta),
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HairColor => {
					menu.hair.value.color.value = color;
					true
				}
				CharacterField::Clothing(clothing) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) => false,
		}
	}

	fn apply_brenal(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brenal;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::BrenalHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrenalBody, AssetValue::BrenalBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::BrenalHead, AssetValue::BrenalHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::BrenalMouth, AssetValue::BrenalMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = BrenalAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_brenal_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_caole(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.caole;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::CaoleBody, AssetValue::CaoleBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::CaoleMouth, AssetValue::CaoleMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = CaoleAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_caole_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_hars(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.hars;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::HarsBody, AssetValue::HarsBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::HarsMouth, AssetValue::HarsMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = HarsAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_hars_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_ylter(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.ylter;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::YilterBody, AssetValue::YilterBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::YilterMouth, AssetValue::YilterMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = YilterAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_ylter_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_sonyak(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.sonyak;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::SonyakBody, AssetValue::SonyakBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::SonyakMouth, AssetValue::SonyakMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = SonyakAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_sonyak_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HairColor => {
					menu.head_features.value.hair_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}


	fn apply_croconot(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.croconot;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::CroconotHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::CroconotBody, AssetValue::CroconotBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::CroconotHead, AssetValue::CroconotHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::CroconotMouth, AssetValue::CroconotMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = CroconotAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_croconot_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_claber(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.claber;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::ClaberHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::ClaberBody, AssetValue::ClaberBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::ClaberHead, AssetValue::ClaberHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::ClaberMouth, AssetValue::ClaberMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = ClaberAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_claber_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Claber(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleClothing(_) => {
				false
			}
		}
	}

	fn apply_brodler(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brodler;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrodlerHead, AssetValue::BrodlerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Horns, AssetValue::Horns(value)) => {
					menu.head.value.horns.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::SkinColor, SwatchValue::BrodlerSkin(color)) => {
					menu.head.value.skin.value = color;
					menu.head_features.value.skin_color = color;
					true
				}
				(CharacterField::BrodlerEyeColor, SwatchValue::BrodlerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::HornColor, SwatchValue::BrodlerHorn(color)) => {
					menu.head_features.value.horn_color.value = color;
					menu.head.value.horn_color = color;
					true
				}
				(CharacterField::MouthColor, SwatchValue::Item(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_mygr(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.mygr;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::MygrHead, AssetValue::MygrHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::MygrMouth, AssetValue::MygrMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::MygrSkinColor, SwatchValue::MygrSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::MygrEyeColor, SwatchValue::MygrEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::MouthColor, SwatchValue::Item(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_dui(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.dui;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::DuiNose, delta) => {
				menu.head_features.value.nose.value =
					cycle_value(menu.head_features.value.nose.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::DuiHead, AssetValue::DuiHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::DuiEye, AssetValue::DuiEye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::DuiMouth, AssetValue::DuiMouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::DuiSkinColor, SwatchValue::DuiSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::DuiMouthColor, SwatchValue::DuiMouth(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}


	fn apply_lidder(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.lidder;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::LidderHead, AssetValue::LidderHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::LidderBeak, AssetValue::LidderBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::LidderPlumageColor, SwatchValue::LidderPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::LidderEyeColor, SwatchValue::LidderEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::LidderBeakColor, SwatchValue::LidderBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_chupri(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.chupri;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::ChupriHead, AssetValue::ChupriHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::ChupriBeak, AssetValue::ChupriBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::ChupriPlumageColor, SwatchValue::ChupriPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::ChupriEyeColor, SwatchValue::ChupriEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::ChupriBeakColor, SwatchValue::ChupriBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_wumbus(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.wumbus;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::Cycle(CharacterField::WumbusHorns, delta) => {
				menu.head.value.horns.value = cycle_value(menu.head.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::WumbusHead, AssetValue::WumbusHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::WumbusMouth, AssetValue::WumbusMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::WumbusSkinColor, SwatchValue::WumbusSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::WumbusEyeColor, SwatchValue::WumbusEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::WumbusEarColor, SwatchValue::WumbusEar(color)) => {
					menu.head_features.value.ear_color.value = color;
					true
				}
				(CharacterField::WumbusMouthColor, SwatchValue::WumbusMouth(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::WumbusHornColor, SwatchValue::WumbusHorn(color)) => {
					menu.head.value.horn_color.value = color;
					true
				}
				(CharacterField::WumbusSpineColor, SwatchValue::WumbusSpine(color)) => {
					menu.head.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_lero(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.lero;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::LeroHead, AssetValue::LeroHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::LeroMouth, AssetValue::LeroMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::LeroSkinColor, SwatchValue::LeroSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::LeroEyeColor, SwatchValue::LeroEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::LeroMouthColor, SwatchValue::LeroMouthColor(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::LeroTailColor, SwatchValue::LeroTail(color)) => {
					menu.body.value.tail_color.value = color;
					true
				}
				(CharacterField::LeroSpineColor, SwatchValue::LeroSpine(color)) => {
					menu.body.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_spibmom(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.spibmom;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::SpibmomHead, AssetValue::SpibmomHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::SpibmomMouth, AssetValue::SpibmomMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::SpibmomSkinColor, SwatchValue::SpibmomSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::SpibmomEyeColor, SwatchValue::SpibmomEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::SpibmomEarColor, SwatchValue::SpibmomEar(color)) => {
					menu.head_features.value.ear_color.value = color;
					true
				}
				(CharacterField::SpibmomMouthColor, SwatchValue::SpibmomMouthColor(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::SpibmomCrownColor, SwatchValue::SpibmomCrown(color)) => {
					menu.head.value.crown_color.value = color;
					true
				}
				(CharacterField::SpibmomSpineColor, SwatchValue::SpibmomSpine(color)) => {
					menu.head.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}
}

impl MenuComponent<MenuEvent> for CharacterMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::section_select(
			"Species",
			self.species.value,
			MenuEvent::SetSpecies,
			self.species_node(),
		)
	}
}

fn apply_brenal_slider(menu: &mut BrenalMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_caole_slider(menu: &mut CaoleMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_hars_slider(menu: &mut HarsMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_ylter_slider(menu: &mut YilterMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_sonyak_slider(menu: &mut SonyakMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}


fn apply_croconot_slider(menu: &mut CroconotMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		CharacterField::SnoutLength => face.snout_length = face.snout_length.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_claber_slider(menu: &mut ClaberMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		CharacterField::SnoutLength => face.snout_length = face.snout_length.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_braidman_slider(menu: &mut BraidmanMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::NoseWidth => face.nose_width = face.nose_width.apply_delta(delta),
		CharacterField::NoseHeight => face.nose_height = face.nose_height.apply_delta(delta),
		CharacterField::MouthWidth => face.mouth_width = face.mouth_width.apply_delta(delta),
		CharacterField::MouthHeight => face.mouth_height = face.mouth_height.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

impl Default for CharacterMenu {
	fn default() -> Self {
		Self::from_braidman(&BraidmanConfig::default_preview(), ConceptAnimation::default())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionOpenState {
	pub presets_open: bool,
	pub head_open: bool,
	pub body_open: bool,
	pub head_features_open: bool,
	pub hair_open: bool,
	pub clothing_open: bool,
	pub animation_open: bool,
}

impl SectionOpenState {
	pub fn is_section_open(self, section: SectionId) -> bool {
		match section {
			SectionId::Presets => self.presets_open,
			SectionId::Head => self.head_open,
			SectionId::Body => self.body_open,
			SectionId::HeadFeatures => self.head_features_open,
			SectionId::Hair => self.hair_open,
			SectionId::Clothing => self.clothing_open,
			SectionId::Animation => self.animation_open,
		}
	}

	pub fn toggle(&mut self, section: SectionId) {
		match section {
			SectionId::Presets => self.presets_open = !self.presets_open,
			SectionId::Head => self.head_open = !self.head_open,
			SectionId::Body => self.body_open = !self.body_open,
			SectionId::HeadFeatures => self.head_features_open = !self.head_features_open,
			SectionId::Hair => self.hair_open = !self.hair_open,
			SectionId::Clothing => self.clothing_open = !self.clothing_open,
			SectionId::Animation => self.animation_open = !self.animation_open,
		}
	}
}

impl Default for SectionOpenState {
	fn default() -> Self {
		Self {
			presets_open: true,
			head_open: true,
			body_open: true,
			head_features_open: false,
			hair_open: false,
			clothing_open: true,
			animation_open: false,
		}
	}
}
