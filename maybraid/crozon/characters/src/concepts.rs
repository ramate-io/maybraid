//! Shared concept-screen identifiers used by CLI, menus, and playgrounds.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ConceptAnimation {
	#[default]
	Still,
	Walk,
	Run,
	Gallop,
	Jump,
	Tuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
}

impl ConceptAnimation {
	pub const VALUES: &'static [Self] = &[
		Self::Still,
		Self::Walk,
		Self::Run,
		Self::Gallop,
		Self::Jump,
		Self::Tuck,
		Self::TuckedFlip,
		Self::TwoFootedTuckedFlip,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Walk => "walk",
			Self::Run => "run",
			Self::Gallop => "gallop",
			Self::Jump => "jump",
			Self::Tuck => "tuck",
			Self::TuckedFlip => "tucked-flip",
			Self::TwoFootedTuckedFlip => "two-footed-tucked-flip",
		}
	}
}
