//! Mocks for the loans module.

#![cfg(test)]

use frame_support::{construct_runtime, parameter_types};
use frame_support::{pallet_prelude::*};
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{ModuleId, testing::Header, testing::TestXt, traits::IdentityLookup};
use sp_runtime::FixedPointNumber;

use model::Ratio;

use super::*;

pub type AccountId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const DOT: CurrencyId = CurrencyId::Token(1);
pub const BTC: CurrencyId = CurrencyId::Token(3);

mod loans {
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

parameter_types! {
	pub const ExistentialDeposit: Balance = 0;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type MaxLocks = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const LoansModuleId: ModuleId = ModuleId(*b"antimatterdc");
	pub AssetPoolIds: Vec<CurrencyId> = vec![DOT];
	pub BlockPercentEachYear: Ratio = Ratio::one();
	pub const UnsignedPriority: u64 = 1 << 20;
}

pub type Extrinsic = TestXt<Call, ()>;
impl<LocalCall> SendTransactionTypes<LocalCall> for Runtime
	where
		Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl Config for Runtime {
	type Event = Event;
	type Currency = Tokens;
	type ModuleId = LoansModuleId;

	type AssetPoolIds = AssetPoolIds;
	type BlockPercentEachYear = BlockPercentEachYear;
	type UnsignedPriority = UnsignedPriority;

	type OnUpdateLoan = ();
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
		LoansModule: loans::{Module, Storage, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
	}
);

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	asset_pool_params: Vec<(CurrencyId, AssetPoolInfo)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				(ALICE, DOT, 1000),
				(ALICE, BTC, 1000),
				(BOB, DOT, 1000),
				(BOB, BTC, 1000),
			],
			asset_pool_params: vec![
				(DOT, AssetPoolInfo {
					maximum_total_debit_ratio: Ratio::saturating_from_rational(90, 100),
					minimum_deposit: 10u128.pow(1),
					minimum_debit: 10u128.pow(1),
					health_ratio: Ratio::saturating_from_rational(75, 100),
					total_deposit: 0,
					total_debit: 0,
					deposit_rate: Ratio::saturating_from_rational(100, 100),
					debit_rate: Ratio::saturating_from_rational(100, 100),
					deposit_apy: Ratio::saturating_from_rational(0, 100),
					debit_apy: Ratio::saturating_from_rational(0, 100),
					reserve_factor: Ratio::saturating_from_rational(90, 100),
					interest_info: InterestInfo {
						critical_point: Ratio::saturating_from_rational(90, 100),
						base: Ratio::saturating_from_rational(0, 100),
						slope_1: Ratio::saturating_from_rational(4, 100),
						slope_2: Ratio::saturating_from_rational(100, 100),
					},
				}),
			]
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}.assimilate_storage(&mut t).unwrap();

		loans::GenesisConfig {
			asset_pool_params: self.asset_pool_params,
		}.assimilate_storage::<Runtime>(&mut t).unwrap();

		t.into()
	}
}
