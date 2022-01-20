#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{FixedU128, RuntimeDebug};
use sp_std::{prelude::*};

pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;
pub type Rate = FixedU128;

pub type Balance = u128;
pub type Amount = i128;

pub type Moment = u64;
pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
pub enum DataProviderId {
	Aggregated = 0,
	antimatter = 1,
}


#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	Token(u8),
	DEXShare(u8, u8),
}

impl CurrencyId {
	pub fn is_token_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Token(_))
	}

	pub fn is_dex_share_currency_id(&self) -> bool {
		matches!(self, CurrencyId::DEXShare(_, _))
	}

	pub fn split_dex_share_currency_id(&self) -> Option<(Self, Self)> {
		match self {
			CurrencyId::DEXShare(token_symbol_0, token_symbol_1) => {
				Some((CurrencyId::Token(*token_symbol_0), CurrencyId::Token(*token_symbol_1)))
			}
			_ => None,
		}
	}

	pub fn join_dex_share_currency_id(currency_id_0: Self, currency_id_1: Self) -> Option<Self> {
		match (currency_id_0, currency_id_1) {
			(CurrencyId::Token(token_symbol_0), CurrencyId::Token(token_symbol_1)) => {
				Some(CurrencyId::DEXShare(token_symbol_0, token_symbol_1))
			}
			_ => None,
		}
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair(pub CurrencyId, pub CurrencyId);

impl TradingPair {
	pub fn new(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> Self {
		if currency_id_a > currency_id_b {
			TradingPair(currency_id_b, currency_id_a)
		} else {
			TradingPair(currency_id_a, currency_id_b)
		}
	}

	pub fn from_token_currency_ids(currency_id_0: CurrencyId, currency_id_1: CurrencyId) -> Option<Self> {
		match currency_id_0.is_token_currency_id() && currency_id_1.is_token_currency_id() {
			true if currency_id_0 > currency_id_1 => Some(TradingPair(currency_id_1, currency_id_0)),
			true if currency_id_0 < currency_id_1 => Some(TradingPair(currency_id_0, currency_id_1)),
			_ => None,
		}
	}

	pub fn get_dex_share_currency_id(&self) -> Option<CurrencyId> {
		CurrencyId::join_dex_share_currency_id(self.0, self.1)
	}
}

pub const MB: CurrencyId = CurrencyId::Token(0);
pub const DOT: CurrencyId = CurrencyId::Token(1);
pub const ETH: CurrencyId = CurrencyId::Token(2);
pub const BTC: CurrencyId = CurrencyId::Token(3);
pub const FIL: CurrencyId = CurrencyId::Token(4);

pub const DOT_ETH_PAIR: TradingPair = TradingPair(DOT, ETH);
pub const DOT_BTC_PAIR: TradingPair = TradingPair(DOT, BTC);
pub const DOT_FIL_PAIR: TradingPair = TradingPair(DOT, FIL);
