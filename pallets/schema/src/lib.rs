#![cfg_attr(not(feature = "std"), no_std)]

use did;
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    StorageMap,
};
use frame_system::{self, ensure_signed};
use sp_std::prelude::*;

#[cfg(test)]
mod tests;

/// The SCHEMA trait
pub trait Config: frame_system::Config + did::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_event!(
    pub enum Event<T> where <T as frame_system::Config>::Hash {
        SchemaCreated(did::Did, Hash),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        //Schema already exists
        SchemaAlreadyExists,
        NotAValidator,
    }
}

decl_module! {

    pub struct Module<T: Config> for enum Call where origin: T::Origin {

        /// Deposit events
        fn deposit_event() = default;
        type Error = Error<T>;

        /// Function to create a new SCHEMA
        /// origin - the origin of the transaction
        /// hash - hash of the schema
        /// json_data - json data of schema as string
        #[weight = 1]
        pub fn add(origin, hash: T::Hash, json_data: Vec<u8>) -> DispatchResult {
            // origin of the transaction needs to be a signed sender account
            let sender = ensure_signed(origin)?;

            // ensure the caller is a validator account
            ensure!(did::Module::<T>::is_caller_validator(&sender), Error::<T>::NotAValidator);

            // check if SCHEMA already exists
            ensure!(!SCHEMA::<T>::contains_key(&hash), Error::<T>::SchemaAlreadyExists);

            // fetch the DID mapped to the origin accountId - cannot panic since the did has
            // already been confirmed in the previous step
            let caller_did = did::Module::<T>::get_did_from_account_id(&sender);

            // add SCHEMA to storage
            debug::print!("insert SCHEMA");
            <SCHEMA<T>>::insert(hash, (caller_did, json_data));
            // deposit event that the SCHEMA has been added
            Self::deposit_event(RawEvent::SchemaCreated(caller_did, hash));
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as Schema {
        // SCHEMA hash -> account_id, json
        pub SCHEMA get(fn schema):map hasher(opaque_blake2_256) T::Hash => Option<(did::Did, Vec<u8>)>;
    }
}
