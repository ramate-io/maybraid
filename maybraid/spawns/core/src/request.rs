//! Request identity and the session owner that issued it.

/// Monotonic identity for one spawn or arrival request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpawnRequestId(u64);

impl SpawnRequestId {
	pub const fn new(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}

	pub const fn next(self) -> Self {
		Self(self.0.wrapping_add(1))
	}

	pub const fn is_unset(self) -> bool {
		self.0 == 0
	}
}

/// One play session's spawn owner. A later session cannot consume an earlier
/// request's outcome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpawnSessionId(u64);

impl SpawnSessionId {
	pub const fn new(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}

	pub const fn next(self) -> Self {
		Self(self.0.wrapping_add(1))
	}

	pub const fn is_unset(self) -> bool {
		self.0 == 0
	}
}

/// Who issued a spawn request. Controllers compare this, they do not rediscover
/// a "current player" or "current session".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpawnOwner {
	pub session: SpawnSessionId,
}

impl SpawnOwner {
	pub const fn new(session: SpawnSessionId) -> Self {
		Self { session }
	}

	pub fn matches(self, other: Self) -> bool {
		self.session == other.session
	}

	pub fn owns(self, session: SpawnSessionId) -> bool {
		self.session == session
	}
}

/// Identity stamped on every domain request and outcome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpawnIdentity {
	pub request: SpawnRequestId,
	pub owner: SpawnOwner,
}

impl SpawnIdentity {
	pub const fn new(request: SpawnRequestId, owner: SpawnOwner) -> Self {
		Self { request, owner }
	}

	pub fn matches(self, other: Self) -> bool {
		self.request == other.request && self.owner.matches(other.owner)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_new_session_does_not_match_the_previous_owner() {
		let first = SpawnOwner::new(SpawnSessionId::new(1));
		let second = SpawnOwner::new(first.session.next());
		assert!(!first.matches(second));
		assert!(!SpawnIdentity::new(SpawnRequestId::new(3), first)
			.matches(SpawnIdentity::new(SpawnRequestId::new(3), second)));
	}
}
