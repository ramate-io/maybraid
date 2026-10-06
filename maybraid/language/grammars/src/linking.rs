//! Embedding of `SemanticValue::Clause` complements.

use maybraid_language_core::{
	AffixPlacement, BoundaryKind, ParticleDomain, SemanticNode, SurfaceConstituent,
};

/// How an embedded clause is introduced. Coordination of roots is separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClauseLinkStrategy {
	Zero,
	Complementizer { form: &'static str, before: bool },
	SubordinateParticle { form: &'static str, before: bool },
	Converb { suffix: &'static str },
	Chain,
}

impl ClauseLinkStrategy {
	pub fn zero() -> Self {
		Self::Zero
	}

	pub fn wrap(
		self,
		mut embedded: Vec<SurfaceConstituent>,
		host: Option<SemanticNode>,
	) -> Vec<SurfaceConstituent> {
		if embedded.is_empty() {
			return embedded;
		}
		match self {
			Self::Zero => embedded,
			Self::Complementizer { form, before } => {
				wrap_particle(embedded, form, before, ParticleDomain::Complementizer, host)
			}
			Self::SubordinateParticle { form, before } => {
				wrap_particle(embedded, form, before, ParticleDomain::Clause, host)
			}
			Self::Converb { suffix } => {
				if let Some(host) = host {
					embedded.push(SurfaceConstituent::Affix {
						form: suffix.to_owned(),
						domain: ParticleDomain::Clause,
						host,
						placement: AffixPlacement::Suffix,
					});
				}
				embedded
			}
			Self::Chain => {
				let mut out = vec![SurfaceConstituent::Boundary { kind: BoundaryKind::Serial }];
				out.extend(embedded);
				out
			}
		}
	}
}

fn wrap_particle(
	mut embedded: Vec<SurfaceConstituent>,
	form: &'static str,
	before: bool,
	domain: ParticleDomain,
	host: Option<SemanticNode>,
) -> Vec<SurfaceConstituent> {
	let particle = SurfaceConstituent::Particle { form: form.to_owned(), domain, host };
	if before {
		embedded.insert(0, particle);
	} else {
		embedded.push(particle);
	}
	embedded
}
