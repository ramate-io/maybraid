//! Bone write masks for cyclic locomotion clips. Must stay aligned with
//! `character_motion::clip_cache` prepared-table bone lists.

use std::sync::OnceLock;

use character_rigs::authoring::humanoid_write_mask;

const IDLE_BONE_NAMES: &[&str] = &[
	"lower_neck",
	"upper_neck",
	"pelvis.L",
	"pelvis.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

const WALK_BONE_NAMES: &[&str] = &[
	"root",
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

const RUN_BONE_NAMES: &[&str] = &[
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

pub fn idle_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(IDLE_BONE_NAMES))
}

pub fn walk_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(WALK_BONE_NAMES))
}

pub fn run_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(RUN_BONE_NAMES))
}
