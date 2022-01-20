#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{pallet_prelude::*, transactional};
use orml_traits::{Happened, MultiCurrency, MultiCurrencyExtended};
use orml_utilities::{IterableStorageDoubleMapExtended, OffchainErr};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use sp_runtime::{
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, Time},
		Duration,
	},
	DispatchResult, RandomNumberGenerator,
	FixedPointNumber, ModuleId, RuntimeDebug,
	traits::{AccountIdConversion, Saturating, Zero, StaticLookup, BlakeTwo256, Hash},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
};
use sp_std::{convert::TryInto, result, vec::Vec};

use model::{Amount, Balance, CurrencyId, Ratio};

pub use module::*;

mod mock;
mod test;


pub const OFFCHAIN_WORKER_DATA: &[u8] = b"antimatter/liquidation/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"antimatter/liquidation/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"antimatter/liquidation/max-iterations/";
pub const LOCK_DURATION: u64 = 100;
pub const DEFAULT_MAX_ITERATIONS: u32 = 1000;


#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LoanInfo {
	// real deposit: deposit * deposit_rate
	pub deposit: Balance,

	// real debit: debit * debit_rate
	pub debit: Balance,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InterestInfo {
	pub critical_point: Ratio,
	pub base: Ratio,
	pub slope_1: Ratio,
	pub slope_2: Ratio,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetPoolInfo {
	pub maximum_total_debit_ratio: Ratio,

	pub minimum_deposit: Balance,

	pub minimum_debit: Balance,

	// health ratio > liquidation ratio
	pub health_ratio: Ratio,

	pub total_deposit: Balance,

	pub total_debit: Balance,

	pub deposit_rate: Ratio,

	pub debit_rate: Ratio,

	pub deposit_apy: Ratio,

	pub debit_apy: Ratio,

	pub reserve_factor: Ratio,

	pub interest_info: InterestInfo,
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId=CurrencyId, Balance=Balance, Amount=Amount>;

		#[pallet::constant]
		type ModuleId: Get<ModuleId>;

		#[pallet::constant]
		type AssetPoolIds: Get<Vec<CurrencyId>>;

		#[pallet::constant]
		type BlockPercentEachYear: Get<Ratio>;

		type OnUpdateLoan: Happened<(Self::AccountId, CurrencyId, Amount, Balance)>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::storage]
	#[pallet::getter(fn asset_pool_infos)]
	pub type AssetPoolInfos<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, AssetPoolInfo, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn loans)]
	pub type LoanInfos<T: Config> = StorageDoubleMap<_, Twox64Concat, CurrencyId, Twox64Concat, T::AccountId, LoanInfo, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		#[allow(clippy::type_complexity)]
		pub asset_pool_params: Vec<(CurrencyId, AssetPoolInfo)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {
				asset_pool_params: vec![],
			}
		}
	}

	#[cfg(feature = "std")]
	impl GenesisConfig {
		pub fn build_storage<T: Config>(&self) -> Result<sp_runtime::Storage, String> {
			<Self as frame_support::traits::GenesisBuild<T>>::build_storage(self)
		}

		pub fn assimilate_storage<T: Config>(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
			<Self as frame_support::traits::GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::liquidate(currency_id, who) => {
					let account = T::Lookup::lookup(who.clone())?;
					// TODO: check unsafe
					if !Self::is_debit_unsafe() {
						return InvalidTransaction::Stale.into();
					}
					ValidTransaction::with_tag_prefix("AssetPoolLiquidationOffchainWorker")
						.priority(T::UnsignedPriority::get())
						.and_provides((currency_id, who))
						.longevity(64_u64)
						.propagate(true)
						.build()
				}
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			self.asset_pool_params.iter().for_each(
				|(asset_pool_id, asset_pool_param)| {
					AssetPoolInfos::<T>::insert(asset_pool_id, asset_pool_param);
				}
			);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_finalize(_now: T::BlockNumber) {
			for asset_pool_id in T::AssetPoolIds::get() {
				let mut asset_pool_info: AssetPoolInfo = Self::asset_pool_infos(asset_pool_id);
				let total_debit = asset_pool_info.total_debit;
				let total_deposit = asset_pool_info.total_deposit;
				if total_debit.is_zero() || total_deposit.is_zero() {
					continue;
				}

				let utilisation_rate = Ratio::saturating_from_rational(total_debit, total_deposit);
				asset_pool_info.debit_apy = Self::calculate_debit_apy(utilisation_rate, asset_pool_info.interest_info);
				asset_pool_info.deposit_apy = asset_pool_info.debit_apy
					.saturating_mul(asset_pool_info.reserve_factor)
					.saturating_mul(utilisation_rate);

				let loan_increment = T::BlockPercentEachYear::get().saturating_mul(asset_pool_info.debit_apy);

				asset_pool_info.debit_rate = asset_pool_info.debit_rate.saturating_add(loan_increment);
				asset_pool_info.deposit_rate = asset_pool_info.deposit_rate
					.saturating_add(loan_increment.saturating_mul(asset_pool_info.reserve_factor));

				AssetPoolInfos::<T>::insert(asset_pool_id, asset_pool_info);
			}
		}

		/// Runs after every block. Check debit-ratio and submit unsigned tx to trigger liquidation.
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(e) = Self::_offchain_worker() {
				debug::info!(target: "liquidation off chain worker", "block number:{:?}", now);
			}
		}

	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000_000)]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;
			Self::liquidate_unsafe_debit(who, currency_id)?;
			Ok(().into())
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		DepositOverflow,

		DepositTooLow,

		DebitOverflow,

		DebitTooLow,

		AmountConvertFailed,

		DepositNotEnough,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		DepositUpdated(T::AccountId, CurrencyId, Amount),
		DebitUpdated(T::AccountId, CurrencyId, Amount),
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn calculate_debit_apy(utilisation_rate: Ratio, interest_info: InterestInfo) -> Ratio {
		let critical_point = interest_info.critical_point;
		if utilisation_rate < critical_point {
			interest_info.base + (interest_info.slope_1) * (utilisation_rate / critical_point)
		} else {
			interest_info.base + interest_info.slope_1 + interest_info.slope_2 *
				((utilisation_rate - critical_point) / (Ratio::from(1) - critical_point))
		}
	}

	#[transactional]
	pub fn update_deposit(who: &T::AccountId, asset_pool_id: CurrencyId, deposit_adjustment: Amount) -> DispatchResult {
		Self::_update_deposit(who, asset_pool_id, deposit_adjustment)?;
		let deposit_balance_adjustment = Self::balance_try_from_amount_abs(deposit_adjustment)?;
		let module_account = Self::account_id();

		// check personal deposit amount
		if deposit_adjustment.is_positive() {
			T::Currency::transfer(asset_pool_id, who, &module_account, deposit_balance_adjustment)?;
		} else if deposit_adjustment.is_negative() {
			T::Currency::transfer(asset_pool_id, &module_account, who, deposit_balance_adjustment)?;
		}

		Self::deposit_event(Event::DepositUpdated(who.clone(), asset_pool_id, deposit_adjustment));
		Ok(())
	}

	fn _update_deposit(who: &T::AccountId, asset_pool_id: CurrencyId, deposit_adjustment: Amount) -> DispatchResult {
		let deposit_balance = Self::balance_try_from_amount_abs(deposit_adjustment)?;

		AssetPoolInfos::<T>::try_mutate(asset_pool_id, |asset_pool_info| -> DispatchResult {
			asset_pool_info.total_deposit = if deposit_adjustment.is_positive() {
				asset_pool_info.total_deposit.checked_add(deposit_balance).ok_or(Error::<T>::DepositOverflow)
			} else {
				asset_pool_info.total_deposit.checked_sub(deposit_balance).ok_or(Error::<T>::DepositTooLow)
			}?;

			<LoanInfos<T>>::try_mutate_exists(asset_pool_id, who, |loan| -> DispatchResult{
				let mut l = loan.take().unwrap_or_default();

				let new_deposit = if deposit_adjustment.is_positive() {
					let increase = asset_pool_info.deposit_rate.reciprocal().unwrap_or_default().saturating_mul_int(deposit_balance);
					l.deposit.checked_add(increase).ok_or(Error::<T>::DepositOverflow)
				} else {
					let decrease = asset_pool_info.deposit_rate.reciprocal().unwrap_or_default().saturating_mul_int(deposit_balance);
					l.deposit.checked_sub(decrease).ok_or(Error::<T>::DepositTooLow)
				}?;
				// TODO
				// ensure!(new_deposit >= asset_pool_info.minimum_deposit, Error::<T>::DepositTooLow);
				T::OnUpdateLoan::happened(&(who.clone(), asset_pool_id, deposit_adjustment, l.deposit));
				l.deposit = new_deposit;

				if l.debit.is_zero() && l.deposit.is_zero() {
					*loan = None;
				} else {
					*loan = Some(l);
				}

				Ok(())
			})?;

			Ok(())
		})
	}

	#[transactional]
	pub fn update_debit(who: &T::AccountId, asset_pool_id: CurrencyId, debit_adjustment: Amount) -> DispatchResult {
		Self::_update_debit(who, asset_pool_id, debit_adjustment)?;
		let debit_balance_adjustment = Self::balance_try_from_amount_abs(debit_adjustment)?;
		let module_account = Self::account_id();

		if debit_adjustment.is_positive() {
			T::Currency::transfer(asset_pool_id, &module_account, who, debit_balance_adjustment)?;
		} else if debit_adjustment.is_negative() {
			T::Currency::transfer(asset_pool_id, who, &module_account, debit_balance_adjustment)?;
		}

		Self::deposit_event(Event::DebitUpdated(who.clone(), asset_pool_id, debit_adjustment));
		Ok(())
	}

	fn _update_debit(who: &T::AccountId, asset_pool_id: CurrencyId, debit_adjustment: Amount) -> DispatchResult {
		let debit_balance = Self::balance_try_from_amount_abs(debit_adjustment)?;

		AssetPoolInfos::<T>::try_mutate(asset_pool_id, |asset_pool_info| -> DispatchResult {
			let total_debit = asset_pool_info.total_debit;
			let total_deposit = asset_pool_info.total_deposit;

			ensure!(!total_deposit.is_zero(), Error::<T>::DepositNotEnough);
			let new_total_debit = if debit_adjustment.is_positive() {
				total_debit.checked_add(debit_balance).ok_or(Error::<T>::DebitOverflow)
			} else {
				total_debit.checked_sub(debit_balance).ok_or(Error::<T>::DebitTooLow)
			}?;

			// println!("maximum_total_debit_ratio: {:?}, new_total_debit: {:?}, total_deposit: {:?}",
			// 			 asset_pool_info.maximum_total_debit_ratio, new_total_debit, total_deposit);
			// check new_total_debit / total_deposit <= maximum_total_debit_ratio
			ensure!(Ratio::saturating_from_rational(new_total_debit, total_deposit)
					<= asset_pool_info.maximum_total_debit_ratio, Error::<T>::DepositNotEnough);
			asset_pool_info.total_debit = new_total_debit;

			<LoanInfos<T>>::try_mutate_exists(asset_pool_id, who, |loan| -> DispatchResult{
				let mut l = loan.take().unwrap_or_default();

				let new_debit = if debit_adjustment.is_positive() {
					let increase = asset_pool_info.debit_rate.reciprocal().unwrap().saturating_mul_int(debit_balance);
					l.debit.checked_add(increase).ok_or(Error::<T>::DebitOverflow)
				} else {
					// debit balance > personal debit
					let decrease = asset_pool_info.debit_rate.reciprocal().unwrap().saturating_mul_int(debit_balance);
					l.debit.checked_sub(decrease).ok_or(Error::<T>::DebitTooLow)
				}?;

				// TODO how to handle this corner in front-end (force add?)
				// ensure!(new_debit >= asset_pool_info.minimum_debit, Error::<T>::DebitTooLow);
				// TODO check health ratio(need to depend PriceManager)

				T::OnUpdateLoan::happened(&(who.clone(), asset_pool_id, debit_adjustment, l.debit));
				l.debit = new_debit;

				if l.debit.is_zero() && l.deposit.is_zero() {
					*loan = None;
				} else {
					*loan = Some(l);
				}

				Ok(())
			})?;

			Ok(())
		})
	}

	fn balance_try_from_amount_abs(a: Amount) -> result::Result<Balance, Error<T>> {
		TryInto::<Balance>::try_into(a.saturating_abs()).map_err(|_| Error::<T>::AmountConvertFailed)
	}


}

impl<T: Config> Pallet<T> {
	// TODO: call dex
	pub fn liquidate_unsafe_debit(who: T::AccountId, currency_id: CurrencyId) -> DispatchResult {
		Ok(())
	}

	fn submit_unsigned_liquidation_tx(currency_id: CurrencyId, who: T::AccountId) {
		let who = T::Lookup::unlookup(who);
		let call = Call::<T>::liquidate(currency_id, who.clone());
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			debug::info!("failed");
		}
	}

	fn _offchain_worker() -> Result<(), OffchainErr> {
		let asset_pool_ids = T::AssetPoolIds::get();
		if asset_pool_ids.len().is_zero() {
			return Ok(());
		}

		// check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let to_be_continue = StorageValueRef::persistent(&OFFCHAIN_WORKER_DATA);

		// get to_be_continue record
		let (asset_pool_id, start_key) =
			if let Some(Some((last_asset_pool_id, maybe_last_iterator_previous_key)))= to_be_continue.get::<(u32, Option<Vec<u8>>)>()
			{
				(last_asset_pool_id, maybe_last_iterator_previous_key)
			} else {
				let random_seed = sp_io::offchain::random_seed();
				let mut rng = RandomNumberGenerator::<BlakeTwo256>::new(BlakeTwo256::hash(&random_seed[..]));
				(rng.pick_u32(asset_pool_ids.len().saturating_sub(1) as u32), None)
			};

		let max_iterations = StorageValueRef::persistent(&OFFCHAIN_WORKER_MAX_ITERATIONS)
			.get::<u32>()
			.unwrap_or(Some(DEFAULT_MAX_ITERATIONS));

		let currency_id = asset_pool_ids[(asset_pool_id as usize)];
		let mut map_iterator =  <LoanInfos<T> as
		IterableStorageDoubleMapExtended<_, _, _>>::iter_prefix(currency_id, max_iterations, start_key.clone());

		let mut iteration_count = 0;
		while let Some(((who, LoanInfo {deposit, debit}))) = map_iterator.next() {
			// if Self::is_debit_unsafe() {
			// 	Self::submit_unsigned_liquidation_tx(currency_id, who);
			// }

			iteration_count += 1;

			guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
		}

		if map_iterator.finished {
			let next_asset_pool_id =
				if asset_pool_id < asset_pool_ids.len().saturating_sub(1) as u32 {
					asset_pool_id + 1
				} else {
					0
				};
			to_be_continue.set(&(next_asset_pool_id, Option::<Vec<u8>>::None));
		} else {
			to_be_continue.set(&(asset_pool_id, Some(map_iterator.map_iterator.previous_key)));
		}

		guard.forget();

		Ok(())
	}

	// TODO: impl unsafe check
	pub fn is_debit_unsafe() -> bool {
		return true;
	}
}
