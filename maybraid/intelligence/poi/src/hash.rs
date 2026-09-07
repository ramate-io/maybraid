/// SplitMix64-style mix used by nearby placement and replacement seeds.
pub fn mix(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

pub(crate) fn unit_f32(value: u64) -> f32 {
	((value >> 40) as f32) / ((1_u32 << 24) as f32)
}
