use crate::{
	AccountId, Balance, BlockNumber, Tokens, CurrencyId, Runtime,
	TradingPair, DOT, ETH, BTC, Ratio, DEX
};

use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrencyExtended;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

const SEED: u32 = 0;

fn inject_liquidity(
	maker: AccountId,
	currency_id_a: CurrencyId,
	currency_id_b: CurrencyId,
	max_amount_a: Balance,
	max_amount_b: Balance,
	deposit: bool,
) -> Result<(), &'static str> {
	// set balance
	Tokens::update_balance(currency_id_a, &maker, max_amount_a.unique_saturated_into())?;
	Tokens::update_balance(currency_id_b, &maker, max_amount_b.unique_saturated_into())?;

	let _ = DEX::enable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);

	DEX::add_liquidity(
		RawOrigin::Signed(maker.clone()).into(),
		currency_id_a, currency_id_b,
		max_amount_a, max_amount_b,
		deposit,
	)?;

	Ok(())
}

runtime_benchmarks! {
	{ Runtime, dex }

	_ {}

	swap_with_exact_supply {
		let maker: AccountId = account("maker", 0, SEED);
		let taker: AccountId = account("taker", 0, SEED);
		inject_liquidity(maker, DOT, ETH, 10_000u128, 10_000u128, false)?;

		Tokens::update_balance(DOT, &taker, (10_000u128).unique_saturated_into())?;
		let mut route: Vec<CurrencyId> = vec![DOT, ETH];
	}: swap_with_exact_supply(RawOrigin::Signed(taker), route, 100u128, 0, Ratio::saturating_from_rational(1, 1))

	swap_with_exact_target {
		let maker: AccountId = account("maker", 0, SEED);
		let taker: AccountId = account("taker", 0, SEED);
		inject_liquidity(maker, DOT, ETH, 10_000u128, 10_000u128, false)?;

		Tokens::update_balance(DOT, &taker, (10_000u128).unique_saturated_into())?;
		let mut route: Vec<CurrencyId> = vec![DOT, ETH];
	}: swap_with_exact_target(RawOrigin::Signed(taker), route, 10u128, 100u128, Ratio::saturating_from_rational(1, 1))

	add_liquidity {
		let first_maker: AccountId = account("first_maker", 0, SEED);
		let second_maker: AccountId = account("second_maker", 0, SEED);
		let trading_pair = TradingPair::new(DOT, ETH);
		let amount_a = 100u128;
		let amount_b = 10_000u128;

		Tokens::update_balance(DOT, &second_maker, amount_a.unique_saturated_into())?;
		Tokens::update_balance(ETH, &second_maker, amount_b.unique_saturated_into())?;

		inject_liquidity(first_maker.clone(), DOT, ETH, amount_a, amount_b, false)?;
	}: add_liquidity(RawOrigin::Signed(second_maker), DOT, ETH, amount_a, amount_b, false)

	remove_liquidity {
		let maker: AccountId = account("maker", 0, SEED);
		let trading_pair = TradingPair::new(DOT, ETH);
		inject_liquidity(maker.clone(), DOT, ETH, 100u128, 10_000u128, false)?;
	}: remove_liquidity(RawOrigin::Signed(maker), DOT, ETH, 50u128, false)

	enable_trading_pair {
		let trading_pair = TradingPair::new(DOT, ETH);
		let currency_id_a = DOT;
		let currency_id_b = ETH;
		let _ = DEX::disable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
	}: _(RawOrigin::Root, currency_id_a, currency_id_b)

	disable_trading_pair {
		let trading_pair = TradingPair::new(DOT, ETH);
		let currency_id_a = DOT;
		let currency_id_b = ETH;
		let _ = DEX::enable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
	}: _(RawOrigin::Root, currency_id_a, currency_id_b)

	new_trading_pair {
		let currency_id_a = ETH;
		let currency_id_b = BTC;
		let min_contribution_a = 0u128;
		let min_contribution_b = 0u128;
		let target_amount_a = 200u128;
		let target_amount_b = 1_000u128;
		let not_before: BlockNumber = Default::default();
		let _ = DEX::disable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
	}: _(RawOrigin::Root, currency_id_a, currency_id_b, min_contribution_a, min_contribution_b, target_amount_a, target_amount_b, not_before)

}
