use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::BraidmanColor,
		brodler::{
			assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor,
		},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
		wumbus::{
            WumbusEarColor, WumbusEyeColor, WumbusHeadMesh, WumbusHornColor,
			WumbusMouthColor, WumbusMouthMesh, WumbusSkinColor, WumbusSpineColor,
		},
		dui::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiMouthColor, DuiSkinColor},
		lero::{
			LeroEyeColor, LeroHeadMesh, LeroMouthColor, LeroMouthMesh, LeroSkinColor,
			LeroSpineColor, LeroTailColor,
		},
		spibmom::{
			SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomHeadMesh, SpibmomMouthColor,
			SpibmomMouthMesh, SpibmomSkinColor, SpibmomSpineColor,
		},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
	},
	ConceptAnimation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionId {
	Presets,
	Head,
	Body,
	HeadFeatures,
	Hair,
	Clothing,
	Animation,
}

impl SectionId {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Presets => "Presets",
			Self::Head => "Head",
			Self::Body => "Body",
			Self::HeadFeatures => "Head & Features",
			Self::Hair => "Hair",
			Self::Clothing => "Clothing",
			Self::Animation => "Animation",
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterField {
	Gender,
	Build,
	BodyMesh,
	HeadMesh,
	BrodlerHead,
	Horns,
	MygrHead,
	MygrMouth,
	WumbusHead,
	WumbusMouth,
	WumbusHorns,
	DuiHead,
	DuiEye,
	DuiNose,
	DuiMouth,
	Eye,
	Nose,
	Mouth,
	Ear,
	Hair,
	Animation,
	BodyColor,
	EyeColor,
	MouthColor,
	HairColor,
	SkinColor,
	BrodlerEyeColor,
	HornColor,
	MygrSkinColor,
	MygrEyeColor,
	WumbusSkinColor,
	WumbusEyeColor,
	WumbusEarColor,
	WumbusMouthColor,
	WumbusHornColor,
	WumbusSpineColor,
	DuiSkinColor,
	DuiMouthColor,
	LeroHead,
	LeroMouth,
	LeroMouthColor,
	LeroSkinColor,
	LeroEyeColor,
	LeroTailColor,
	LeroSpineColor,
	SpibmomHead,
	SpibmomMouth,
	SpibmomSkinColor,
	SpibmomEyeColor,
	SpibmomEarColor,
	SpibmomMouthColor,
	SpibmomCrownColor,
	SpibmomSpineColor,
	Clothing(ClothingMesh),
	ShoulderWidth,
	HipWidth,
	ChestThickness,
	HipThickness,
	LegThickness,
	ButtocksThickness,
	WaistThickness,
	LowerTrunkThickness,
	ArmLength,
	ArmThickness,
	LegLength,
	EyeWidth,
	EyeHeight,
	EyeTilt,
	NoseWidth,
	NoseHeight,
	MouthWidth,
	MouthHeight,
	EarWidth,
	EarHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetValue {
	Gender(GenderPreset),
	Build(BuildPreset),
	Body(BodyMesh),
	Head(HeadMesh),
	BrodlerHead(BrodlerHeadMesh),
	Horns(HornMesh),
	MygrHead(MygrHeadMesh),
	MygrMouth(MygrMouthMesh),
	WumbusHead(WumbusHeadMesh),
	WumbusMouth(WumbusMouthMesh),
	LeroHead(LeroHeadMesh),
	LeroMouth(LeroMouthMesh),
	SpibmomHead(SpibmomHeadMesh),
	SpibmomMouth(SpibmomMouthMesh),
	DuiHead(DuiHeadMesh),
	DuiEye(DuiEyeMesh),
	DuiMouth(DuiMouthMesh),
	Eye(EyeMesh),
	Nose(NoseMesh),
	Mouth(MouthMesh),
	Ear(EarMesh),
	Hair(HairMesh),
	Animation(ConceptAnimation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwatchValue {
	Braidman(BraidmanColor),
	BrodlerSkin(BrodlerSkinColor),
	BrodlerEye(BrodlerEyeColor),
	BrodlerHorn(BrodlerHornColor),
	MygrSkin(MygrSkinColor),
	MygrEye(MygrEyeColor),
	WumbusSkin(WumbusSkinColor),
	WumbusEye(WumbusEyeColor),
	WumbusEar(WumbusEarColor),
	WumbusMouth(WumbusMouthColor),
	WumbusHorn(WumbusHornColor),
	WumbusSpine(WumbusSpineColor),
	DuiSkin(DuiSkinColor),
	DuiMouth(DuiMouthColor),
	LeroSkin(LeroSkinColor),
	LeroEye(LeroEyeColor),
	LeroMouthColor(LeroMouthColor),
	LeroTail(LeroTailColor),
	LeroSpine(LeroSpineColor),
	SpibmomSkin(SpibmomSkinColor),
	SpibmomEye(SpibmomEyeColor),
	SpibmomEar(SpibmomEarColor),
	SpibmomMouthColor(SpibmomMouthColor),
	SpibmomCrown(SpibmomCrownColor),
	SpibmomSpine(SpibmomSpineColor),
}

impl SwatchValue {
	pub fn with_braidman(self, color: BraidmanColor) -> Self {
		Self::Braidman(color)
	}

	pub fn with_brodler_skin(self, color: BrodlerSkinColor) -> Self {
		Self::BrodlerSkin(color)
	}

	pub fn with_brodler_eye(self, color: BrodlerEyeColor) -> Self {
		Self::BrodlerEye(color)
	}

	pub fn with_brodler_horn(self, color: BrodlerHornColor) -> Self {
		Self::BrodlerHorn(color)
	}

	pub fn with_mygr_skin(self, color: MygrSkinColor) -> Self {
		Self::MygrSkin(color)
	}

	pub fn with_mygr_eye(self, color: MygrEyeColor) -> Self {
		Self::MygrEye(color)
	}

	pub fn with_dui_skin(self, color: DuiSkinColor) -> Self {
		Self::DuiSkin(color)
	}

	pub fn with_dui_mouth(self, color: DuiMouthColor) -> Self {
		Self::DuiMouth(color)
	}

	pub fn with_wumbus_skin(self, color: WumbusSkinColor) -> Self {
		Self::WumbusSkin(color)
	}

	pub fn with_wumbus_eye(self, color: WumbusEyeColor) -> Self {
		Self::WumbusEye(color)
	}

	pub fn with_wumbus_ear(self, color: WumbusEarColor) -> Self {
		Self::WumbusEar(color)
	}

	pub fn with_wumbus_mouth(self, color: WumbusMouthColor) -> Self {
		Self::WumbusMouth(color)
	}

	pub fn with_wumbus_horn(self, color: WumbusHornColor) -> Self {
		Self::WumbusHorn(color)
	}

	pub fn with_wumbus_spine(self, color: WumbusSpineColor) -> Self {
		Self::WumbusSpine(color)
	}

	pub fn with_lero_skin(self, color: LeroSkinColor) -> Self {
		Self::LeroSkin(color)
	}

	pub fn with_lero_eye(self, color: LeroEyeColor) -> Self {
		Self::LeroEye(color)
	}

	pub fn with_lero_tail(self, color: LeroTailColor) -> Self {
		Self::LeroTail(color)
	}

	pub fn with_lero_spine(self, color: LeroSpineColor) -> Self {
		Self::LeroSpine(color)
	}

	pub fn with_lero_mouth_color(self, color: LeroMouthColor) -> Self {
		Self::LeroMouthColor(color)
	}

	pub fn with_spibmom_skin(self, color: SpibmomSkinColor) -> Self {
		Self::SpibmomSkin(color)
	}

	pub fn with_spibmom_eye(self, color: SpibmomEyeColor) -> Self {
		Self::SpibmomEye(color)
	}

	pub fn with_spibmom_ear(self, color: SpibmomEarColor) -> Self {
		Self::SpibmomEar(color)
	}

	pub fn with_spibmom_mouth_color(self, color: SpibmomMouthColor) -> Self {
		Self::SpibmomMouthColor(color)
	}

	pub fn with_spibmom_crown(self, color: SpibmomCrownColor) -> Self {
		Self::SpibmomCrown(color)
	}

	pub fn with_spibmom_spine(self, color: SpibmomSpineColor) -> Self {
		Self::SpibmomSpine(color)
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuEvent {
	ToggleSection(SectionId),
	SetSpecies(crate::character::ConceptSpecies),
	Cycle(CharacterField, i32),
	SetAsset(CharacterField, AssetValue),
	SliderDelta(CharacterField, f32),
	ToggleClothing(ClothingMesh),
	SetSwatch(CharacterField, SwatchValue),
}
