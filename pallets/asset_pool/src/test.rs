#![cfg(test)]

use frame_support::{assert_noop, assert_ok};

use mock::{*};

use super::*;

#[test]
fn update_deposit_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Tokens::free_balance(DOT, &LoansModule::account_id()), 0);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 1000);
		assert_eq!(LoansModule::asset_pool_infos(DOT).total_deposit, 0);

		// provide deposit
		assert_ok!(LoansModule::update_deposit(&ALICE, DOT, 500));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 500);
		// repay deposit
		assert_ok!(LoansModule::update_deposit(&ALICE, DOT, -500));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 1000);
	});
}

#[test]
fn update_debit_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Tokens::free_balance(DOT, &LoansModule::account_id()), 0);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 1000);
		assert_eq!(LoansModule::asset_pool_infos(DOT).total_deposit, 0);

		assert_ok!(LoansModule::update_deposit(&ALICE, DOT, 500));
		// new debit
		assert_noop!(
			LoansModule::update_debit(&ALICE, DOT, 490),
			Error::<Runtime>::DepositNotEnough
		);
		assert_ok!(LoansModule::update_debit(&ALICE, DOT, 100));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 600);
		// repay debit
		assert_ok!(LoansModule::update_debit(&ALICE, DOT, -100));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 500);
	});
}

#[test]
fn calculate_debit_apy_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			LoansModule::calculate_debit_apy(Ratio::saturating_from_rational(950, 1000),
											 LoansModule::asset_pool_infos(DOT).interest_info),
			Ratio::saturating_from_rational(54, 100)
		);
	});
}

#[test]
fn on_finalize_work() {
	ExtBuilder::default().build().execute_with(|| {
		// provide deposit
		assert_ok!(LoansModule::update_deposit(&ALICE, DOT, 100));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 900);
		// new debit
		assert_ok!(LoansModule::update_debit(&ALICE, DOT, 50));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 950);
		LoansModule::on_finalize(1);
		assert_eq!(
			LoansModule::asset_pool_infos(DOT).debit_rate,
			Ratio::saturating_from_rational(1022222222222222222u128, 1000000000000000000u128)
		);
		LoansModule::on_finalize(2);
		assert_eq!(
			LoansModule::asset_pool_infos(DOT).debit_rate,
			Ratio::saturating_from_rational(1044444444444444444u128, 1000000000000000000u128)
		);
	});
}
