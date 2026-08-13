//! Semantic part slots used by character recipes and debug/UI reporting.

/// Semantic slot used for debugging and future UI/status reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CharacterPartSlot {
	#[default]
	BodyMesh,
	/// Intermediate OwnRig armature socketed between body and head (e.g. multi-bone neck).
	NeckRig,
	NeckMesh,
	HeadRig,
	HeadMesh,
	EyeLeft,
	EyeRight,
	Nose,
	Mouth,
	EarLeft,
	EarRight,
	Hair,
	Horns,
	Clothing,
	Spine,
	Tail,
}
