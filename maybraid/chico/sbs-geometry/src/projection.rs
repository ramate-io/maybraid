//! Shared height-fraction projection profiles for stalk-and-ball-stick trees.

/// Friend's / Temperate Conifer logarithmic rounding ([RFC §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
///
/// \(\ell(u) = \ell_{\max} + (\ell_{\min} - \ell_{\max})\,\mathrm{falloff}(u)\) with
/// \(\mathrm{falloff}(u) = \ln(1 + \alpha u^\beta) / \ln(1 + \alpha)\).
pub fn logarithmic_rounding_projection(
	ell_max: f32,
	ell_min: f32,
	u: f32,
	alpha: f32,
	beta: f32,
) -> f32 {
	let u = u.clamp(0.0, 1.0);
	let alpha = alpha.max(1e-6);
	let denom = (1.0 + alpha).ln();
	let falloff = if denom.abs() < 1e-12 {
		u
	} else {
		(1.0 + alpha * u.powf(beta)).ln() / denom
	};
	ell_max + (ell_min - ell_max) * falloff
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn endpoints_match_max_and_min() {
		let h = 30.0;
		let ell_max = 0.06 * h;
		let ell_min = 0.015 * h;
		let l0 = logarithmic_rounding_projection(ell_max, ell_min, 0.0, 8.0, 3.0);
		let l1 = logarithmic_rounding_projection(ell_max, ell_min, 1.0, 8.0, 3.0);
		assert!((l0 - ell_max).abs() < 1e-4);
		assert!((l1 - ell_min).abs() < 1e-4);
	}

	#[test]
	fn mid_canopy_stays_near_max_longer_than_linear() {
		let h = 30.0;
		let ell_max = 0.06 * h;
		let ell_min = 0.015 * h;
		let u = 0.5;
		let log = logarithmic_rounding_projection(ell_max, ell_min, u, 8.0, 3.0);
		let linear = ell_max + (ell_min - ell_max) * u;
		assert!(log > linear, "log profile should delay falloff: log={log} linear={linear}");
	}
}
