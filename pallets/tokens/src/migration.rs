use super::*;
use crate::structs::V1TokenDetails;

pub mod deprecated {
    use super::*;
    use crate::Config;
    use frame_support::{decl_module, decl_storage};

    decl_storage! {
        trait Store for Module<T: Config> as Tokens {
            pub V1TokenData get(fn v1_token_data) : map hasher(blake2_128_concat) T::CurrencyId => super::V1TokenDetails;
            pub TokenData get(fn token_data) : map hasher(blake2_128_concat) T::CurrencyId => TokenDetails;
            pub Locks get(fn locks): double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) T::CurrencyId => Vec<BalanceLock<TokenBalance>>;
            pub TokenIssuer get(fn token_issuer): map hasher(blake2_128_concat) T::CurrencyId => Did;
            pub Accounts get(fn accounts): double_map hasher(blake2_128_concat) did::Did, hasher(twox_64_concat) T::CurrencyId => TokenAccountInfo<T::Index ,TokenAccountData>;
            pub TotalIssuance get(fn total_issuance): map hasher(twox_64_concat) T::CurrencyId => TokenBalance;
        }
    }
    decl_module! {
        pub struct Module<T: Config> for enum Call where origin: T::Origin {}
    }
}

pub fn migrate<T: Config>() -> frame_support::weights::Weight {
    frame_support::debug::RuntimeLogger::init();
    // Storage migrations should use storage versions for safety.
    match PalletVersion::get() {
        StorageVersion::V1_0_0 => {
            let current_block_no: BlockNumber = <frame_system::Module<T>>::block_number()
                .try_into()
                .ok()
                .unwrap_or_default();
            // We transform the storage values from the old into the new format.
            deprecated::TokenData::<T>::translate(
                |_: T::CurrencyId, token_data: V1TokenDetails| {
                    Some(TokenDetails {
                        token_name: token_data.token_name,
                        currency_code: token_data.currency_code,
                        decimal: token_data.decimal,
                        block_number: current_block_no,
                    })
                },
            );

            // Update storage version.
            PalletVersion::put(StorageVersion::V2_0_0);
            // Very inefficient, mostly here for illustration purposes.
            let count = TokenData::iter().count();

            // Return the weight consumed by the migration.
            T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
        }
        StorageVersion::V2_0_0 => {
            let mut new_token_datas = Vec::new();
            for (ccy_id, token_details) in deprecated::TokenData::<T>::drain() {
                let ccy_code: CurrencyCode =
                    convert_to_array::<8>(token_details.currency_code.clone());
                Module::<T>::set_token_info(ccy_id, ccy_code);
                new_token_datas.push((ccy_code, token_details));
            }
            for (ccy_code, token_details) in new_token_datas {
                TokenData::insert(ccy_code, token_details);
            }

            let mut new_token_issuers = Vec::new();
            for (ccy_id, token_issuer) in deprecated::TokenIssuer::<T>::drain() {
                let ccy_code = TokenInfoRLookup::<T>::get(ccy_id);
                new_token_issuers.push((ccy_code, token_issuer));
            }
            for (ccy_code, token_issuer) in new_token_issuers {
                TokenIssuer::insert(ccy_code, token_issuer);
            }

            let mut new_locks = Vec::new();
            for (acc_id, ccy_id, bal_dets) in deprecated::Locks::<T>::drain() {
                let ccy_code = TokenInfoRLookup::<T>::get(ccy_id);
                new_locks.push((acc_id, ccy_code, bal_dets));
            }
            for (acc_id, ccy_code, bal_dets) in new_locks {
                Locks::<T>::insert(acc_id, ccy_code, bal_dets);
            }

            let mut new_accounts = Vec::new();
            for (did, ccy_id, value) in deprecated::Accounts::<T>::drain() {
                let ccy_code = TokenInfoRLookup::<T>::get(ccy_id);
                new_accounts.push((did, ccy_code, value));
            }
            for (did, ccy_code, value) in new_accounts {
                Accounts::<T>::insert(did, ccy_code, value);
            }

            let mut new_token_issuance = Vec::new();
            for (ccy_id, amount) in deprecated::TotalIssuance::<T>::drain() {
                let ccy_code = TokenInfoRLookup::<T>::get(ccy_id);
                new_token_issuance.push((ccy_code, amount));
            }
            for (ccy_code, amount) in new_token_issuance {
                TotalIssuance::insert(ccy_code, amount);
            }

            // Update storage version.
            PalletVersion::put(StorageVersion::V3_0_0);

            let count = TokenData::iter().count();
            // Return the weight consumed by the migration.
            T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
        }
        _ => {
            frame_support::debug::info!(" >>> Unused migration!");
            0
        }
    }
}

fn convert_to_array<const N: usize>(mut v: Vec<u8>) -> [u8; N] {
    if v.len() != N {
        for _ in v.len()..N {
            v.push(0);
        }
    }
    v.try_into().unwrap_or_else(|_| [0; N])
}
