#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
use orml_traits::MultiReservableCurrency;
use sp_runtime::traits::BadOrigin;

use mock::{
	ALICE, BOB, DexModule, DOT, ETH, Event, ExtBuilder, FIL, WhiteListOrigin, MB, MB_DOT_PAIR, MB_ETH_PAIR,
	Origin, Runtime, System, Tokens,
};

use super::*;

#[test]
fn enable_trading_pair_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DexModule::enable_trading_pair(Origin::signed(ALICE), MB, DOT),
			BadOrigin
		);

		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Disabled
		);
		assert_ok!(DexModule::enable_trading_pair(Origin::signed(WhiteListOrigin::get()), MB, DOT));
		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Enabled
		);

		let enable_trading_pair_event = Event::dex(crate::Event::EnableTradingPair(MB_DOT_PAIR));
		assert!(System::events()
			.iter()
			.any(|record| record.event == enable_trading_pair_event));

		assert_noop!(
			DexModule::enable_trading_pair(Origin::signed(WhiteListOrigin::get()), DOT, MB),
			Error::<Runtime>::MustBeDisabled
		);
	});
}

#[test]
fn new_trading_pair_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DexModule::new_trading_pair(
				Origin::signed(ALICE),
				MB,
				DOT,
				1_000_000_000_000u128,
				1_000_000_000_000u128,
				5_000_000_000_000u128,
				2_000_000_000_000u128,
				10,
			),
			BadOrigin
		);

		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Disabled
		);
		assert_ok!(DexModule::new_trading_pair(
			Origin::signed(WhiteListOrigin::get()),
			MB,
			DOT,
			1_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000u128,
			2_000_000_000_000u128,
			10,
		));
		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Preparing(TradingPairPreparingParameters {
				min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
				target_amount: (5_000_000_000_000u128, 2_000_000_000_000u128),
				accumulated_amount: (0, 0),
				not_before: 10,
			})
		);

		let new_trading_pair_event = Event::dex(crate::Event::NewTradingPair(MB_DOT_PAIR));
		assert!(System::events()
			.iter()
			.any(|record| record.event == new_trading_pair_event));

		assert_noop!(
			DexModule::new_trading_pair(
				Origin::signed(WhiteListOrigin::get()),
				MB,
				DOT,
				1_000_000_000_000u128,
				1_000_000_000_000u128,
				5_000_000_000_000u128,
				2_000_000_000_000u128,
				10,
			),
			Error::<Runtime>::MustBeDisabled
		);
	});
}

#[test]
fn disable_enabled_trading_pair_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(DexModule::enable_trading_pair(
			Origin::signed(WhiteListOrigin::get()),
			MB,
			DOT
		));
		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Enabled
		);

		assert_noop!(
			DexModule::disable_trading_pair(Origin::signed(ALICE), MB, DOT),
			BadOrigin
		);

		assert_ok!(DexModule::disable_trading_pair(
			Origin::signed(WhiteListOrigin::get()),
			MB,
			DOT
		));
		assert_eq!(
			DexModule::trading_pair_statuses(MB_DOT_PAIR),
			TradingPairStatus::<_, _>::Disabled
		);

		let disable_trading_pair_event = Event::dex(crate::Event::DisableTradingPair(MB_DOT_PAIR));
		assert!(System::events()
			.iter()
			.any(|record| record.event == disable_trading_pair_event));

		assert_noop!(
			DexModule::disable_trading_pair(Origin::signed(WhiteListOrigin::get()), MB, DOT),
			Error::<Runtime>::DisabledTradingPair
		);
	});
}

#[test]
fn disable_preparinging_trading_pair_work() {
	ExtBuilder::default()
		.initialize_listing_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				5_000_000_000_000u128,
				0,
				false
			));
			assert_ok!(DexModule::add_liquidity(
				Origin::signed(BOB),
				MB,
				DOT,
				5_000_000_000_000u128,
				1_000_000_000_000u128,
				false
			));

			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_995_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(MB, &BOB), 999_995_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_999_000_000_000_000u128);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				10_000_000_000_000u128
			);
			assert_eq!(
				Tokens::free_balance(DOT, &DexModule::account_id()),
				1_000_000_000_000u128
			);
			assert_eq!(
				DexModule::preparing_pool(MB_DOT_PAIR, ALICE),
				(5_000_000_000_000u128, 0)
			);
			assert_eq!(
				DexModule::preparing_pool(MB_DOT_PAIR, BOB),
				(5_000_000_000_000u128, 1_000_000_000_000u128)
			);
			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Preparing(TradingPairPreparingParameters {
					min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
					target_amount: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
					accumulated_amount: (10_000_000_000_000u128, 1_000_000_000_000u128),
					not_before: 10,
				})
			);
			let alice_ref_count_0 = System::consumers(&ALICE);
			let bob_ref_count_0 = System::consumers(&BOB);

			assert_ok!(DexModule::disable_trading_pair(
				Origin::signed(WhiteListOrigin::get()),
				MB,
				DOT
			));
			assert_eq!(Tokens::free_balance(MB, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 0);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 0);
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, ALICE), (0, 0));
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, BOB), (0, 0));
			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Disabled
			);
			assert_eq!(System::consumers(&ALICE), alice_ref_count_0 - 1);
			assert_eq!(System::consumers(&BOB), bob_ref_count_0 - 1);
		});
}

#[test]
fn add_preparing_work() {
	ExtBuilder::default()
		.initialize_listing_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_noop!(
				DexModule::add_liquidity(
					Origin::signed(ALICE),
					MB,
					DOT,
					4_999_999_999_999u128,
					999_999_999_999u128,
					false
				),
				Error::<Runtime>::InvalidContributionIncrement
			);

			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Preparing(TradingPairPreparingParameters {
					min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
					target_amount: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
					accumulated_amount: (0, 0),
					not_before: 10,
				})
			);
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, ALICE), (0, 0));
			assert_eq!(Tokens::free_balance(MB, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 0);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 0);
			let alice_ref_count_0 = System::consumers(&ALICE);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				5_000_000_000_000u128,
				0,
				false
			));
			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Preparing(TradingPairPreparingParameters {
					min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
					target_amount: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
					accumulated_amount: (5_000_000_000_000u128, 0),
					not_before: 10,
				})
			);
			assert_eq!(
				DexModule::preparing_pool(MB_DOT_PAIR, ALICE),
				(5_000_000_000_000u128, 0)
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_995_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				5_000_000_000_000u128
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 0);
			let alice_ref_count_1 = System::consumers(&ALICE);
			assert_eq!(alice_ref_count_1, alice_ref_count_0 + 1);

			let add_preparing_event_0 =
				Event::dex(crate::Event::AddPreparing(ALICE, MB, 5_000_000_000_000u128, DOT, 0));
			assert!(System::events()
				.iter()
				.any(|record| record.event == add_preparing_event_0));

			// bob add preparing
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, BOB), (0, 0));
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 1_000_000_000_000_000_000u128);
			let bob_ref_count_0 = System::consumers(&BOB);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(BOB),
				DOT,
				MB,
				1_000_000_000_000_000u128,
				0,
				false
			));
			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Preparing(TradingPairPreparingParameters {
					min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
					target_amount: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
					accumulated_amount: (5_000_000_000_000u128, 1_000_000_000_000_000u128),
					not_before: 10,
				})
			);
			assert_eq!(
				DexModule::preparing_pool(MB_DOT_PAIR, BOB),
				(0, 1_000_000_000_000_000u128)
			);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_000_000_000_000_000u128);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				5_000_000_000_000u128
			);
			assert_eq!(
				Tokens::free_balance(DOT, &DexModule::account_id()),
				1_000_000_000_000_000u128
			);
			let bob_ref_count_1 = System::consumers(&BOB);
			assert_eq!(bob_ref_count_1, bob_ref_count_0 + 1);

			let add_preparing_event_1 =
				Event::dex(crate::Event::AddPreparing(BOB, MB, 0, DOT, 1_000_000_000_000_000u128));
			assert!(System::events()
				.iter()
				.any(|record| record.event == add_preparing_event_1));

			// alice add preparing again and trigger trading pair convert to Enabled from
			// Preparing
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_995_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000u128);
			assert_eq!(
				Tokens::total_issuance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap()),
				0
			);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				0
			);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
				0
			);

			System::set_block_number(10);
			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				995_000_000_000_000u128,
				1_000_000_000_000_000u128,
				false
			));
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_000_000_000_000_000u128);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 999_000_000_000_000_000u128);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				1_000_000_000_000_000u128
			);
			assert_eq!(
				Tokens::free_balance(DOT, &DexModule::account_id()),
				2_000_000_000_000_000u128
			);
			assert_eq!(
				Tokens::total_issuance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap()),
				4_000_000_000_000_000u128
			);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				3_000_000_000_000_000u128
			);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
				1_000_000_000_000_000u128
			);
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, ALICE), (0, 0));
			assert_eq!(DexModule::preparing_pool(MB_DOT_PAIR, BOB), (0, 0));
			assert_eq!(
				DexModule::trading_pair_statuses(MB_DOT_PAIR),
				TradingPairStatus::<_, _>::Enabled
			);

			let preparing_to_enabled_event = Event::dex(crate::Event::PreparingToEnabled(
				MB_DOT_PAIR,
				1_000_000_000_000_000u128,
				2_000_000_000_000_000u128,
				4_000_000_000_000_000u128,
			));
			assert!(System::events()
				.iter()
				.any(|record| record.event == preparing_to_enabled_event));
		});
}

#[test]
fn get_liquidity_work() {
	ExtBuilder::default().build().execute_with(|| {
		LiquidityPool::<Runtime>::insert(MB_DOT_PAIR, (1000, 20));
		assert_eq!(DexModule::liquidity_pool(MB_DOT_PAIR), (1000, 20));
		assert_eq!(DexModule::get_liquidity(MB, DOT), (1000, 20));
		assert_eq!(DexModule::get_liquidity(DOT, MB), (20, 1000));
	});
}

#[test]
fn get_target_amount_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(DexModule::get_target_amount(10000, 0, 1000), 0);
		assert_eq!(DexModule::get_target_amount(0, 20000, 1000), 0);
		assert_eq!(DexModule::get_target_amount(10000, 20000, 0), 0);
		assert_eq!(DexModule::get_target_amount(10000, 1, 1000000), 0);
		assert_eq!(DexModule::get_target_amount(10000, 20000, 10000), 9949);
		assert_eq!(DexModule::get_target_amount(10000, 20000, 1000), 1801);
	});
}

#[test]
fn get_supply_amount_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(DexModule::get_supply_amount(10000, 0, 1000), 0);
		assert_eq!(DexModule::get_supply_amount(0, 20000, 1000), 0);
		assert_eq!(DexModule::get_supply_amount(10000, 20000, 0), 0);
		assert_eq!(DexModule::get_supply_amount(10000, 1, 1), 0);
		assert_eq!(DexModule::get_supply_amount(10000, 20000, 9949), 9999);
		assert_eq!(DexModule::get_target_amount(10000, 20000, 9999), 9949);
		assert_eq!(DexModule::get_supply_amount(10000, 20000, 1801), 1000);
		assert_eq!(DexModule::get_target_amount(10000, 20000, 1000), 1801);
	});
}

#[test]
fn get_target_amounts_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Runtime>::insert(MB_DOT_PAIR, (50000, 10000));
			LiquidityPool::<Runtime>::insert(MB_ETH_PAIR, (100000, 10));
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT], 10000, None),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT, MB, ETH, DOT], 10000, None),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT, MB, FIL], 10000, None),
				Error::<Runtime>::MustBeEnabled,
			);
			assert_eq!(
				DexModule::get_target_amounts(&vec![DOT, MB], 10000, None),
				Ok(vec![10000, 24874])
			);
			assert_eq!(
				DexModule::get_target_amounts(&vec![DOT, MB], 10000, Ratio::checked_from_rational(50, 100)),
				Ok(vec![10000, 24874])
			);
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT, MB], 10000, Ratio::checked_from_rational(49, 100)),
				Error::<Runtime>::ExceedPriceImpactLimit,
			);
			assert_eq!(
				DexModule::get_target_amounts(&vec![DOT, MB, ETH], 10000, None),
				Ok(vec![10000, 24874, 1])
			);
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT, MB, ETH], 100, None),
				Error::<Runtime>::ZeroTargetAmount,
			);
			assert_noop!(
				DexModule::get_target_amounts(&vec![DOT, ETH], 100, None),
				Error::<Runtime>::InsufficientLiquidity,
			);
		});
}

#[test]
fn calculate_amount_for_big_number_work() {
	ExtBuilder::default().build().execute_with(|| {
		LiquidityPool::<Runtime>::insert(
			MB_DOT_PAIR,
			(171_000_000_000_000_000_000_000, 56_000_000_000_000_000_000_000),
		);
		assert_eq!(
			DexModule::get_supply_amount(
				171_000_000_000_000_000_000_000,
				56_000_000_000_000_000_000_000,
				1_000_000_000_000_000_000_000,
			),
			3_140_495_867_768_595_041_323
		);
		assert_eq!(
			DexModule::get_target_amount(
				171_000_000_000_000_000_000_000,
				56_000_000_000_000_000_000_000,
				3_140_495_867_768_595_041_323,
			),
			1_000_000_000_000_000_000_000
		);
	});
}

#[test]
fn get_supply_amounts_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Runtime>::insert(MB_DOT_PAIR, (50000, 10000));
			LiquidityPool::<Runtime>::insert(MB_ETH_PAIR, (100000, 10));
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT], 10000, None),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT, MB, ETH, DOT], 10000, None),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT, MB, FIL], 10000, None),
				Error::<Runtime>::MustBeEnabled,
			);
			assert_eq!(
				DexModule::get_supply_amounts(&vec![DOT, MB], 24874, None),
				Ok(vec![10000, 24874])
			);
			assert_eq!(
				DexModule::get_supply_amounts(&vec![DOT, MB], 25000, Ratio::checked_from_rational(50, 100)),
				Ok(vec![10102, 25000])
			);
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT, MB], 25000, Ratio::checked_from_rational(49, 100)),
				Error::<Runtime>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT, MB, ETH], 10000, None),
				Error::<Runtime>::ZeroSupplyAmount,
			);
			assert_noop!(
				DexModule::get_supply_amounts(&vec![DOT, ETH], 10000, None),
				Error::<Runtime>::InsufficientLiquidity,
			);
		});
}

#[test]
fn _swap_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Runtime>::insert(MB_DOT_PAIR, (50000, 10000));

			assert_eq!(DexModule::get_liquidity(MB, DOT), (50000, 10000));
			DexModule::_swap(MB, DOT, 1000, 1000);
			assert_eq!(DexModule::get_liquidity(MB, DOT), (51000, 9000));
			DexModule::_swap(DOT, MB, 100, 800);
			assert_eq!(DexModule::get_liquidity(MB, DOT), (50200, 9100));
		});
}

#[test]
fn _swap_by_path_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Runtime>::insert(MB_DOT_PAIR, (50000, 10000));
			LiquidityPool::<Runtime>::insert(MB_ETH_PAIR, (100000, 10));

			assert_eq!(DexModule::get_liquidity(MB, DOT), (50000, 10000));
			assert_eq!(DexModule::get_liquidity(MB, ETH), (100000, 10));
			DexModule::_swap_by_path(&vec![DOT, MB], &vec![10000, 25000]);
			assert_eq!(DexModule::get_liquidity(MB, DOT), (25000, 20000));
			DexModule::_swap_by_path(&vec![DOT, MB, ETH], &vec![4000, 10000, 2]);
			assert_eq!(DexModule::get_liquidity(MB, DOT), (15000, 24000));
			assert_eq!(DexModule::get_liquidity(MB, ETH), (110000, 8));
		});
}

#[test]
fn add_liquidity_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_noop!(
				DexModule::add_liquidity(Origin::signed(ALICE), FIL, MB, 100_000_000, 100_000_000, false),
				Error::<Runtime>::DisabledTradingPair
			);
			assert_noop!(
				DexModule::add_liquidity(Origin::signed(ALICE), MB, DOT, 0, 100_000_000, false),
				Error::<Runtime>::InvalidLiquidityIncrement
			);

			assert_eq!(DexModule::get_liquidity(MB, DOT), (0, 0));
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 0);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 0);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				0
			);
			assert_eq!(
				Tokens::reserved_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				0
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				5_000_000_000_000,
				1_000_000_000_000,
				false,
			));
			let add_liquidity_event_1 = Event::dex(crate::Event::AddLiquidity(
				ALICE,
				MB,
				5_000_000_000_000,
				DOT,
				1_000_000_000_000,
				5_000_000_000_000,
			));
			assert!(System::events()
				.iter()
				.any(|record| record.event == add_liquidity_event_1));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(5_000_000_000_000, 1_000_000_000_000)
			);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 5_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 1_000_000_000_000);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				5_000_000_000_000
			);
			assert_eq!(
				Tokens::reserved_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				0
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_995_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 999_999_000_000_000_000);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
				0
			);
			assert_eq!(
				Tokens::reserved_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
				0
			);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 1_000_000_000_000_000_000);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(BOB),
				MB,
				DOT,
				50_000_000_000_000,
				8_000_000_000_000,
				true,
			));
			let add_liquidity_event_2 = Event::dex(crate::Event::AddLiquidity(
				BOB,
				MB,
				40_000_000_000_000,
				DOT,
				8_000_000_000_000,
				40_000_000_000_000,
			));
			assert!(System::events()
				.iter()
				.any(|record| record.event == add_liquidity_event_2));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(45_000_000_000_000, 9_000_000_000_000)
			);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 45_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 9_000_000_000_000);
		});
}

#[test]
fn remove_liquidity_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				5_000_000_000_000,
				1_000_000_000_000,
				false
			));
			assert_noop!(
				DexModule::remove_liquidity(
					Origin::signed(ALICE),
					MB_DOT_PAIR.get_dex_share_currency_id().unwrap(),
					DOT,
					100_000_000,
					false,
				),
				Error::<Runtime>::InvalidCurrencyId
			);

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(5_000_000_000_000, 1_000_000_000_000)
			);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 5_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 1_000_000_000_000);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				5_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_995_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 999_999_000_000_000_000);

			assert_ok!(DexModule::remove_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				4_000_000_000_000,
				false,
			));
			let remove_liquidity_event_1 = Event::dex(crate::Event::RemoveLiquidity(
				ALICE,
				MB,
				4_000_000_000_000,
				DOT,
				800_000_000_000,
				4_000_000_000_000,
			));
			assert!(System::events()
				.iter()
				.any(|record| record.event == remove_liquidity_event_1));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(1_000_000_000_000, 200_000_000_000)
			);
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 1_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 200_000_000_000);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				1_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 999_999_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 999_999_800_000_000_000);

			assert_ok!(DexModule::remove_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				1_000_000_000_000,
				false,
			));
			let remove_liquidity_event_2 = Event::dex(crate::Event::RemoveLiquidity(
				ALICE,
				MB,
				1_000_000_000_000,
				DOT,
				200_000_000_000,
				1_000_000_000_000,
			));
			assert!(System::events()
				.iter()
				.any(|record| record.event == remove_liquidity_event_2));

			assert_eq!(DexModule::get_liquidity(MB, DOT), (0, 0));
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 0);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 0);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				0
			);
			assert_eq!(Tokens::free_balance(MB, &ALICE), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000_000_000_000_000);
		});
}

#[test]
fn do_swap_with_exact_supply_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				500_000_000_000_000,
				100_000_000_000_000,
				false,
			));
			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				ETH,
				100_000_000_000_000,
				10_000_000_000,
				false,
			));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(500_000_000_000_000, 100_000_000_000_000)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				600_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 100_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 10_000_000_000);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_000_000_000_000);

			assert_noop!(
				DexModule::do_swap_with_exact_supply(
					&BOB,
					&[DOT, MB],
					100_000_000_000_000,
					250_000_000_000_000,
					None
				),
				Error::<Runtime>::InsufficientTargetAmount
			);
			assert_noop!(
				DexModule::do_swap_with_exact_supply(
					&BOB,
					&[DOT, MB],
					100_000_000_000_000,
					0,
					Ratio::checked_from_rational(10, 100)
				),
				Error::<Runtime>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				DexModule::do_swap_with_exact_supply(&BOB, &[DOT, MB, ETH, DOT], 100_000_000_000_000, 0, None),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::do_swap_with_exact_supply(&BOB, &[DOT, FIL], 100_000_000_000_000, 0, None),
				Error::<Runtime>::MustBeEnabled,
			);

			assert_ok!(DexModule::do_swap_with_exact_supply(
				&BOB,
				&[DOT, MB],
				100_000_000_000_000,
				200_000_000_000_000,
				None
			));
			let swap_event_1 = Event::dex(crate::Event::Swap(
				BOB,
				vec![DOT, MB],
				100_000_000_000_000,
				248_743_718_592_964,
			));
			assert!(System::events().iter().any(|record| record.event == swap_event_1));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(251_256_281_407_036, 200_000_000_000_000)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				351_256_281_407_036
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 200_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 10_000_000_000);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_248_743_718_592_964);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_900_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_000_000_000_000);

			assert_ok!(DexModule::do_swap_with_exact_supply(
				&BOB,
				&[DOT, MB, ETH],
				200_000_000_000_000,
				1,
				None
			));
			let swap_event_2 = Event::dex(crate::Event::Swap(
				BOB,
				vec![DOT, MB, ETH],
				200_000_000_000_000,
				5_530_663_837,
			));
			assert!(System::events().iter().any(|record| record.event == swap_event_2));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(126_259_437_892_983, 400_000_000_000_000)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(224_996_843_514_053, 4_469_336_163)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				351_256_281_407_036
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 400_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 4_469_336_163);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_248_743_718_592_964);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_700_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_005_530_663_837);
		});
}

#[test]
fn do_swap_with_exact_target_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				DOT,
				500_000_000_000_000,
				100_000_000_000_000,
				false,
			));
			assert_ok!(DexModule::add_liquidity(
				Origin::signed(ALICE),
				MB,
				ETH,
				100_000_000_000_000,
				10_000_000_000,
				false,
			));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(500_000_000_000_000, 100_000_000_000_000)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				600_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 100_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 10_000_000_000);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_000_000_000_000);

			assert_noop!(
				DexModule::do_swap_with_exact_target(
					&BOB,
					&[DOT, MB],
					250_000_000_000_000,
					100_000_000_000_000,
					None
				),
				Error::<Runtime>::ExcessiveSupplyAmount
			);
			assert_noop!(
				DexModule::do_swap_with_exact_target(
					&BOB,
					&[DOT, MB],
					250_000_000_000_000,
					200_000_000_000_000,
					Ratio::checked_from_rational(10, 100)
				),
				Error::<Runtime>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				DexModule::do_swap_with_exact_target(
					&BOB,
					&[DOT, MB, ETH, DOT],
					250_000_000_000_000,
					200_000_000_000_000,
					None
				),
				Error::<Runtime>::InvalidTradingPathLength,
			);
			assert_noop!(
				DexModule::do_swap_with_exact_target(&BOB, &[DOT, FIL], 250_000_000_000_000, 200_000_000_000_000, None),
				Error::<Runtime>::MustBeEnabled,
			);

			assert_ok!(DexModule::do_swap_with_exact_target(
				&BOB,
				&[DOT, MB],
				250_000_000_000_000,
				200_000_000_000_000,
				None
			));
			let swap_event_1 = Event::dex(crate::Event::Swap(
				BOB,
				vec![DOT, MB],
				101_010_101_010_102,
				250_000_000_000_000,
			));
			assert!(System::events().iter().any(|record| record.event == swap_event_1));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(250_000_000_000_000, 201_010_101_010_102)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				350_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 201_010_101_010_102);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 10_000_000_000);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_250_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_898_989_898_989_898);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_000_000_000_000);

			assert_ok!(DexModule::do_swap_with_exact_target(
				&BOB,
				&[DOT, MB, ETH],
				5_000_000_000,
				2_000_000_000_000_000,
				None
			));
			let swap_event_2 = Event::dex(crate::Event::Swap(
				BOB,
				vec![DOT, MB, ETH],
				137_654_580_386_993,
				5_000_000_000,
			));
			assert!(System::events().iter().any(|record| record.event == swap_event_2));

			assert_eq!(
				DexModule::get_liquidity(MB, DOT),
				(148_989_898_989_898, 338_664_681_397_095)
			);
			assert_eq!(
				DexModule::get_liquidity(MB, ETH),
				(201_010_101_010_102, 5_000_000_000)
			);
			assert_eq!(
				Tokens::free_balance(MB, &DexModule::account_id()),
				350_000_000_000_000
			);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 338_664_681_397_095);
			assert_eq!(Tokens::free_balance(ETH, &DexModule::account_id()), 5_000_000_000);
			assert_eq!(Tokens::free_balance(MB, &BOB), 1_000_250_000_000_000_000);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 999_761_335_318_602_905);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 1_000_000_005_000_000_000);
		});
}

#[test]
fn initialize_added_liquidity_pools_genesis_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.initialize_added_liquidity_pools(ALICE)
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_eq!(DexModule::get_liquidity(MB, DOT), (1000000, 2000000));
			assert_eq!(Tokens::free_balance(MB, &DexModule::account_id()), 2000000);
			assert_eq!(Tokens::free_balance(DOT, &DexModule::account_id()), 3000000);
			assert_eq!(
				Tokens::free_balance(MB_DOT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
				2000000
			);
		});
}
