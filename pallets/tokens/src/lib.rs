//! # Tokens Module
//!
//! ## Overview
//!
//! The tokens module provides fungible multi-currency functionality that
//! implements `MultiCurrency` trait.
//!
//! The tokens module provides functions for:
//!
//! - Querying and setting the balance of a given account.
//! - Getting and managing total issuance.
//! - Balance transfer between accounts.
//! - Depositing and withdrawing balance.
//! - Slashing an account balance.
//!
//! ### Implementations
//!
//! The tokens module provides implementations for following traits.
//!
//! - `MultiCurrency` - Abstraction over a fungible multi-currency system.
//! - `MultiCurrencyExtended` - Extended `MultiCurrency` with additional helper
//!   types and methods, like updating balance
//! by a given signed integer amount.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `transfer` - Transfer some balance to another account.
//! - `transfer_all` - Transfer all balance to another account.
//!
//! ### Genesis Config
//!
//! The tokens module depends on the `GenesisConfig`. Endowed accounts could be
//! configured in genesis configs.

#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, fail,
    traits::Get,
    traits::{
        BalanceStatus as Status, Currency as PalletCurrency, ExistenceRequirement, Imbalance,
        ReservableCurrency as PalletReservableCurrency, SignedImbalance, WithdrawReasons,
    },
    weights::Weight,
    Parameter, StorageMap,
};
use frame_system::{ensure_root, ensure_signed};
use num::traits::{FromPrimitive, ToPrimitive};
use sp_runtime::{
    traits::{Bounded, MaybeSerializeDeserialize, Member, StaticLookup, Zero},
    DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    marker,
    prelude::*,
    result,
};

pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};
pub use crate::structs::*;
pub type TokenBalance = u128;
use did::Did;
use orml_traits::{
    arithmetic::{self, Signed},
    BalanceStatus, LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiReservableCurrency,
};
pub type CurrencyCode = [u8; 8];

mod migration;
mod structs;
mod tests;

mod default_weight;
mod imbalances;

pub trait WeightInfo {
    fn transfer() -> Weight;
    fn transfer_all() -> Weight;
}

type BalanceOf<T> =
    <<T as Config>::Currency as PalletCurrency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config: frame_system::Config + did::Config + vc::Config {
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

    /// The currency trait.
    type Currency: PalletReservableCurrency<Self::AccountId>;

    /// The amount type, should be signed version of `Balance`
    type Amount: Signed
        + TryInto<TokenBalance>
        + TryFrom<TokenBalance>
        + Parameter
        + Member
        + arithmetic::SimpleArithmetic
        + Default
        + Copy
        + MaybeSerializeDeserialize;

    /// The currency ID type
    type CurrencyId: Parameter
        + Member
        + Copy
        + MaybeSerializeDeserialize
        + Ord
        + Default
        + FromPrimitive
        + ToPrimitive;

    /// The treasury reserve type
    type TreasuryReserve: Get<BalanceOf<Self>>;

    /// Weight information for extrinsics in this module.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as Tokens {
        /// The total issuance of a token type.
        pub TotalIssuance get(fn total_issuance): map hasher(twox_64_concat) CurrencyCode => TokenBalance;

        /// Any liquidity locks of a token type under an account.
        /// NOTE: Should only be accessed when setting, changing and freeing a lock.
        pub Locks get(fn locks): double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) CurrencyCode => Vec<BalanceLock<TokenBalance>>;

        /// The balance of a token type under an account.
        ///
        /// NOTE: If the total is ever zero, decrease account ref account.
        ///
        /// NOTE: This is only used in the case that this module is used to store balances.
        pub Accounts get(fn accounts): double_map hasher(blake2_128_concat) did::Did, hasher(twox_64_concat) CurrencyCode => TokenAccountInfo<T::Index ,TokenAccountData>;
        /// map to store a friendsly name for token
        pub TokenData get(fn token_data) : map hasher(blake2_128_concat) CurrencyCode => Option<TokenDetails>;
        /// To get the owner of the token
        pub TokenIssuer get(fn token_issuer): map hasher(blake2_128_concat) CurrencyCode => Did;
        // Counter for currency
        pub TokenCurrencyCounter get(fn currency_id): Option<T::CurrencyId>;
        /// The current version of the pallet
        PalletVersion: StorageVersion = StorageVersion::V2_0_0;
        /// To get the currency_code to currency_id mapping
        TokenInfo get(fn token_info): map hasher(blake2_128_concat) CurrencyCode => T::CurrencyId;
        /// To get the reverse currency_code to currency_id mapping
        TokenInfoRLookup get(fn token_info_reverse_lookup): map hasher(blake2_128_concat) T::CurrencyId => CurrencyCode;
    }
    add_extra_genesis {
        config(endowed_accounts): Vec<(did::Did, CurrencyCode, TokenBalance)>;

        build(|config: &GenesisConfig| {
            config.endowed_accounts.iter().for_each(|(account_id, currenct_code, initial_balance)| {
                <Accounts<T>>::mutate(account_id, currenct_code, |account_data| account_data.data.free = *initial_balance)
            })
        })
    }
}

decl_event!(
    pub enum Event {
        /// Token transfer success. [CurrencyCode, from, to, amount]
        Transferred(CurrencyCode, Did, Did, TokenBalance),
        /// Token issuance successful [CurrencyCode, dest, amount]
        TokenIssued(CurrencyCode, Did, TokenBalance, vc::VCid),
        /// Withdrawn from treasury reserve
        TreasuryWithdrawal(Did, Did),
        /// Token amount slashed
        TokenSlashed(CurrencyCode, Did, TokenBalance, vc::VCid),
        /// Token amount is minted
        TokenMinted(CurrencyCode, Did, TokenBalance, vc::VCid),
        /// Token amount is tranfered
        TransferredWithVC(CurrencyCode, Did, TokenBalance, vc::VCid),
        /// Token Balance Set
        TokenBalanceSet(CurrencyCode, Did, TokenBalance),
    }
);

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Transfer some balance to another account.
        ///
        /// The dispatch origin for this call must be `Signed` by the transactor.
        ///
        /// # <weight>
        /// - Complexity: `O(1)`
        /// - Db reads: 4
        /// - Db writes: 2
        /// -------------------
        /// Base Weight: 84.08 µs
        /// # </weight>
        #[weight = 1]
        pub fn transfer(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            currency_code: CurrencyCode,
            amount: TokenBalance,
        ) {
            let from = ensure_signed(origin)?;
            let to = T::Lookup::lookup(dest)?;
            let currency_id = Self::get_ccy_id_from_ccy_code(&currency_code);

            // ensure the recipent DID is valid
            ensure!(did::Module::<T>::does_did_exist(&to), Error::<T>::RecipentDIDNotRegistered);
            <Self as MultiCurrency<_>>::transfer(currency_id, &from, &to, amount)?;

            // Emit transfer event - fetch DID of account to emit event correctly
            let source_did= did::Module::<T>::get_did_from_account_id(&from);
            let dest_did = did::Module::<T>::get_did_from_account_id(&to);
            Self::deposit_event(Event::Transferred(currency_code, source_did, dest_did, amount));
        }

        /// Transfer all remaining balance to the given account.
        ///
        /// The dispatch origin for this call must be `Signed` by the transactor.
        ///
        /// # <weight>
        /// - Complexity: `O(1)`
        /// - Db reads: 4
        /// - Db writes: 2
        /// -------------------
        /// Base Weight: 87.71 µs
        /// # </weight>
        #[weight = 1]
        pub fn transfer_all(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            currency_code: CurrencyCode,
        ) {
            let from = ensure_signed(origin)?;
            let to = T::Lookup::lookup(dest)?;
            let currency_id = Self::get_ccy_id_from_ccy_code(&currency_code);

            // ensure the recipent DID is valid
            ensure!(did::Module::<T>::does_did_exist(&to), Error::<T>::RecipentDIDNotRegistered);
            let balance = <Self as MultiCurrency<T::AccountId>>::free_balance(currency_id, &from);
            <Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &from, &to, balance)?;
            let source_did= did::Module::<T>::get_did_from_account_id(&from);
            let dest_did = did::Module::<T>::get_did_from_account_id(&to);
            Self::deposit_event(Event::Transferred(currency_code, source_did, dest_did, balance));
        }

        /// Create a fixed supply of tokens
        ///
        /// The dispatch origin for this call must be `Signed` by either Sudo user or owner of the TokenVC.
        ///
        #[weight = 1]
        pub fn issue_token(
            origin,
            vc_id: vc::VCid,
            amount: TokenBalance,
        ) {
            let (owner, vc_struct) = match ensure_root(origin.clone()) {
                Ok(_) => {
                    let vc_struct = Self::get_vc_struct(&vc_id, &vc::VCType::TokenVC, Error::<T>::InvalidVC)?;
                    let owner = did::Module::<T>::get_accountid_from_did(&vc_struct.owner)?;
                    (owner, vc_struct)
                },
                Err(_) => {
                    let sender = ensure_signed(origin)?;
                    let vc_struct = Self::validate_vc(&sender, &vc_id, &vc::VCType::TokenVC, Error::<T>::InvalidVC)?;
                    (sender, vc_struct)
                }
            };

            let currency_id = Self::get_currency_id();
            let token_vc: vc::TokenVC =
                vc::Module::<T>::get_vc(&vc_struct.vc_property)?;
            let reservable_balance: BalanceOf<T> = token_vc.reservable_balance.try_into().ok().unwrap_or_default();

            // Checking for duplicate currency_code
            ensure!(!TokenInfo::<T>::contains_key(token_vc.currency_code), Error::<T>::CurrencyCodeAlreadyRegistered);

            // reserve the mui balance required to issue new token
            T::Currency::reserve(&owner, reservable_balance)?;

            // set total issuance to amount
            TotalIssuance::mutate(token_vc.currency_code, |issued| {
                *issued = issued.checked_add(amount).unwrap_or_else(|| {
                    *issued
                })
            });

            // allocate total issuance to the destination account - the token central bank
            Self::set_free_balance(token_vc.currency_code, &owner, amount);

            // set decimal, nonce, currency code and token_name of the destination account
            Self::set_fields(vc_struct.owner, currency_id, token_vc.clone(), token_vc.token_name.to_vec());

            let dest_did = did::Module::<T>::get_did_from_account_id(&owner);
            // store the token issuer/owner for lookup
            TokenIssuer::insert(token_vc.currency_code, dest_did);

            // update vc's is_used flag as used
            vc::Module::<T>::set_is_used_flag(vc_id);

            Self::set_currency_id(currency_id);

            Self::deposit_event(Event::TokenIssued(token_vc.currency_code, dest_did, amount, vc_id));
        }

        /// Slash the balance from the issuer account
        ///
        /// The dispatch origin for this call must be `Signed` by a issuer account.
        ///
        #[weight = 1]
        pub fn slash_token(
            origin,
            vc_id: vc::VCid,
        ) {
            let sender = ensure_signed(origin)?;
            let vc_struct = Self::validate_vc(&sender, &vc_id, &vc::VCType::SlashTokens, Error::<T>::InvalidVC)?;
            let slash_vc: vc::SlashMintTokens =
                vc::Module::<T>::get_vc::<vc::SlashMintTokens>(&vc_struct.vc_property)?;
            let amount: TokenBalance = slash_vc.amount.try_into().ok().unwrap_or_default();

            let currency_id = Self::get_ccy_id_from_ccy_code(&slash_vc.currency_code);
            let issuer = TokenIssuer::get(slash_vc.currency_code);

            let vc_owner = Self::get_vc_owner::<vc::SlashMintTokens>(&vc_struct)?;
            ensure!(<Self as MultiCurrency<T::AccountId>>::can_slash(currency_id, &vc_owner, amount), Error::<T>::BalanceTooLow);

            <Self as MultiCurrency<T::AccountId>>::slash(currency_id, &vc_owner, amount);

            // update vc's is_used flag as used
            vc::Module::<T>::set_is_used_flag(vc_id);

            Self::deposit_event(Event::TokenSlashed(slash_vc.currency_code, issuer, amount, vc_id));
        }

        /// Add amount to the issuer account
        ///
        /// The dispatch origin for this call must be `Signed` by a issuer account.
        /// Sender must be part of vc
        ///
        #[weight = 1]
        pub fn mint_token(
            origin,
            vc_id: vc::VCid,
        ) {
            let sender = ensure_signed(origin)?;

            let vc_struct =
                Self::validate_vc(&sender, &vc_id, &vc::VCType::MintTokens, Error::<T>::InvalidVC)?;
            let mint_vc: vc::SlashMintTokens =
                vc::Module::<T>::get_vc::<vc::SlashMintTokens>(&vc_struct.vc_property)?;
            let amount: TokenBalance = mint_vc.amount.try_into().ok().unwrap_or_default();

            let currency_id = Self::get_ccy_id_from_ccy_code(&mint_vc.currency_code);
            let issuer =  TokenIssuer::get(mint_vc.currency_code);

            let vc_owner = Self::get_vc_owner::<vc::SlashMintTokens>(&vc_struct)?;
            <Self as MultiCurrency<T::AccountId>>::deposit(currency_id, &vc_owner, amount)?;

            // update vc's is_used flag as used
            vc::Module::<T>::set_is_used_flag(vc_id);

            Self::deposit_event(Event::TokenMinted(mint_vc.currency_code, issuer, amount, vc_id));
        }

        // Transfer to admin from reserved amount for operational costs
        // The dispatch origin for this call must be `Signed` by a validator account.
        #[weight = 1]
        pub fn withdraw_reserved(
            origin,
            to : <T::Lookup as StaticLookup>::Source,
            from : <T::Lookup as StaticLookup>::Source,
            amount: BalanceOf<T>,
        ) {
            let _ = ensure_signed(origin)?;
            let to = T::Lookup::lookup(to)?;
            let from = T::Lookup::lookup(from)?;
            // unreserve the mui balance required to issue new token
            T::Currency::unreserve(&from, amount);
            // transfer amount to destination
            T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
            let source_did = did::Module::<T>::get_did_from_account_id(&from);
            let dest_did = did::Module::<T>::get_did_from_account_id(&to);
            // Self::deposit_event(Event::TreasuryWithdrawal(from, to));
            Self::deposit_event(Event::TreasuryWithdrawal(source_did, dest_did));
        }

        /// Transfer amount from token owner Did to given account's Did
        ///
        /// The dispatch origin for this call must be `Signed` by a issuer account.
        /// Sender must be part of vc
        ///
        #[weight = 1]
        pub fn transfer_token(
            origin,
            vc_id: vc::VCid,
            to : <T::Lookup as StaticLookup>::Source,
        ) {
            let sender = ensure_signed(origin)?;
            let to = T::Lookup::lookup(to)?;
            ensure!(did::Module::<T>::does_did_exist(&to), Error::<T>::RecipentDIDNotRegistered);
            let vc_struct =
                Self::validate_vc(&sender, &vc_id, &vc::VCType::TokenTransferVC, Error::<T>::InvalidVC)?;
            let transfer_vc: vc::TokenTransferVC =
                vc::Module::<T>::get_vc::<vc::TokenTransferVC>(&vc_struct.vc_property)?;
            let currency_id = Self::get_ccy_id_from_ccy_code(&transfer_vc.currency_code);
            let amount: TokenBalance = transfer_vc.amount.try_into().ok().unwrap_or_default();
            let vc_owner = Self::get_vc_owner::<vc::TokenTransferVC>(&vc_struct)?;

            <Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &vc_owner, &to, amount)?;

            // update vc's is_used flag as used
            vc::Module::<T>::set_is_used_flag(vc_id);

            let dest_did = did::Module::<T>::get_did_from_account_id(&to);

            Self::deposit_event(Event::TransferredWithVC(transfer_vc.currency_code, dest_did, amount, vc_id));
        }

        /// Set Balance of given did of given currency
        /// Balance will be transfered from/to owner's did to keep total issuance same
        #[weight = 1]
        pub fn set_balance(
            origin, 
            dest: Did,
            currency_code: CurrencyCode, 
            amount: TokenBalance,
        ) {
            let token_owner = match ensure_root(origin.clone()) {
                Ok(_) => {
                    Self::token_issuer(currency_code)
                },
                Err(_) => {
                    let sender = ensure_signed(origin)?;
                    Self::ensure_token_owner(&sender, currency_code)?
                }
            };
            
            ensure!(token_owner != dest, Error::<T>::NotAllowed);
            
            Self::set_token_balance(currency_code, token_owner, dest, amount)?;

            Self::deposit_event(Event::TokenBalanceSet(currency_code, dest, amount));
        }

        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            migration::migrate::<T>()
        }
    }
}

decl_error! {
    /// Error for token module.
    pub enum Error for Module<T: Config> {
        /// The balance is too low
        BalanceTooLow,
        /// This operation will cause balance to overflow
        BalanceOverflow,
        /// This operation will cause total issuance to overflow
        TotalIssuanceOverflow,
        /// Cannot convert Amount into Balance type
        AmountIntoBalanceFailed,
        /// Failed because liquidity restrictions due to locking
        LiquidityRestrictions,
        /// the recipent did must be valid
        RecipentDIDNotRegistered,
        /// Unable to decode the VC
        InvalidVC,
        /// Currency Code already registered
        CurrencyCodeAlreadyRegistered,
        /// Token Amount Overflow
        TokenAmountOverflow,
        /// Only Token owner can set other's balance
        NotAllowed
    }
}

impl<T: Config> Module<T> {
    /// Set free balance of `who` to a new value.
    ///
    /// Note this will not maintain total issuance.
    fn set_free_balance(currency_code: CurrencyCode, who: &T::AccountId, balance: TokenBalance) {
        let did = did::Module::<T>::get_did_from_account_id(who);
        <Accounts<T>>::mutate(did, currency_code, |account_data| {
            account_data.data.free = balance
        });
    }

    /// Set free balance of `who` to a new value.
    ///
    /// Note this will not maintain total issuance.
    fn get_ccy_id_from_ccy_code(ccy_code: &CurrencyCode) -> T::CurrencyId {
        TokenInfo::<T>::get(ccy_code)
    }

    /// Set free balance of `who` to a new value.
    ///
    /// Note this will not maintain total issuance.
    fn get_ccy_code_from_id_code(ccy_id: &T::CurrencyId) -> CurrencyCode {
        TokenInfoRLookup::<T>::get(ccy_id)
    }

    /// This function will set constant fields
    fn set_fields(
        identifier: Did,
        ccy_id: T::CurrencyId,
        token_vc: vc::TokenVC,
        mut token_name: Vec<u8>,
    ) {
        let mut currency_code = token_vc.currency_code.to_vec();
        let current_block_no: BlockNumber = <frame_system::Module<T>>::block_number()
            .try_into()
            .ok()
            .unwrap_or_default();
        currency_code.retain(|val| *val != 0);
        token_name.retain(|val| *val != 0);
        TokenData::insert(
            token_vc.currency_code,
            TokenDetails {
                token_name,
                currency_code,
                decimal: token_vc.decimal,
                block_number: current_block_no,
            },
        );
        Accounts::<T>::mutate(identifier, token_vc.currency_code, |account_data| {
            account_data.nonce = did::Module::<T>::get_nonce_from_did(identifier);
        });
        Self::set_token_info(ccy_id, token_vc.currency_code);
    }

    /// This function will set the token related informations
    pub fn set_token_info(ccy_id: T::CurrencyId, ccy_code: CurrencyCode) {
        TokenInfo::<T>::insert(ccy_code, ccy_id);
        TokenInfoRLookup::<T>::insert(ccy_id, ccy_code);
    }

    /// This will return unique currency_id.
    fn get_currency_id() -> T::CurrencyId {
        let currency_id: T::CurrencyId = if let Some(id) = TokenCurrencyCounter::<T>::get() {
            let mut id_u64 = id.to_u64().unwrap_or(0);
            id_u64 += 1;
            T::CurrencyId::from_u64(id_u64).unwrap_or_default()
        } else {
            T::CurrencyId::from_u64(1_u64).unwrap_or_default()
        };
        currency_id
    }

    /// This will set currency_id.
    fn set_currency_id(currency_id: T::CurrencyId) {
        TokenCurrencyCounter::<T>::put(currency_id);
    }

    /// Get VC Owner
    fn get_vc_owner<G: codec::Decode + vc::HasVCId>(
        vc_struct: &vc::VC<T::Hash>,
    ) -> Result<T::AccountId, DispatchError> {
        let vc_property: G = vc::Module::<T>::get_vc::<G>(&vc_struct.vc_property)?;

        let (token_vc_struct, _) =
            if let Some((vc_struct, vc_status)) = vc::VCs::<T>::get(&vc_property.vc_id()) {
                (vc_struct, vc_status)
            } else {
                fail!(vc::Error::<T>::LinkedVCNotFound);
            };

        let owners_acc_id = did::Module::<T>::get_accountid_from_did(&token_vc_struct.owner)?;

        Ok(owners_acc_id)
    }

    // Get vc struct
    fn get_vc_struct(
        vc_id: &vc::VCid,
        vc_type: &vc::VCType,
        vc_type_error: Error<T>,
    ) -> Result<vc::VC<T::Hash>, DispatchError> {
        // ensure vc exists
        let (vc_struct, vc_status) = if let Some((vc_struct, vc_status)) = vc::VCs::<T>::get(&vc_id)
        {
            (vc_struct, vc_status)
        } else {
            fail!(vc::Error::<T>::VCIdDoesNotExist);
        };

        // ensure vc is active
        ensure!(
            vc_status.eq(&vc::VCStatus::Active),
            vc::Error::<T>::VCIsNotActive
        );

        // ensure vc_type
        ensure!(vc_struct.vc_type.eq(vc_type), vc_type_error);

        // ensure VC is unused
        ensure!(!vc_struct.is_vc_used, vc::Error::<T>::VCAlreadyUsed);

        Ok(vc_struct)
    }

    // Validate vc
    fn validate_vc(
        senders_acccount_id: &T::AccountId,
        vc_id: &vc::VCid,
        vc_type: &vc::VCType,
        vc_type_error: Error<T>,
    ) -> Result<vc::VC<T::Hash>, DispatchError> {
        let senders_did = did::Module::<T>::get_did_from_account_id(&senders_acccount_id);

        let vc_struct = Self::get_vc_struct(vc_id, vc_type, vc_type_error)?;

        // ensure sender has associated vc
        ensure!(
            senders_did.eq(&vc_struct.owner),
            vc::Error::<T>::DidNotRegisteredWithVC
        );

        Ok(vc_struct)
    }

    /// Set reserved balance of `who` to a new value, meanwhile enforce
    /// existential rule.
    ///
    /// Note this will not maintain total issuance, and the caller is expected
    /// to do it.
    fn set_reserved_balance(
        currency_code: CurrencyCode,
        who: &T::AccountId,
        balance: TokenBalance,
    ) {
        let did = did::Module::<T>::get_did_from_account_id(who);
        <Accounts<T>>::mutate(did, currency_code, |account_data| {
            account_data.data.reserved = balance
        });
    }

    // Update the account entry for `who` under `currency_id`, given the locks.
    // fn update_locks(currency_id: T::CurrencyId, who: &T::AccountId, locks: &[BalanceLock<T::Balance>]) {
    // 	// update account data
    // 	<Accounts<T>>::mutate(who, currency_id, |account_data| {
    // 		account_data.frozen = Zero::zero();
    // 		for lock in locks.iter() {
    // 			account_data.frozen = account_data.frozen.max(lock.amount);
    // 		}
    // 	});

    // 	// update locks
    // 	let existed = <Locks<T>>::contains_key(who, currency_id);
    // 	if locks.is_empty() {
    // 		<Locks<T>>::remove(who, currency_id);
    // 		if existed {
    // 			// decrease account ref count when destruct lock
    // 			frame_system::Module::<T>::dec_ref(who);
    // 		}
    // 	} else {
    // 		<Locks<T>>::insert(who, currency_id, locks);
    // 		if !existed {
    // 			// increase account ref count when initialize lock
    // 			frame_system::Module::<T>::inc_ref(who);
    // 		}
    // 	}
    // }

    /// Ensure the given sender is owner of the given currency
    fn ensure_token_owner(sender: &T::AccountId, currency_code: CurrencyCode) -> Result<Did, DispatchError> {
        let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
        let token_owner = Self::token_issuer(currency_code);
        ensure!(sender_did == token_owner, Error::<T>::NotAllowed);
        Ok(token_owner)
    }

    /// Gets updated token balance of owner
    /// Validate Whether balance can be set
    /// Also checks if overflow or underflow occurs
    fn get_updated_owner_balance(currency_code: CurrencyCode, token_owner: Did, dest: Did, amount: TokenBalance) -> Result<TokenBalance, DispatchError> {
        let owner_balance = Self::accounts(token_owner, currency_code).data.free;
        let dest_balance = Self::accounts(dest, currency_code).data.free;
        if amount > dest_balance {
            let difference = amount.checked_sub(dest_balance).ok_or(Error::<T>::TokenAmountOverflow)?;
            ensure!(difference <= owner_balance, Error::<T>::TokenAmountOverflow);
            let updated_owner_balance = owner_balance.checked_sub(difference).ok_or(Error::<T>::TokenAmountOverflow)?;
            Ok(updated_owner_balance)
        } else {
            let difference = dest_balance.checked_sub(amount).ok_or(Error::<T>::TokenAmountOverflow)?;
            let updated_owner_balance = owner_balance.checked_add(difference).ok_or(Error::<T>::TokenAmountOverflow)?;
            Ok(updated_owner_balance)
        }
    }

    /// Set token balance to given did
    /// Balance will be transfered from/to owner's did to keep total issuance same
    fn set_token_balance(currency_code: CurrencyCode, token_owner: Did, dest: Did, amount: TokenBalance) -> DispatchResult {
        let updated_owner_balance = Self::get_updated_owner_balance(currency_code, token_owner, dest, amount)?;
        let dest_acc = did::Module::<T>::get_accountid_from_did(&dest)?;
        let owner_acc = did::Module::<T>::get_accountid_from_did(&token_owner)?;
        Self::set_free_balance(currency_code, &dest_acc, amount);
        Self::set_free_balance(currency_code, &owner_acc, updated_owner_balance);
        Ok(())
    }
}

impl<T: Config> MultiCurrency<T::AccountId> for Module<T> {
    type CurrencyId = T::CurrencyId;
    type Balance = TokenBalance;

    fn minimum_balance(_: Self::CurrencyId) -> Self::Balance {
        Default::default()
    }

    fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        TotalIssuance::get(currency_code)
    }

    fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::accounts(did, currency_code).data.total()
    }

    fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::accounts(did, currency_code).data.free
    }

    // Ensure that an account can withdraw from their free balance given any
    // existing withdrawal restrictions like locks and vesting balance.
    // Is a no-op if amount to be withdrawn is zero.
    fn ensure_can_withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        let did = did::Module::<T>::get_did_from_account_id(who);
        let new_balance = Self::free_balance(currency_id, who)
            .checked_sub(amount)
            .ok_or(Error::<T>::BalanceTooLow)?;
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        ensure!(
            new_balance >= Self::accounts(did, currency_code).data.frozen(),
            Error::<T>::LiquidityRestrictions
        );
        Ok(())
    }

    /// Transfer some free balance from `from` to `to`.
    /// Is a no-op if value to be transferred is zero or the `from` is the same
    /// as `to`.
    fn transfer(
        currency_id: Self::CurrencyId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() || from == to {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, from, amount)?;

        let from_balance = Self::free_balance(currency_id, from);
        let to_balance = Self::free_balance(currency_id, to)
            .checked_add(amount)
            .ok_or(Error::<T>::BalanceOverflow)?;
        // Cannot underflow because ensure_can_withdraw check
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::set_free_balance(currency_code, from, from_balance - amount);
        Self::set_free_balance(currency_code, to, to_balance);

        Ok(())
    }

    /// Deposit some `amount` into the free balance of account `who`.
    ///
    /// Is a no-op if the `amount` to be deposited is zero.
    fn deposit(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }

        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let new_total = Self::total_issuance(currency_code)
            .checked_add(amount)
            .ok_or(Error::<T>::TotalIssuanceOverflow)?;
        TotalIssuance::insert(currency_code, new_total);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::set_free_balance(
            currency_code,
            who,
            Self::free_balance(currency_id, who) + amount,
        );

        Ok(())
    }

    fn withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, who, amount)?;

        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        // Cannot underflow because ensure_can_withdraw check
        TotalIssuance::mutate(currency_code, |v| *v -= amount);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::set_free_balance(
            currency_code,
            who,
            Self::free_balance(currency_id, who) - amount,
        );

        Ok(())
    }

    // Check if `value` amount of free balance can be slashed from `who`.
    fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
        if value.is_zero() {
            return true;
        }
        Self::free_balance(currency_id, who) >= value
    }

    /// Is a no-op if `value` to be slashed is zero.
    ///
    /// NOTE: `slash()` prefers free balance, but assumes that reserve balance
    /// can be drawn from in extreme circumstances. `can_slash()` should be used
    /// prior to `slash()` to avoid having to draw from reserved funds, however
    /// we err on the side of punishment if things are inconsistent
    /// or `can_slash` wasn't used appropriately.
    fn slash(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> Self::Balance {
        if amount.is_zero() {
            return amount;
        }
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let account = Self::accounts(did, currency_code);
        let free_slashed_amount = account.data.free.min(amount);
        // Cannot underflow becuase free_slashed_amount can never be greater than amount
        let mut remaining_slash = amount - free_slashed_amount;

        // slash free balance
        if !free_slashed_amount.is_zero() {
            // Cannot underflow becuase free_slashed_amount can never be greater than
            // account.free
            Self::set_free_balance(currency_code, who, account.data.free - free_slashed_amount);
        }

        // slash reserved balance
        if !remaining_slash.is_zero() {
            let reserved_slashed_amount = account.data.reserved.min(remaining_slash);
            // Cannot underflow due to above line
            remaining_slash -= reserved_slashed_amount;
            Self::set_reserved_balance(
                currency_code,
                who,
                account.data.reserved - reserved_slashed_amount,
            );
        }

        // Cannot underflow because the slashed value cannot be greater than total
        // issuance
        TotalIssuance::mutate(currency_code, |v| *v -= amount - remaining_slash);
        remaining_slash
    }
}

impl<T: Config> MultiCurrencyExtended<T::AccountId> for Module<T> {
    type Amount = T::Amount;

    fn update_balance(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        by_amount: Self::Amount,
    ) -> DispatchResult {
        if by_amount.is_zero() {
            return Ok(());
        }

        // Ensure this doesn't overflow. There isn't any traits that exposes
        // `saturating_abs` so we need to do it manually.
        let by_amount_abs = if by_amount == Self::Amount::min_value() {
            Self::Amount::max_value()
        } else {
            by_amount.abs()
        };

        let by_balance = TryInto::<Self::Balance>::try_into(by_amount_abs)
            .map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
        if by_amount.is_positive() {
            Self::deposit(currency_id, who, by_balance)
        } else {
            Self::withdraw(currency_id, who, by_balance).map(|_| ())
        }
    }
}

// impl<T: Config> MultiLockableCurrency<T::AccountId> for Module<T> {
// 	type Moment = T::BlockNumber;

// Set a lock on the balance of `who` under `currency_id`.
// Is a no-op if lock amount is zero.
// fn set_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) {
// 	if amount.is_zero() {
// 		return;
// 	}
// 	let mut new_lock = Some(BalanceLock { id: lock_id, amount });
// 	let mut locks = Self::locks(who, currency_id)
// 		.into_iter()
// 		.filter_map(|lock| {
// 			if lock.id == lock_id {
// 				new_lock.take()
// 			} else {
// 				Some(lock)
// 			}
// 		})
// 		.collect::<Vec<_>>();
// 	if let Some(lock) = new_lock {
// 		locks.push(lock)
// 	}
// 	Self::update_locks(currency_id, who, &locks[..]);
// }

// Extend a lock on the balance of `who` under `currency_id`.
// Is a no-op if lock amount is zero
// fn extend_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) {
// 	if amount.is_zero() {
// 		return;
// 	}
// 	let mut new_lock = Some(BalanceLock { id: lock_id, amount });
// 	let mut locks = Self::locks(who, currency_id)
// 		.into_iter()
// 		.filter_map(|lock| {
// 			if lock.id == lock_id {
// 				new_lock.take().map(|nl| BalanceLock {
// 					id: lock.id,
// 					amount: lock.amount.max(nl.amount),
// 				})
// 			} else {
// 				Some(lock)
// 			}
// 		})
// 		.collect::<Vec<_>>();
// 	if let Some(lock) = new_lock {
// 		locks.push(lock)
// 	}
// 	Self::update_locks(currency_id, who, &locks[..]);
// }

// fn remove_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId) {
// 	let mut locks = Self::locks(who, currency_id);
// 	locks.retain(|lock| lock.id != lock_id);
// 	Self::update_locks(currency_id, who, &locks[..]);
// }
//}

impl<T: Config> MultiReservableCurrency<T::AccountId> for Module<T> {
    /// Check if `who` can reserve `value` from their free balance.
    ///
    /// Always `true` if value to be reserved is zero.
    fn can_reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> bool {
        if value.is_zero() {
            return true;
        }
        Self::ensure_can_withdraw(currency_id, who, value).is_ok()
    }

    /// Slash from reserved balance, returning any amount that was unable to be
    /// slashed.
    ///
    /// Is a no-op if the value to be slashed is zero.
    fn slash_reserved(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if value.is_zero() {
            return value;
        }

        let reserved_balance = Self::reserved_balance(currency_id, who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let actual = reserved_balance.min(value);
        Self::set_reserved_balance(currency_code, who, reserved_balance - actual);
        TotalIssuance::mutate(currency_code, |v| *v -= actual);
        value - actual
    }

    fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        Self::accounts(did, currency_code).data.reserved
    }

    /// Move `value` from the free balance from `who` to their reserved balance.
    ///
    /// Is a no-op if value to be reserved is zero.
    fn reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> DispatchResult {
        if value.is_zero() {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, who, value)?;
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let account = Self::accounts(did, currency_code);
        Self::set_free_balance(currency_code, who, account.data.free - value);
        // Cannot overflow becuase total issuance is using the same balance type and
        // this doesn't increase total issuance
        Self::set_reserved_balance(currency_code, who, account.data.reserved + value);
        Ok(())
    }

    /// Unreserve some funds, returning any amount that was unable to be
    /// unreserved.
    ///
    /// Is a no-op if the value to be unreserved is zero.
    fn unreserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if value.is_zero() {
            return value;
        }
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let account = Self::accounts(did, currency_code);
        let actual = account.data.reserved.min(value);
        Self::set_reserved_balance(currency_code, who, account.data.reserved - actual);
        Self::set_free_balance(currency_code, who, account.data.free + actual);
        value - actual
    }

    /// Move the reserved balance of one account into the balance of another,
    /// according to `status`.
    ///
    /// Is a no-op if:
    /// - the value to be moved is zero; or
    /// - the `slashed` id equal to `beneficiary` and the `status` is
    ///   `Reserved`.
    fn repatriate_reserved(
        currency_id: Self::CurrencyId,
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> result::Result<Self::Balance, DispatchError> {
        if value.is_zero() {
            return Ok(value);
        }

        if slashed == beneficiary {
            return match status {
                BalanceStatus::Free => Ok(Self::unreserve(currency_id, slashed, value)),
                BalanceStatus::Reserved => {
                    Ok(value.saturating_sub(Self::reserved_balance(currency_id, slashed)))
                }
            };
        }
        let slashed_did = did::Module::<T>::get_did_from_account_id(slashed);
        let ben_did = did::Module::<T>::get_did_from_account_id(beneficiary);
        let currency_code = Self::get_ccy_code_from_id_code(&currency_id);
        let from_account = Self::accounts(slashed_did, currency_code);
        let to_account = Self::accounts(ben_did, currency_code);
        let actual = from_account.data.reserved.min(value);
        match status {
            BalanceStatus::Free => {
                Self::set_free_balance(currency_code, beneficiary, to_account.data.free + actual);
            }
            BalanceStatus::Reserved => {
                Self::set_reserved_balance(
                    currency_code,
                    beneficiary,
                    to_account.data.reserved + actual,
                );
            }
        }
        Self::set_reserved_balance(currency_code, slashed, from_account.data.reserved - actual);
        Ok(value - actual)
    }
}

// fn balance_to_token_balance(input: T::Balance) -> TokenBalance {
//     TryInto::<TokenBalance>::try_into(input).ok().unwrap_or_default()
// }

pub struct CurrencyAdapter<T, GetCurrencyId>(marker::PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> PalletCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
    T: Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    type Balance = TokenBalance;
    type PositiveImbalance = PositiveImbalance<T, GetCurrencyId>;
    type NegativeImbalance = NegativeImbalance<T, GetCurrencyId>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Module::<T>::total_balance(GetCurrencyId::get(), who)
    }

    fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
        Module::<T>::can_slash(GetCurrencyId::get(), who, value)
    }

    fn total_issuance() -> Self::Balance {
        let currency_id = GetCurrencyId::get();
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        Module::<T>::total_issuance(currency_code)
    }

    fn minimum_balance() -> Self::Balance {
        Zero::zero()
    }

    fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
        if amount.is_zero() {
            return PositiveImbalance::zero();
        }
        let currency_id = GetCurrencyId::get();
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        TotalIssuance::mutate(currency_code, |issued| {
            *issued = issued.checked_sub(amount).unwrap_or_else(|| {
                amount = *issued;
                Zero::zero()
            });
        });
        PositiveImbalance::new(amount)
    }

    fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
        if amount.is_zero() {
            return NegativeImbalance::zero();
        }
        let currency_id = GetCurrencyId::get();
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        TotalIssuance::mutate(currency_code, |issued| {
            *issued = issued.checked_add(amount).unwrap_or_else(|| {
                amount = Self::Balance::max_value() - *issued;
                Self::Balance::max_value()
            })
        });
        NegativeImbalance::new(amount)
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        Module::<T>::free_balance(GetCurrencyId::get(), who)
    }

    fn ensure_can_withdraw(
        who: &T::AccountId,
        amount: Self::Balance,
        _reasons: WithdrawReasons,
        _new_balance: Self::Balance,
    ) -> DispatchResult {
        Module::<T>::ensure_can_withdraw(GetCurrencyId::get(), who, amount)
    }

    fn transfer(
        source: &T::AccountId,
        dest: &T::AccountId,
        value: Self::Balance,
        _existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        <Module<T> as MultiCurrency<T::AccountId>>::transfer(
            GetCurrencyId::get(),
            &source,
            &dest,
            value,
        )
    }

    fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        if value.is_zero() {
            return (Self::NegativeImbalance::zero(), value);
        }

        let currency_id = GetCurrencyId::get();
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        let account = Module::<T>::accounts(did, currency_code);
        let free_slashed_amount = account.data.free.min(value);
        let mut remaining_slash = value - free_slashed_amount;

        // slash free balance
        if !free_slashed_amount.is_zero() {
            Module::<T>::set_free_balance(
                currency_code,
                who,
                account.data.free - free_slashed_amount,
            );
        }

        // slash reserved balance
        if !remaining_slash.is_zero() {
            let reserved_slashed_amount = account.data.reserved.min(remaining_slash);
            remaining_slash -= reserved_slashed_amount;
            Module::<T>::set_reserved_balance(
                currency_code,
                who,
                account.data.reserved - reserved_slashed_amount,
            );
            (
                Self::NegativeImbalance::new(free_slashed_amount + reserved_slashed_amount),
                remaining_slash,
            )
        } else {
            (Self::NegativeImbalance::new(value), remaining_slash)
        }
    }

    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> result::Result<Self::PositiveImbalance, DispatchError> {
        if value.is_zero() {
            return Ok(Self::PositiveImbalance::zero());
        }
        let currency_id = GetCurrencyId::get();
        let new_total = Module::<T>::free_balance(currency_id, who)
            .checked_add(value)
            .ok_or(Error::<T>::TotalIssuanceOverflow)?;
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        Module::<T>::set_free_balance(currency_code, who, new_total);

        Ok(Self::PositiveImbalance::new(value))
    }

    fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        Self::deposit_into_existing(who, value).unwrap_or_else(|_| Self::PositiveImbalance::zero())
    }

    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        _reasons: WithdrawReasons,
        _liveness: ExistenceRequirement,
    ) -> result::Result<Self::NegativeImbalance, DispatchError> {
        if value.is_zero() {
            return Ok(Self::NegativeImbalance::zero());
        }
        let currency_id = GetCurrencyId::get();
        Module::<T>::ensure_can_withdraw(currency_id, who, value)?;
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);
        Module::<T>::set_free_balance(
            currency_code,
            who,
            Module::<T>::free_balance(currency_id, who) - value,
        );

        Ok(Self::NegativeImbalance::new(value))
    }

    fn make_free_balance_be(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        let did = did::Module::<T>::get_did_from_account_id(who);
        let currency_id = GetCurrencyId::get();
        let currency_code = Module::<T>::get_ccy_code_from_id_code(&currency_id);

        <Accounts<T>>::mutate(
            did,
            currency_code,
            |account| -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()> {
                let imbalance = if account.data.free <= value {
                    SignedImbalance::Positive(PositiveImbalance::new(value - account.data.free))
                } else {
                    SignedImbalance::Negative(NegativeImbalance::new(account.data.free - value))
                };
                account.data.free = value;
                Ok(imbalance)
            },
        )
        .unwrap_or_else(|_| SignedImbalance::Positive(Self::PositiveImbalance::zero()))
    }
}

impl<T, GetCurrencyId> PalletReservableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
    T: Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
        Module::<T>::can_reserve(GetCurrencyId::get(), who, value)
    }

    fn slash_reserved(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> (Self::NegativeImbalance, Self::Balance) {
        let actual = Module::<T>::slash_reserved(GetCurrencyId::get(), who, value);
        (Self::NegativeImbalance::zero(), actual)
    }

    fn reserved_balance(who: &T::AccountId) -> Self::Balance {
        Module::<T>::reserved_balance(GetCurrencyId::get(), who)
    }

    fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
        Module::<T>::reserve(GetCurrencyId::get(), who, value)
    }

    fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
        Module::<T>::unreserve(GetCurrencyId::get(), who, value)
    }

    fn repatriate_reserved(
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        status: Status,
    ) -> result::Result<Self::Balance, DispatchError> {
        Module::<T>::repatriate_reserved(GetCurrencyId::get(), slashed, beneficiary, value, status)
    }
}

// impl<T, GetCurrencyId> PalletLockableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
// where
// 	T: Config,
// 	GetCurrencyId: Get<T::CurrencyId>,
// {
// 	type Moment = T::BlockNumber;
// 	type MaxLocks = ();

// 	fn set_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
// 		Module::<T>::set_lock(id, GetCurrencyId::get(), who, amount)
// 	}

// 	fn extend_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
// 		Module::<T>::extend_lock(id, GetCurrencyId::get(), who, amount)
// 	}

// 	fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
// 		Module::<T>::remove_lock(id, GetCurrencyId::get(), who)
// 	}
// }
