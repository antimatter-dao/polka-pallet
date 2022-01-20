#![cfg(test)]

use frame_support::{
	construct_runtime,
	ord_parameter_types, parameter_types,
};
use frame_system::EnsureSignedBy;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

use model::{Amount, CurrencyId};

use super::*;

pub type AccountId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const MB: CurrencyId = CurrencyId::Token(0);
pub const DOT: CurrencyId = CurrencyId::Token(1);
pub const BTC: CurrencyId = CurrencyId::Token(3);

mod incentives {
	pub use super::super::*;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
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
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
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

impl orml_rewards::Config for Runtime {
	type Share = Balance;
	type Balance = Balance;
	type PoolId = PoolId;
	type Handler = IncentivesModule;
	type WeightInfo = ();
}

parameter_types! {
	pub const LoansIncentivePool: AccountId = 10;
	pub const AccumulatePeriod: BlockNumber = 10;
	pub const IncentiveCurrencyId: CurrencyId = MB;
}

ord_parameter_types! {
	pub const Four: AccountId = 4;
}

impl Config for Runtime {
	type Event = Event;
	type LoansIncentivePool = LoansIncentivePool;
	type AccumulatePeriod = AccumulatePeriod;
	type IncentiveCurrencyId = IncentiveCurrencyId;
	type WhiteListOrigin = EnsureSignedBy<Four, AccountId>;
	type Currency = TokensModule;
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
		IncentivesModule: incentives::{Module, Storage, Call, Event<T>},
		TokensModule: orml_tokens::{Module, Storage, Event<T>, Config<T>},
		RewardsModule: orml_rewards::{Module, Storage, Call},
	}
);

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		t.into()
	}
}
