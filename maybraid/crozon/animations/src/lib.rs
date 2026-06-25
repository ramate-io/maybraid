pub mod animations;
pub mod rigs;

pub trait Animation<Rig> {
	fn apply(&self, rig: &mut Rig);
}
