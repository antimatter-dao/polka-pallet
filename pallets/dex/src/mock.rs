//! Mocks for the dex module.

#![cfg(test)]

use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use orml_traits::{ parameter_type_with_key};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

use model::Amount;

use super::*;

pub type BlockNumber = u64;
pub type AccountId = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
// order is important for unit tests
pub const MB: CurrencyId = CurrencyId::Token(1);
pub const ETH: CurrencyId = CurrencyId::Token(4);
pub const DOT: CurrencyId = CurrencyId::Token(2);
pub const FIL: CurrencyId = CurrencyId::Token(0);
pub const MB_ETH_PAIR: TradingPair = TradingPair(MB, ETH);
pub const MB_DOT_PAIR: TradingPair = TradingPair(MB, DOT);
pub const DOT_ETH_PAIR: TradingPair = TradingPair(DOT, ETH);

mod dex {
	pub use super::super::*;
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

ord_parameter_types! {
	pub const WhiteListOrigin: AccountId = 3;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (1, 100);
	pub const TradingPathLimit: u32 = 3;
	pub const DEXModuleId: ModuleId = ModuleId(*b"antimatterex");
}

impl Config for Runtime {
	type Event = Event;
	type Currency = Tokens;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type ModuleId = DEXModuleId;
	type WhiteListOrigin = EnsureSignedBy<WhiteListOrigin, AccountId>;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		DexModule: dex::{Module, Storage, Call, Event<T>, Config<T>},
		Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
	}
);

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	initial_preparing_trading_pairs: Vec<(TradingPair, (Balance, Balance), (Balance, Balance), BlockNumber)>,
	initial_enabled_trading_pairs: Vec<TradingPair>,
	initial_liquidity_pools: Vec<(AccountId, Vec<(TradingPair, (Balance, Balance))>)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				(ALICE, MB, 1_000_000_000_000_000_000u128),
				(BOB, MB, 1_000_000_000_000_000_000u128),
				(ALICE, ETH, 1_000_000_000_000_000_000u128),
				(BOB, ETH, 1_000_000_000_000_000_000u128),
				(ALICE, DOT, 1_000_000_000_000_000_000u128),
				(BOB, DOT, 1_000_000_000_000_000_000u128),
			],
			initial_preparing_trading_pairs: vec![],
			initial_enabled_trading_pairs: vec![],
			initial_liquidity_pools: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn initialize_listing_trading_pairs(mut self) -> Self {
		self.initial_preparing_trading_pairs = vec![
			(
				MB_DOT_PAIR,
				(5_000_000_000_000u128, 1_000_000_000_000u128),
				(5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
				10,
			),
			(
				MB_ETH_PAIR,
				(20_000_000_000_000u128, 1_000_000_000u128),
				(20_000_000_000_000_000u128, 1_000_000_000_000u128),
				10,
			),
			(
				DOT_ETH_PAIR,
				(4_000_000_000_000u128, 1_000_000_000u128),
				(4_000_000_000_000_000u128, 1_000_000_000_000u128),
				20,
			),
		];
		self
	}

	pub fn initialize_enabled_trading_pairs(mut self) -> Self {
		self.initial_enabled_trading_pairs = vec![MB_DOT_PAIR, MB_ETH_PAIR, DOT_ETH_PAIR];
		self
	}

	pub fn initialize_added_liquidity_pools(mut self, who: AccountId) -> Self {
		self.initial_liquidity_pools = vec![(
			who,
			vec![
				(MB_DOT_PAIR, (1_000_000u128, 2_000_000u128)),
				(MB_ETH_PAIR, (1_000_000u128, 2_000_000u128)),
				(DOT_ETH_PAIR, (1_000_000u128, 2_000_000u128)),
			],
		)];
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
			.assimilate_storage(&mut t)
			.unwrap();

		dex::GenesisConfig::<Runtime> {
			initial_preparing_trading_pairs: self.initial_preparing_trading_pairs,
			initial_enabled_trading_pairs: self.initial_enabled_trading_pairs,
			initial_liquidity_pools: self.initial_liquidity_pools,
		}
			.assimilate_storage(&mut t)
			.unwrap();

		t.into()
	}
}
