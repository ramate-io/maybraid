//! Parent forest bias means passed into grove sampling ([RFC-183 3.5.1.1]).

/// Unit-interval preferred means inside grove-authored ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Grove Biases"))]
pub struct ForestGroveBiases {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub scale_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub density_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub offset_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub noise_amplitude_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.5))]
	pub noise_frequency_mean: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub bucket_mean_shift: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub bucket_perturbation_bias: f32,
}

impl Default for ForestGroveBiases {
	fn default() -> Self {
		Self {
			scale_mean: 0.5,
			density_mean: 0.5,
			offset_mean: 0.5,
			noise_amplitude_mean: 0.5,
			noise_frequency_mean: 0.5,
			bucket_mean_shift: 0.0,
			bucket_perturbation_bias: 0.0,
		}
	}
}
