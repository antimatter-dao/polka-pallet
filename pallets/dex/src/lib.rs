#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_core::U256;
use sp_runtime::{
	DispatchError,
	DispatchResult, FixedPointNumber, ModuleId, RuntimeDebug, SaturatedConversion,
	traits::{AccountIdConversion, UniqueSaturatedInto, Zero},
};
use sp_std::{convert::TryInto, prelude::*, vec};

use model::{Balance, CurrencyId, Price, Ratio, TradingPair};
pub use module::*;

# mod mock;
# mod test;
pub mod weights;
pub use weights::WeightInfo;

pub type CurrencyIds = Vec<CurrencyId>;

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub struct TradingPairPreparingParameters<Balance, BlockNumber> {
	min_contribution: (Balance, Balance),

	target_amount: (Balance, Balance),

	accumulated_amount: (Balance, Balance),

	not_before: BlockNumber,
}

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub enum TradingPairStatus<Balance, BlockNumber> {
	Enabled,

	Preparing(TradingPairPreparingParameters<Balance, BlockNumber>),

	Disabled,
}

impl<Balance, BlockNumber> Default for TradingPairStatus<Balance, BlockNumber> {
	fn default() -> Self {
		Self::Disabled
	}
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId=CurrencyId, Balance=Balance>;

		#[pallet::constant]
		type GetExchangeFee: Get<(u32, u32)>;

		#[pallet::constant]
		type TradingPathLimit: Get<u32>;

		#[pallet::constant]
		type ModuleId: Get<ModuleId>;

		type WhiteListOrigin: EnsureOrigin<Self::Origin>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		DisabledTradingPair,

		MustBeEnabled,

		MustBePreparing,

		MustBeDisabled,

		NotAllowedNew,

		InvalidContributionIncrement,

		InvalidLiquidityIncrement,

		InvalidCurrencyId,

		InvalidTradingPathLength,

		InsufficientTargetAmount,

		ExcessiveSupplyAmount,

		ExceedPriceImpactLimit,

		InsufficientLiquidity,

		ZeroSupplyAmount,

		ZeroTargetAmount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		AddPreparing(T::AccountId, CurrencyId, Balance, CurrencyId, Balance),

		AddLiquidity(T::AccountId, CurrencyId, Balance, CurrencyId, Balance, Balance),

		RemoveLiquidity(T::AccountId, CurrencyId, Balance, CurrencyId, Balance, Balance),

		Swap(T::AccountId, CurrencyIds, Balance, Balance),

		EnableTradingPair(TradingPair),

		NewTradingPair(TradingPair),

		DisableTradingPair(TradingPair),

		PreparingToEnabled(TradingPair, Balance, Balance, Balance),
	}

	#[pallet::storage]
	#[pallet::getter(fn liquidity_pool)]
	pub type LiquidityPool<T: Config> = StorageMap<_, Twox64Concat, TradingPair, (Balance, Balance), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn trading_pair_statuses)]
	pub type TradingPairStatuses<T: Config> =
	StorageMap<_, Twox64Concat, TradingPair, TradingPairStatus<Balance, T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn preparing_pool)]
	pub type PreparingPool<T: Config> =
	StorageDoubleMap<_, Twox64Concat, TradingPair, Twox64Concat, T::AccountId, (Balance, Balance), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn leverage_pool)]
	pub type LeveragePool<T: Config> =
	StorageDoubleMap<_, Twox64Concat, TradingPair, Twox64Concat, T::AccountId, (Balance, Balance), ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_preparing_trading_pairs: Vec<(TradingPair, (Balance, Balance), (Balance, Balance), T::BlockNumber)>,
		pub initial_enabled_trading_pairs: Vec<TradingPair>,
		pub initial_liquidity_pools: Vec<(T::AccountId, Vec<(TradingPair, (Balance, Balance))>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				initial_preparing_trading_pairs: vec![],
				initial_enabled_trading_pairs: vec![],
				initial_liquidity_pools: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.initial_preparing_trading_pairs.iter().for_each(
				|(trading_pair, min_contribution, target_amount, not_before)| {
					assert!(
						trading_pair.get_dex_share_currency_id().is_some(),
						"the trading pair is invalid!",
					);
					TradingPairStatuses::<T>::insert(
						trading_pair,
						TradingPairStatus::Preparing(TradingPairPreparingParameters {
							min_contribution: *min_contribution,
							target_amount: *target_amount,
							accumulated_amount: Default::default(),
							not_before: *not_before,
						}),
					);
				},
			);

			self.initial_enabled_trading_pairs.iter().for_each(|trading_pair| {
				assert!(
					trading_pair.get_dex_share_currency_id().is_some(),
					"the trading pair is invalid!",
				);
				TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::<_, _>::Enabled);
			});

			self.initial_liquidity_pools
				.iter()
				.for_each(|(who, trading_pairs_data)| {
					trading_pairs_data
						.iter()
						.for_each(|(trading_pair, (deposit_amount_0, deposit_amount_1))| {
							assert!(
								trading_pair.get_dex_share_currency_id().is_some(),
								"the trading pair is invalid!",
							);

							let result = match <Module<T>>::trading_pair_statuses(trading_pair) {
								TradingPairStatus::<_, _>::Enabled => <Module<T>>::do_add_liquidity(
									&who,
									trading_pair.0,
									trading_pair.1,
									*deposit_amount_0,
									*deposit_amount_1,
									false,
								),
								_ => Err(Error::<T>::DisabledTradingPair.into()),
							};

							assert!(result.is_ok(), "genesis add liquidity pool failed.");
						});
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// TODO
		pub fn swap_with_exact_supply_by_leverage() -> DispatchResultWithPostInfo {
			Ok(().into())
		}

		// TODO
		pub fn swap_with_exact_target_by_leverage() -> DispatchResultWithPostInfo {
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::swap_with_exact_supply(), DispatchClass::Operational))]
		#[transactional]
		pub fn swap_with_exact_supply(
			origin: OriginFor<T>,
			route: CurrencyIds,
			supply_amount: Balance,
			min_target_amount: Balance,
			price_impact_limit: Ratio,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let _ = Self::do_swap_with_exact_supply(&who, &route,
													supply_amount, min_target_amount,
													Some(price_impact_limit))?;
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::swap_with_exact_target(), DispatchClass::Operational))]
		#[transactional]
		pub fn swap_with_exact_target(
			origin: OriginFor<T>,
			route: CurrencyIds,
			target_amount: Balance,
			max_supply_amount: Balance,
			price_impact_limit: Ratio,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let _ = Self::do_swap_with_exact_target(&who, &route,
													target_amount, max_supply_amount,
													Some(price_impact_limit))?;
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::add_liquidity(), DispatchClass::Operational))]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			max_amount_a: Balance,
			max_amount_b: Balance,
			deposit_increment_share: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
				.ok_or(Error::<T>::InvalidCurrencyId)?;

			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::<_, _>::Enabled => Self::do_add_liquidity(
					&who,
					currency_id_a,
					currency_id_b,
					max_amount_a,
					max_amount_b,
					deposit_increment_share,
				),
				TradingPairStatus::<_, _>::Preparing(_) => {
					Self::do_add_preparing(&who, currency_id_a, currency_id_b, max_amount_a, max_amount_b)
						.map(|_| Self::convert_to_enabled_if_possible(trading_pair))
				}
				TradingPairStatus::<_, _>::Disabled => Err(Error::<T>::DisabledTradingPair.into()),
			}?;
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::remove_liquidity(), DispatchClass::Operational))]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			remove_share: Balance,
			by_withdraw: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_remove_liquidity(&who, currency_id_a, currency_id_b, remove_share, by_withdraw)?;
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::new_trading_pair(), DispatchClass::Operational))]
		#[transactional]
		pub fn new_trading_pair(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			min_contribution_a: Balance,
			min_contribution_b: Balance,
			target_amount_a: Balance,
			target_amount_b: Balance,
			not_before: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			T::WhiteListOrigin::ensure_origin(origin)?;

			let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
				.ok_or(Error::<T>::InvalidCurrencyId)?;
			let dex_share_currency_id = trading_pair
				.get_dex_share_currency_id()
				.ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(Self::trading_pair_statuses(trading_pair), TradingPairStatus::<_, _>::Disabled),
				Error::<T>::MustBeDisabled
			);
			ensure!(
				T::Currency::total_issuance(dex_share_currency_id).is_zero(),
				Error::<T>::NotAllowedNew
			);

			let (min_contribution, target_amount) = if currency_id_a == trading_pair.0 {
				(
					(min_contribution_a, min_contribution_b),
					(target_amount_a, target_amount_b),
				)
			} else {
				(
					(min_contribution_b, min_contribution_a),
					(target_amount_b, target_amount_a),
				)
			};

			TradingPairStatuses::<T>::insert(
				trading_pair,
				TradingPairStatus::Preparing(TradingPairPreparingParameters {
					min_contribution,
					target_amount,
					accumulated_amount: Default::default(),
					not_before,
				}),
			);
			Self::deposit_event(Event::NewTradingPair(trading_pair));
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::enable_trading_pair(), DispatchClass::Operational))]
		#[transactional]
		pub fn enable_trading_pair(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::WhiteListOrigin::ensure_origin(origin)?;

			let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
				.ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(
					Self::trading_pair_statuses(trading_pair),
					TradingPairStatus::<_, _>::Disabled
				),
				Error::<T>::MustBeDisabled
			);

			TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Enabled);
			Self::deposit_event(Event::EnableTradingPair(trading_pair));
			Ok(().into())
		}

		#[pallet::weight((<T as Config>::WeightInfo::disable_trading_pair(), DispatchClass::Operational))]
		#[transactional]
		pub fn disable_trading_pair(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::WhiteListOrigin::ensure_origin(origin)?;
			let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
				.ok_or(Error::<T>::InvalidCurrencyId)?;

			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::<_, _>::Enabled => {
					TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Disabled);
					Self::deposit_event(Event::DisableTradingPair(trading_pair));
				}
				TradingPairStatus::<_, _>::Preparing(_) => {
					let module_account_id = Self::account_id();

					for (who, contribution) in PreparingPool::<T>::drain_prefix(trading_pair) {
						T::Currency::transfer(trading_pair.0, &module_account_id, &who, contribution.0)?;
						T::Currency::transfer(trading_pair.1, &module_account_id, &who, contribution.1)?;

						frame_system::Module::<T>::dec_consumers(&who);
					}

					TradingPairStatuses::<T>::remove(trading_pair);
					Self::deposit_event(Event::DisableTradingPair(trading_pair));
				}
				TradingPairStatus::<_, _>::Disabled => {
					return Err(Error::<T>::DisabledTradingPair.into());
				}
			};
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	fn convert_to_enabled_if_possible(trading_pair: TradingPair) {
		if let TradingPairStatus::<_, _>::Preparing(preparing_parameters) = Self::trading_pair_statuses(trading_pair)
		{
			if frame_system::Module::<T>::block_number() >= preparing_parameters.not_before
				&& !preparing_parameters.accumulated_amount.0.is_zero()
				&& !preparing_parameters.accumulated_amount.1.is_zero()
				&& (preparing_parameters.accumulated_amount.0 >= preparing_parameters.target_amount.0
				|| preparing_parameters.accumulated_amount.1 >= preparing_parameters.target_amount.1)
			{
				let initial_price_0_in_1: Price = Price::checked_from_rational(
					preparing_parameters.accumulated_amount.1,
					preparing_parameters.accumulated_amount.0,
				)
					.unwrap_or_default();

				let lp_share_currency_id = trading_pair.get_dex_share_currency_id().expect("shouldn't be invalid!");
				let mut total_shares_issued: Balance = Default::default();
				for (who, contribution) in PreparingPool::<T>::drain_prefix(trading_pair) {
					let share_amount = initial_price_0_in_1
						.saturating_mul_int(contribution.0)
						.saturating_add(contribution.1);

					if T::Currency::deposit(lp_share_currency_id, &who, share_amount).is_ok() {
						total_shares_issued = total_shares_issued.saturating_add(share_amount);
					}

					frame_system::Module::<T>::dec_consumers(&who);
				}

				LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
					*pool_0 = pool_0.saturating_add(preparing_parameters.accumulated_amount.0);
					*pool_1 = pool_1.saturating_sub(preparing_parameters.accumulated_amount.1);
				});

				TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::<_, _>::Enabled);

				Self::deposit_event(Event::PreparingToEnabled(
					trading_pair,
					preparing_parameters.accumulated_amount.0,
					preparing_parameters.accumulated_amount.1,
					total_shares_issued,
				));
			}
		}
	}

	fn do_add_preparing(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		contribution_a: Balance,
		contribution_b: Balance,
	) -> DispatchResult {
		let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
		let mut preparing_parameters = match Self::trading_pair_statuses(trading_pair) {
			TradingPairStatus::<_, _>::Preparing(preparing_parameters) => preparing_parameters,
			_ => return Err(Error::<T>::MustBePreparing.into()),
		};
		let (contribution_0, contribution_1) = if currency_id_a == trading_pair.0 {
			(contribution_a, contribution_b)
		} else {
			(contribution_b, contribution_a)
		};

		ensure!(
			contribution_0 >= preparing_parameters.min_contribution.0
				|| contribution_1 >= preparing_parameters.min_contribution.1,
			Error::<T>::InvalidContributionIncrement
		);

		PreparingPool::<T>::try_mutate_exists(trading_pair, &who, |maybe_pool| -> DispatchResult {
			let existed = maybe_pool.is_some();
			let mut pool = maybe_pool.unwrap_or_default();
			pool.0 = pool.0.saturating_add(contribution_0);
			pool.1 = pool.1.saturating_add(contribution_1);

			let module_account_id = Self::account_id();
			T::Currency::transfer(trading_pair.0, &who, &module_account_id, contribution_0)?;
			T::Currency::transfer(trading_pair.1, &who, &module_account_id, contribution_1)?;

			*maybe_pool = Some(pool);

			if !existed && maybe_pool.is_some() {
				if frame_system::Module::<T>::inc_consumers(&who).is_err() {
					frame_support::debug::warn!(
						"Warning: Attempt to introduce lock consumer reference, yet no providers. \
						This is unexpected but should be safe."
					);
				}
			}

			preparing_parameters.accumulated_amount.0 = preparing_parameters
				.accumulated_amount
				.0
				.saturating_add(contribution_0);
			preparing_parameters.accumulated_amount.1 = preparing_parameters
				.accumulated_amount
				.1
				.saturating_add(contribution_1);

			TradingPairStatuses::<T>::insert(
				trading_pair,
				TradingPairStatus::<_, _>::Preparing(preparing_parameters),
			);

			Self::deposit_event(Event::AddPreparing(
				who.clone(),
				trading_pair.0,
				contribution_0,
				trading_pair.1,
				contribution_1,
			));
			Ok(())
		})
	}

	fn do_add_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		max_amount_a: Balance,
		max_amount_b: Balance,
		deposit_increment_share: bool,
	) -> DispatchResult {
		let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
		let lp_share_currency_id = trading_pair
			.get_dex_share_currency_id()
			.ok_or(Error::<T>::InvalidCurrencyId)?;
		ensure!(
			matches!(
				Self::trading_pair_statuses(trading_pair),
				TradingPairStatus::<_, _>::Enabled
			),
			Error::<T>::MustBeEnabled,
		);

		LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
			let total_shares = T::Currency::total_issuance(lp_share_currency_id);
			let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.0 {
				(max_amount_a, max_amount_b)
			} else {
				(max_amount_b, max_amount_a)
			};
			let (pool_0_increment, pool_1_increment, share_increment): (Balance, Balance, Balance) =
				if total_shares.is_zero() {
					let initial_share = sp_std::cmp::max(max_amount_0, max_amount_1);
					(max_amount_0, max_amount_1, initial_share)
				} else {
					let price_0_1 = Price::checked_from_rational(*pool_1, *pool_0).unwrap_or_default();
					let input_price_0_1 = Price::checked_from_rational(max_amount_1, max_amount_0).unwrap_or_default();

					if input_price_0_1 <= price_0_1 {
						let price_1_0 = Price::checked_from_rational(*pool_0, *pool_1).unwrap_or_default();
						let amount_0 = price_1_0.saturating_mul_int(max_amount_1);
						let share_increment = Ratio::checked_from_rational(amount_0, *pool_0)
							.and_then(|n| n.checked_mul_int(total_shares))
							.unwrap_or_default();
						(amount_0, max_amount_1, share_increment)
					} else {
						let amount_1 = price_0_1.saturating_mul_int(max_amount_0);
						let share_increment = Ratio::checked_from_rational(amount_1, *pool_1)
							.and_then(|n| n.checked_mul_int(total_shares))
							.unwrap_or_default();
						(max_amount_0, amount_1, share_increment)
					}
				};

			ensure!(
				!share_increment.is_zero() && !pool_0_increment.is_zero() && !pool_1_increment.is_zero(),
				Error::<T>::InvalidLiquidityIncrement,
			);

			let module_account_id = Self::account_id();
			T::Currency::transfer(trading_pair.0, who, &module_account_id, pool_0_increment)?;
			T::Currency::transfer(trading_pair.1, who, &module_account_id, pool_1_increment)?;
			T::Currency::deposit(lp_share_currency_id, who, share_increment)?;

			*pool_0 = pool_0.saturating_add(pool_0_increment);
			*pool_1 = pool_1.saturating_add(pool_1_increment);

			Self::deposit_event(Event::AddLiquidity(
				who.clone(),
				trading_pair.0,
				pool_0_increment,
				trading_pair.1,
				pool_1_increment,
				share_increment,
			));
			Ok(())
		})
	}

	fn do_remove_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		remove_share: Balance,
		by_withdraw: bool,
	) -> DispatchResult {
		if remove_share.is_zero() {
			return Ok(());
		}
		let trading_pair =
			TradingPair::from_token_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
		let lp_share_currency_id = trading_pair
			.get_dex_share_currency_id()
			.ok_or(Error::<T>::InvalidCurrencyId)?;

		LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
			let total_shares = T::Currency::total_issuance(lp_share_currency_id);
			let proportion = Ratio::checked_from_rational(remove_share, total_shares).unwrap_or_default();
			let pool_0_decrement = proportion.saturating_mul_int(*pool_0);
			let pool_1_decrement = proportion.saturating_mul_int(*pool_1);
			let module_account_id = Self::account_id();

			T::Currency::withdraw(lp_share_currency_id, &who, remove_share)?;
			T::Currency::transfer(trading_pair.0, &module_account_id, &who, pool_0_decrement)?;
			T::Currency::transfer(trading_pair.1, &module_account_id, &who, pool_1_decrement)?;

			*pool_0 = pool_0.saturating_sub(pool_0_decrement);
			*pool_1 = pool_1.saturating_sub(pool_1_decrement);

			Self::deposit_event(Event::RemoveLiquidity(
				who.clone(),
				trading_pair.0,
				pool_0_decrement,
				trading_pair.1,
				pool_1_decrement,
				remove_share,
			));
			Ok(())
		})
	}

	fn get_target_amounts(
		path: &[CurrencyId],
		supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut target_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		target_amounts[0] = supply_amount;

		let mut i: usize = 0;
		while i + 1 < path_length {
			ensure!(
				matches!(
					Self::trading_pair_statuses(TradingPair::new(path[i], path[i + 1])),
					TradingPairStatus::<_, _>::Enabled
				),
				Error::<T>::MustBeEnabled
			);
			let (supply_pool, target_pool) = Self::get_liquidity(path[i], path[i + 1]);
			ensure!(
				!supply_pool.is_zero() && !target_pool.is_zero(),
				Error::<T>::InsufficientLiquidity
			);
			let target_amount = Self::get_target_amount(supply_pool, target_pool, target_amounts[i]);
			ensure!(!target_amount.is_zero(), Error::<T>::ZeroTargetAmount);

			if let Some(limit) = price_impact_limit {
				let price_impact = Ratio::checked_from_rational(target_amount, target_pool).unwrap_or_else(Ratio::zero);
				ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
			}

			target_amounts[i + 1] = target_amount;
			i += 1;
		}

		Ok(target_amounts)
	}

	fn get_target_amount(supply_pool: Balance, target_pool: Balance, supply_amount: Balance) -> Balance {
		if supply_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
			Zero::zero()
		} else {
			let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
			let supply_amount_with_fee =
				supply_amount.saturating_mul(fee_denominator.saturating_sub(fee_numerator).unique_saturated_into());
			let numerator: U256 = U256::from(supply_amount_with_fee).saturating_mul(U256::from(target_pool));
			let denominator: U256 = U256::from(supply_pool)
				.saturating_mul(U256::from(fee_denominator))
				.saturating_add(U256::from(supply_amount_with_fee));
			numerator
				.checked_div(denominator)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.unwrap_or_else(Zero::zero)
		}
	}

	fn get_supply_amounts(
		path: &[CurrencyId],
		target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		debug::RuntimeLogger::init();
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut supply_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		supply_amounts[path_length - 1] = target_amount;

		let mut i: usize = path_length - 1;
		while i > 0 {
			ensure!(
				matches!(
					Self::trading_pair_statuses(TradingPair::new(path[i - 1], path[i])),
					TradingPairStatus::<_, _>::Enabled
				),
				Error::<T>::MustBeEnabled
			);
			let (supply_pool, target_pool) = Self::get_liquidity(path[i - 1], path[i]);
			ensure!(
				!supply_pool.is_zero() && !target_pool.is_zero(),
				Error::<T>::InsufficientLiquidity
			);
			let supply_amount = Self::get_supply_amount(supply_pool, target_pool, supply_amounts[i]);
			ensure!(!supply_amount.is_zero(), Error::<T>::ZeroSupplyAmount);

			debug::info!(target: "dex debug:", "price_impact_limit: {:?}, supply_amount: {:?}, target_pool: {:?}",
						 price_impact_limit, supply_amounts[i], target_pool);
			if let Some(limit) = price_impact_limit {
				let price_impact =
					Ratio::checked_from_rational(supply_amounts[i], target_pool).unwrap_or_else(Ratio::zero);
				ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
			};

			supply_amounts[i - 1] = supply_amount;
			i -= 1;
		}

		Ok(supply_amounts)
	}

	fn get_supply_amount(supply_pool: Balance, target_pool: Balance, target_amount: Balance) -> Balance {
		if target_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
			Zero::zero()
		} else {
			let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
			let numerator: U256 = U256::from(supply_pool)
				.saturating_mul(U256::from(target_amount))
				.saturating_mul(U256::from(fee_denominator));
			let denominator: U256 = U256::from(target_pool)
				.saturating_sub(U256::from(target_amount))
				.saturating_mul(U256::from(fee_denominator.saturating_sub(fee_numerator)));
			numerator
				.checked_div(denominator)
				.and_then(|r| r.checked_add(U256::one()))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.unwrap_or_else(Zero::zero)
		}
	}

	fn get_liquidity(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
		let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
		let (pool_0, pool_1) = Self::liquidity_pool(trading_pair);
		if currency_id_a == trading_pair.0 {
			(pool_0, pool_1)
		} else {
			(pool_1, pool_0)
		}
	}

	fn _swap(supply_currency_id: CurrencyId,
			 target_currency_id: CurrencyId,
			 supply_increment: Balance,
			 target_decrement: Balance,
	) {
		if let Some(trading_pair) = TradingPair::from_token_currency_ids(supply_currency_id, target_currency_id) {
			LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
				if supply_currency_id == trading_pair.0 {
					*pool_0 = pool_0.saturating_add(supply_increment);
					*pool_1 = pool_1.saturating_sub(target_decrement);
				} else {
					*pool_0 = pool_0.saturating_sub(target_decrement);
					*pool_1 = pool_1.saturating_add(supply_increment);
				}
			});
		}
	}

	fn _swap_by_path(path: &[CurrencyId], amounts: &[Balance]) {
		let mut i: usize = 0;
		while i + 1 < path.len() {
			let (supply_currency_id, target_currency_id) = (path[i], path[i + 1]);
			let (supply_increment, target_decrement) = (amounts[i], amounts[i + 1]);
			Self::_swap(supply_currency_id, target_currency_id, supply_increment, target_decrement);
			i += 1;
		}
	}

	#[transactional]
	fn do_swap_with_exact_supply(
		who: &T::AccountId,
		path: &[CurrencyId],
		supply_amount: Balance,
		min_target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let amounts = Self::get_target_amounts(&path, supply_amount, price_impact_limit)?;
		ensure!(
			amounts[amounts.len() - 1] >= min_target_amount,
			Error::<T>::InsufficientTargetAmount
		);
		let module_account_id = Self::account_id();
		let actual_target_amount = amounts[amounts.len() - 1];

		T::Currency::transfer(path[0], who, &module_account_id, supply_amount)?;
		Self::_swap_by_path(&path, &amounts);
		T::Currency::transfer(path[path.len() - 1], &module_account_id, who, actual_target_amount)?;

		Self::deposit_event(Event::Swap(
			who.clone(),
			path.to_vec(),
			supply_amount,
			actual_target_amount,
		));
		Ok(actual_target_amount)
	}

	#[transactional]
	fn do_swap_with_exact_target(
		who: &T::AccountId,
		path: &[CurrencyId],
		target_amount: Balance,
		max_supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let amounts = Self::get_supply_amounts(&path, target_amount, price_impact_limit)?;
		ensure!(amounts[0] <= max_supply_amount, Error::<T>::ExcessiveSupplyAmount);
		let module_account_id = Self::account_id();
		let actual_supply_amount = amounts[0];

		T::Currency::transfer(path[0], who, &module_account_id, actual_supply_amount)?;
		Self::_swap_by_path(&path, &amounts);
		T::Currency::transfer(path[path.len() - 1], &module_account_id, who, target_amount)?;

		Self::deposit_event(Event::Swap(
			who.clone(),
			path.to_vec(),
			actual_supply_amount,
			target_amount,
		));
		Ok(actual_supply_amount)
	}

}
