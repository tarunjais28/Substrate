#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    debug, decl_event, decl_module, decl_storage, dispatch::DispatchResult, StorageMap,
};
use frame_system::{self, ensure_signed};
use sp_std::prelude::*;

/// The VC trait
pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_event!(
    //Events for verified_credentials
    pub enum Event<T> where <T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash {
        // new verified credential issued
        VCIssued(AccountId, Hash, Hash),
    }
);

decl_module! {
    // runtime module for VC issuance
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Issue a new VC and insert the hash on chain
        /// origin - VC issuer
        /// schema - hash of vc schema
        /// vc_hash - hash of the vc issued
        #[weight = 1]
        pub fn add(origin, schema: T::Hash, vc_hash: T::Hash) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            debug::RuntimeLogger::init();
            debug::print!("Here- Issuing VC");
            <VC<T>>::insert(vc_hash, (sender.clone(), schema));

            Self::deposit_event(RawEvent::VCIssued(sender, schema, vc_hash));
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as VC {
        VC get(fn vc): map hasher(opaque_blake2_256) T::Hash => Option<(T::AccountId, T::Hash)>;
    }
}
