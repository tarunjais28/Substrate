// wrapping these imbalances in a private module is necessary to ensure absolute
// privacy of the inner member.
use super::TokenBalance;
use crate::{Config, TotalIssuance};
use frame_support::storage::StorageMap;
use frame_support::traits::{Get, Imbalance, TryDrop};
use sp_runtime::traits::Zero;
use sp_std::{marker, mem, result};

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been created without any equal and opposite
/// accounting.
#[must_use]
pub struct PositiveImbalance<T: Config, GetCurrencyId: Get<T::CurrencyId>>(
    TokenBalance,
    marker::PhantomData<(T, GetCurrencyId)>,
);

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> PositiveImbalance<T, GetCurrencyId> {
    /// Create a new positive imbalance from a balance.
    pub fn new(amount: TokenBalance) -> Self {
        PositiveImbalance(amount, marker::PhantomData::<(T, GetCurrencyId)>)
    }
}

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been destroyed without any equal and opposite
/// accounting.
#[must_use]
pub struct NegativeImbalance<T: Config, GetCurrencyId: Get<T::CurrencyId>>(
    TokenBalance,
    marker::PhantomData<(T, GetCurrencyId)>,
);

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> NegativeImbalance<T, GetCurrencyId> {
    /// Create a new negative imbalance from a balance.
    pub fn new(amount: TokenBalance) -> Self {
        NegativeImbalance(amount, marker::PhantomData::<(T, GetCurrencyId)>)
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> TryDrop for PositiveImbalance<T, GetCurrencyId> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Imbalance<TokenBalance>
    for PositiveImbalance<T, GetCurrencyId>
{
    type Opposite = NegativeImbalance<T, GetCurrencyId>;

    fn zero() -> Self {
        Self::new(Zero::zero())
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
    fn split(self, amount: TokenBalance) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;

        mem::forget(self);
        (Self::new(first), Self::new(second))
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);

        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
        let (a, b) = (self.0, other.0);
        mem::forget((self, other));

        if a >= b {
            Ok(Self::new(a - b))
        } else {
            Err(NegativeImbalance::new(b - a))
        }
    }
    fn peek(&self) -> TokenBalance {
        self.0
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> TryDrop for NegativeImbalance<T, GetCurrencyId> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Imbalance<TokenBalance>
    for NegativeImbalance<T, GetCurrencyId>
{
    type Opposite = PositiveImbalance<T, GetCurrencyId>;

    fn zero() -> Self {
        Self::new(Zero::zero())
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
    fn split(self, amount: TokenBalance) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;

        mem::forget(self);
        (Self::new(first), Self::new(second))
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);

        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
        let (a, b) = (self.0, other.0);
        mem::forget((self, other));

        if a >= b {
            Ok(Self::new(a - b))
        } else {
            Err(PositiveImbalance::new(b - a))
        }
    }
    fn peek(&self) -> TokenBalance {
        self.0
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Drop for PositiveImbalance<T, GetCurrencyId> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        let currency_id = GetCurrencyId::get();
        let currency_code = super::Module::<T>::get_ccy_code_from_id_code(&currency_id);
        TotalIssuance::mutate(currency_code, |v| *v = v.saturating_add(self.0));
    }
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Drop for NegativeImbalance<T, GetCurrencyId> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        let currency_id = GetCurrencyId::get();
        let currency_code = super::Module::<T>::get_ccy_code_from_id_code(&currency_id);
        TotalIssuance::mutate(currency_code, |v| *v = v.saturating_sub(self.0));
    }
}
