use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_runtime::traits::SaturatedConversion;

use crate::{AccountId, Balance, CurrencyId, Tokens};

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	let _ = <Tokens as MultiCurrencyExtended<_>>::update_balance(currency_id, &who, balance.saturated_into());
	assert_eq!(<Tokens as MultiCurrency<_>>::free_balance(currency_id, who), balance);
}
