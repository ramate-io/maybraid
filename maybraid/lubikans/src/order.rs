//! Editable account order. Charge, activation, and selection remain in the ledger.

use crate::{LubikanAccountId, LubikanLedger};

/// A menu can edit this projection without copying live charge or activation state.
/// All owned Lubikans participate in the queue, including the non-removable Self.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LubikanOrder {
	accounts: Vec<LubikanAccountId>,
}

impl LubikanOrder {
	pub fn from_ledger(ledger: &LubikanLedger) -> Self {
		Self { accounts: ledger.iter().map(|account| account.id()).collect() }
	}

	pub fn accounts(&self) -> &[LubikanAccountId] {
		&self.accounts
	}

	/// Stable within this catalog even when the displayed order changes.
	pub fn catalog_index(&self, account: LubikanAccountId) -> Option<usize> {
		self.accounts
			.contains(&account)
			.then(|| self.accounts.iter().filter(|&&other| other < account).count())
	}

	/// Move exactly one adjacent slot, without wrapping at either end.
	pub fn move_account(&mut self, account: LubikanAccountId, delta: i8) -> bool {
		let Some(from) = self.accounts.iter().position(|&owned| owned == account) else {
			return false;
		};
		let to = (from as isize + delta.signum() as isize)
			.clamp(0, self.accounts.len() as isize - 1) as usize;
		if from == to {
			return false;
		}
		self.accounts.swap(from, to);
		true
	}

	/// Swap an owned account into an exact queue position. Self is protected in
	/// the slot picker. Returns acceptance, including a same-account no-op.
	pub fn assign_to_slot(&mut self, account: LubikanAccountId, position: usize) -> bool {
		let Some(&target) = self.accounts.get(position) else {
			return false;
		};
		if account.is_self() || target.is_self() {
			return false;
		}
		let Some(from) = self.accounts.iter().position(|&owned| owned == account) else {
			return false;
		};
		self.accounts.swap(from, position);
		true
	}
}

impl Default for LubikanOrder {
	fn default() -> Self {
		Self { accounts: vec![LubikanAccountId::SELF] }
	}
}

#[cfg(test)]
mod tests {
	use crate::{LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanOrder};

	#[test]
	fn slot_assignment_swaps_owned_accounts_and_protects_self() {
		let mut ledger = LubikanLedger::default();
		let first = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(8),
			20.0,
			5.0,
		));
		let second = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(3),
			20.0,
			5.0,
		));
		let mut order = LubikanOrder::from_ledger(&ledger);
		assert!(order.assign_to_slot(second, 1));
		assert_eq!(order.accounts(), &[crate::LubikanAccountId::SELF, second, first]);
		let before = order.clone();
		assert!(order.assign_to_slot(second, 1), "same account completes selection");
		assert_eq!(order, before);
		for (account, position) in [
			(crate::LubikanAccountId::SELF, 1),
			(first, 0),
			(first, 3),
			(first, usize::MAX),
			(crate::LubikanAccountId::unique(LubikanDefinitionId::from_raw(99)), 1),
		] {
			assert!(!order.assign_to_slot(account, position));
			assert_eq!(order, before);
		}
	}

	#[test]
	fn reorder_preserves_identity_charge_selection_and_new_accounts() -> anyhow::Result<()> {
		let mut ledger = LubikanLedger::default();
		let first = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(8),
			20.0,
			5.0,
		));
		let second = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(3),
			20.0,
			5.0,
		));
		assert!(ledger.select(Some(first)));
		ledger.apply_to(first, 12.0);
		let mut order = LubikanOrder::from_ledger(&ledger);
		let key = order.catalog_index(second);
		assert!(order.move_account(second, -1));
		assert_eq!(order.accounts(), &[crate::LubikanAccountId::SELF, second, first]);
		assert_eq!(order.catalog_index(second), key);
		assert!(order.move_account(second, -1));
		assert!(!order.move_account(second, -1));
		assert!(!order.move_account(first, 1));
		let new = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(1),
			20.0,
			5.0,
		));
		assert!(ledger.apply_order(&order));
		assert_eq!(
			ledger.iter().map(|account| account.id()).collect::<Vec<_>>(),
			vec![second, crate::LubikanAccountId::SELF, first, new]
		);
		assert_eq!(ledger.selection().resolved(), first);
		assert_eq!(
			ledger
				.account(first)
				.ok_or_else(|| anyhow::anyhow!("first account"))?
				.charge()
				.current(),
			12.0
		);
		assert!(!ledger.apply_order(&order));
		Ok(())
	}
}
