use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_runtime::traits::SaturatedConversion;
use sp_std::prelude::*;

use crate::{AccountId, Balance, CurrencyId, Tokens, AssetPool, DOT, Runtime};

use super::utils::set_balance;

runtime_benchmarks! {
	{ Runtime, pb }

	_ {}

	adjust_deposit {
		let caller: AccountId = account("caller", 0, 0);
		let _ = Tokens::update_balance(DOT, &caller, 10i128);
	}: _(RawOrigin::Signed(caller), DOT, 10i128)

	adjust_debit {
		let caller = account("caller", 0, 0);
		let _ = Tokens::update_balance(DOT, &caller, 1000i128);
		AssetPool::update_deposit(&caller, DOT, 100i128);
	}: _(RawOrigin::Signed(caller), DOT, 10i128)

}
