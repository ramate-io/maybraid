//! Shared humanoid asset catalog used by multiple species.

pub mod assets;

pub use assets::{
	BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh, BODY_FULL,
	BODY_RIG, BODY_STANDARD, EAR_FLANK, EAR_ROUND, EAR_STANDARD, EYE_FALCON, EYE_STANDARD,
	HEAD_FULL, HEAD_GAUNT, HEAD_RIG, HEAD_STANDARD, HORNS_HARROWED_CROWN, HORNS_LORKEN_CROWN,
	MOUTH_STANDARD, NOSE_BALLOON, NOSE_BROAD, NOSE_LOAF, NOSE_STANDARD,
};
