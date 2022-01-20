#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;

use model::{Amount, CurrencyId};
pub use weights::WeightInfo;
pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + asset_pool::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		AdjustDepositSuccess(T::AccountId, CurrencyId, Amount),
		AdjustDebitSuccess(T::AccountId, CurrencyId, Amount),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((<T as Config>::WeightInfo::adjust_deposit(), DispatchClass::Operational))]
		#[transactional]
		pub fn adjust_deposit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			deposit_adjustment_amount: Amount,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			<asset_pool::Module<T>>::update_deposit(&who, currency_id, deposit_adjustment_amount)?;
			Self::deposit_event(Event::AdjustDepositSuccess(who.clone(), currency_id, deposit_adjustment_amount));
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::adjust_debit(), DispatchClass::Operational))]
		#[transactional]
		pub fn adjust_debit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			debit_adjustment_amount: Amount,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			<asset_pool::Module<T>>::update_debit(&who, currency_id, debit_adjustment_amount)?;
			Self::deposit_event(Event::AdjustDebitSuccess(who.clone(), currency_id, debit_adjustment_amount));
			Ok(().into())
		}
	}
}
