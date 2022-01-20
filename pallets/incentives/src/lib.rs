#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use orml_traits::{Happened, MultiCurrency, RewardHandler};
use sp_runtime::{
	traits::Zero,
};
use sp_std::prelude::*;

use model::{Amount, Balance, CurrencyId};
pub use module::*;

mod mock;
mod test;

pub type PoolId = CurrencyId;

// TODO: adapt dex
#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config:
	frame_system::Config + orml_rewards::Config<Share=Balance, Balance=Balance, PoolId=PoolId>
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type LoansIncentivePool: Get<Self::AccountId>;

		#[pallet::constant]
		type IncentiveCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		type AccumulatePeriod: Get<Self::BlockNumber>;

		type WhiteListOrigin: EnsureOrigin<Self::Origin>;

		type Currency: MultiCurrency<Self::AccountId, CurrencyId=CurrencyId, Balance=Balance>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnough,

		InvalidCurrencyId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ClaimRewards(T::AccountId, T::PoolId),
	}

	#[pallet::storage]
	#[pallet::getter(fn loans_incentive_rewards)]
	pub type LoansIncentiveRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(100000)]
		pub fn claim_rewards(origin: OriginFor<T>, pool_id: T::PoolId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			<orml_rewards::Module<T>>::claim_rewards(&who, pool_id);
			Self::deposit_event(Event::ClaimRewards(who, pool_id));
			Ok(().into())
		}

		#[transactional]
		#[pallet::weight(100000)]
		pub fn update_loans_incentive_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::WhiteListOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				LoansIncentiveRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		#[allow(clippy::type_complexity)]
		pub loans_incentive_rewards_params: Vec<(CurrencyId, Balance)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {
				loans_incentive_rewards_params: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			self.loans_incentive_rewards_params.iter().for_each(
				|(currency_id, balance)| {
					LoansIncentiveRewards::<T>::insert(currency_id, balance);
				}
			);
		}
	}
}

pub struct OnUpdateLoan<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Happened<(T::AccountId, CurrencyId, Amount, Balance)> for OnUpdateLoan<T> {
	fn happened(info: &(T::AccountId, CurrencyId, Amount, Balance)) {
		let (who, currency_id, adjustment, previous_amount) = info;
		let adjustment_abs =
			sp_std::convert::TryInto::<Balance>::try_into(adjustment.saturating_abs()).unwrap_or_default();

		if !adjustment_abs.is_zero() {
			let new_share_amount = if adjustment.is_positive() {
				previous_amount.saturating_add(adjustment_abs)
			} else {
				previous_amount.saturating_sub(adjustment_abs)
			};

			<orml_rewards::Module<T>>::set_share(who, *currency_id, new_share_amount);
		}
	}
}

impl<T: Config> RewardHandler<T::AccountId, T::BlockNumber> for Pallet<T> {
	type Share = Balance;
	type Balance = Balance;
	type PoolId = PoolId;
	type CurrencyId = CurrencyId;

	fn accumulate_reward(now: T::BlockNumber, mut callback: impl FnMut(PoolId, Balance)) -> Vec<(CurrencyId, Balance)> {
		let mut accumulated_rewards: Vec<(CurrencyId, Balance)> = vec![];

		if now % T::AccumulatePeriod::get() == Zero::zero() {
			let mut accumulated_incentive: Balance = Zero::zero();
			let incentive_currency_id = T::IncentiveCurrencyId::get();

			for (pool_id, pool_info) in orml_rewards::Pools::<T>::iter() {
				if !pool_info.total_shares.is_zero() {
					let incentive_reward = Self::loans_incentive_rewards(pool_id);
					debug::info!(target: "incentives debug 1:", "incentive_reward: {:?}", incentive_reward);

					// TODO: transfer from RESERVED TREASURY instead of issuing
					if !incentive_reward.is_zero()
						&& T::Currency::deposit(incentive_currency_id, &T::LoansIncentivePool::get(), incentive_reward).is_ok()
					{
						callback(pool_id, incentive_reward);
						debug::info!(target: "incentives debug 2:", "pool_id:{:?}, pool_info: {:?}", pool_id, pool_info);
						accumulated_incentive = accumulated_incentive.saturating_add(incentive_reward);
					}
				}
			}

			if !accumulated_incentive.is_zero() {
				accumulated_rewards.push((incentive_currency_id, accumulated_incentive));
			}
		}

		accumulated_rewards
	}

	fn payout(who: &T::AccountId, _pool_id: PoolId, amount: Balance) {
		let (pool_account, currency_id) = (T::LoansIncentivePool::get(), T::IncentiveCurrencyId::get());

		// payout the reward to user from the pool. it should not affect the
		// process, ignore the result to continue. if it fails, just the user will not
		// be rewarded, there will not increase user balance.
		let _ = T::Currency::transfer(currency_id, &pool_account, &who, amount);
	}
}
