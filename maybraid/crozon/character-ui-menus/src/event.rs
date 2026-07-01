use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::BraidmanColor,
		brodler::{
			assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor,
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
