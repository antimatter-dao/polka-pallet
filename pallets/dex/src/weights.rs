#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{constants::RocksDbWeight, Weight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
	fn enable_trading_pair() -> Weight;
	fn disable_trading_pair() -> Weight;
	fn new_trading_pair() -> Weight;
	fn add_liquidity() -> Weight;
	fn remove_liquidity() -> Weight;
	fn swap_with_exact_supply() -> Weight;
	fn swap_with_exact_target() -> Weight;
}

impl WeightInfo for () {
	fn enable_trading_pair() -> Weight {
		(10_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn disable_trading_pair() -> Weight {
		(10_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn new_trading_pair() -> Weight {
		(10_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn add_liquidity() -> Weight {
		(100_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(8 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	fn remove_liquidity() -> Weight {
		(200_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	fn swap_with_exact_supply() -> Weight {
		(100_000_000 as Weight)
			.saturating_add((400_000 as Weight).saturating_mul(1 as Weight))
			.saturating_add(RocksDbWeight::get().reads(10 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn swap_with_exact_target() -> Weight {
		(100_000_000 as Weight)
			.saturating_add((100_000 as Weight).saturating_mul(1 as Weight))
			.saturating_add(RocksDbWeight::get().reads(10 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
}
