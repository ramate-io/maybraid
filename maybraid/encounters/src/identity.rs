//! Stable encounter identity outside HCSG generated values.
//!
//! A slug such as `bikanwarda.chained.v0` packs into 32 bytes so a later
//! HCSG `lod::gen::Id::Bytes` can reuse the same key.
//! Bounds edits must not change the id
//! ([#1121](https://github.com/ramate-io/maybraid/issues/1121)).

/// Authored encounter key. Not a Richmond cell and not a generated Arc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncounterId([u8; 32]);

impl EncounterId {
	/// Pack a UTF-8 slug into 32 zero-padded bytes.
	///
	/// Slugs longer than 32 bytes are truncated. Keep authored names short.
	pub const fn from_slug(slug: &str) -> Self {
		let raw = slug.as_bytes();
		let mut bytes = [0u8; 32];
		let mut i = 0;
		while i < raw.len() && i < 32 {
			bytes[i] = raw[i];
			i += 1;
		}
		Self(bytes)
	}

	/// Exact 32-byte key, including the mapping used by future HCSG `Id::Bytes`.
	pub const fn from_bytes(bytes: [u8; 32]) -> Self {
		Self(bytes)
	}

	pub const fn as_bytes(self) -> [u8; 32] {
		self.0
	}

	/// UTF-8 slug prefix, if the stored bytes are a zero-padded string.
	pub fn slug_str(&self) -> Option<&str> {
		let end = self.0.iter().position(|&b| b == 0).unwrap_or(32);
		std::str::from_utf8(&self.0[..end]).ok().filter(|s| !s.is_empty())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn slug_is_stable_and_independent_of_bounds() {
		let a = EncounterId::from_slug("bikanwarda.chained.v0");
		let b = EncounterId::from_slug("bikanwarda.chained.v0");
		let other = EncounterId::from_slug("bikanwarda.chained.v1");
		assert_eq!(a, b);
		assert_ne!(a, other);
		assert_eq!(a.slug_str(), Some("bikanwarda.chained.v0"));
		assert_eq!(a.as_bytes()[0], b'b');
		assert_eq!(a.as_bytes()[21], 0);
		assert_eq!(EncounterId::from_bytes(a.as_bytes()), a);
	}

	#[test]
	fn empty_and_long_slugs_are_sanitized() {
		assert_eq!(EncounterId::from_slug("").slug_str(), None);
		let long = EncounterId::from_slug("abcdefghijklmnopqrstuvwxyz0123456789");
		assert_eq!(long.slug_str(), Some("abcdefghijklmnopqrstuvwxyz012345"));
		assert_eq!(long.as_bytes()[31], b'5');
	}
}
