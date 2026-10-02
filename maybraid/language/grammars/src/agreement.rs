//! Predicate agreement with a controller argument.

use maybraid_language_core::{Number, ParticleDomain, Person, SemanticNode, SurfaceConstituent};

use crate::marking::Marking;

/// Which grammatical relation controls verbal agreement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgreementController {
	None,
	Subject,
	Object,
	SubjectAndObject,
}

/// Person/number features copied from a controller referent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgreementFeatures {
	pub person: Person,
	pub number: Number,
}

/// Configurable agreement markers. Gender/noun-class can be added later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgreementStrategy {
	pub controller: AgreementController,
	pub plural: Marking,
	pub first: Marking,
	pub second: Marking,
	pub third: Marking,
}

impl AgreementStrategy {
	pub fn unmarked() -> Self {
		Self {
			controller: AgreementController::None,
			plural: Marking::Zero,
			first: Marking::Zero,
			second: Marking::Zero,
			third: Marking::Zero,
		}
	}

	pub fn subject_number(plural: Marking) -> Self {
		Self { controller: AgreementController::Subject, plural, ..Self::unmarked() }
	}

	pub fn apply(
		self,
		predicate: &mut Vec<SurfaceConstituent>,
		subject: Option<AgreementFeatures>,
		object: Option<AgreementFeatures>,
		host: SemanticNode,
	) {
		match self.controller {
			AgreementController::None => {}
			AgreementController::Subject => self.mark(predicate, subject, host),
			AgreementController::Object => self.mark(predicate, object, host),
			AgreementController::SubjectAndObject => {
				self.mark(predicate, subject, host);
				self.mark(predicate, object, host);
			}
		}
	}

	fn mark(
		self,
		predicate: &mut Vec<SurfaceConstituent>,
		features: Option<AgreementFeatures>,
		host: SemanticNode,
	) {
		let Some(features) = features else {
			return;
		};
		match features.person {
			Person::First => self.first.apply(predicate, ParticleDomain::Agreement, Some(host)),
			Person::Second => self.second.apply(predicate, ParticleDomain::Agreement, Some(host)),
			Person::Third => self.third.apply(predicate, ParticleDomain::Agreement, Some(host)),
			Person::Unspecified => {}
		}
		if matches!(features.number, Number::Plural | Number::Many) {
			self.plural.apply(predicate, ParticleDomain::Agreement, Some(host));
		}
	}
}
