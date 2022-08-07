#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
    codec::{Decode, Encode},
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, fail,
    traits::StoredMap,
    StorageMap,
};
use frame_system::{self, ensure_signed, split_inner};
use sp_core::sr25519;
use sp_runtime::traits::{LookupError, StaticLookup, StoredMapError};
use sp_runtime::{codec::Codec, RuntimeDebug};
use sp_std::prelude::*;
use validator_set;

mod multiaddress;
pub use multiaddress::MultiAddress;

#[cfg(feature = "std")]
pub use serde;

// Tests for DID module
#[cfg(test)]
mod tests;

/// Type used to encode the number of references an account has.
pub type RefCount = u32;

/// Information of an account.
#[derive(Clone, Eq, PartialEq, Default, RuntimeDebug, Encode, Decode)]
pub struct AccountInfo<Index, AccountData> {
    /// The number of transactions this account has sent.
    pub nonce: Index,
    /// The number of other modules that currently depend on this account's existence. The account
    /// cannot be reaped until this is zero.
    pub refcount: RefCount,
    /// The additional data that belongs to this account. Used to store the balance(s) in a lot of
    /// chains.
    pub data: AccountData,
}

/// The DID trait
pub trait Config: frame_system::Config + validator_set::Config {
    /// DID specific event type
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
}

/// type of the did identifier to be used
/// set to raw bytes, might need to optimise later
pub type Did = [u8; 32];

// type of public key used within the DID
// currently only supporting single key type - can add struct in future
// in application - this will just be the AccountId of the signing account for now
pub type PublicKey = sr25519::Public;

// use signature type from sr25519
pub type DiDSignature = sr25519::Signature;

/// Struct to store the details of each DID
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DidStruct {
    pub identifier: Did,
    pub public_key: PublicKey,
    pub metadata: Vec<u8>,
}

decl_event!(
    /// Events for DIDs
    pub enum Event {
        /// A DID has been created
        DidCreated(Did),
        /// A DID has been removed
        DidRemoved(Did),
        /// DID key have been rotated
        DidKeyUpdated(Did),
        /// DID Metadata has been updated
        DidMetadataUpdated(Did),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// The given DID already exists on chain
        DIDAlreadyExists,
        /// Invalid DID, either format or length is wrong
        InvalidDid,
        /// PublicKey already linked to another DID on chain
        PublicKeyRegistered,
        /// The given DID does not exist on chain
        DIDDoesNotExist,
        /// The operation is restricted to the validator only
        NotAValidator,
    }
}

decl_module! {
    /// The DID runtime module
    pub struct Module<T: Config> for enum Call where origin: T::Origin {

        /// Deposit events
        fn deposit_event() = default;
        type Error = Error<T>;

        /// Adds a DID on chain, where
        /// origin - the origin of the transaction
        /// sign_key - public signing key of the DID
        /// identifier - public unique identifier for the DID
        /// metadata - optional metadata to the DID - meant for bank nodes to display URL
        #[weight = 1]
        pub fn add(origin, public_key: PublicKey, identifier : Did, metadata: Vec<u8>) -> DispatchResult {
            // origin of the transaction needs to be a signed sender account
            let sender = ensure_signed(origin)?;

            // ensure the caller is a validator account
            ensure!(Self::is_caller_validator(&sender), Error::<T>::NotAValidator);

            // ensure did is valid
            ensure!(Self::is_did_valid(identifier.clone()), Error::<T>::InvalidDid);

            // ensure did is not already taken
            ensure!(!DIDs::<T>::contains_key(identifier.clone()), Error::<T>::DIDAlreadyExists);

            // ensure the public key is not already linked to a DID
            ensure!(!RLookup::<T>::contains_key(Self::get_accountid_from_pubkey(&public_key)), Error::<T>::PublicKeyRegistered);

            let current_block_no = <frame_system::Module<T>>::block_number();

            // debug::info!("Current Block Number: {:?}", current_block_no);
            // debug::info!("MetaData: {:?}", metadata);

            // add DID to the storage
            DIDs::<T>::insert(identifier.clone(), (DidStruct{
                identifier : identifier.clone(),
                public_key,
                metadata
            }, current_block_no));

            Lookup::<T>::insert(identifier.clone(), Self::get_accountid_from_pubkey(&public_key));
            RLookup::<T>::insert(Self::get_accountid_from_pubkey(&public_key), identifier.clone());

            // deposit an event that the DID has been created
            Self::deposit_event(Event::DidCreated(identifier));
            Ok(())
        }
        /// Removes a DID from chain storage, where
        /// origin - the origin of the transaction
        #[weight = 1]
        pub fn remove(origin, identifier : Did) -> DispatchResult {
            // origin of the transaction needs to be a signed sender account
            let sender = ensure_signed(origin)?;

            // ensure the caller is a validator account
            ensure!(Self::is_caller_validator(&sender), Error::<T>::NotAValidator);

            let (did_doc, _last_updated_block) = Self::get_did_details(identifier.clone())?;

            // remove DID from storage
            DIDs::<T>::remove(&identifier);
            Lookup::<T>::remove(identifier.clone());
            RLookup::<T>::remove(Self::get_accountid_from_pubkey(&did_doc.public_key));

            // deposit an event that the DID has been removed
            Self::deposit_event(Event::DidRemoved(identifier));
            Ok(())
        }
        /// Updates a DID public key on the chain
        /// origin - the origin of the transaction
        #[weight = 1]
        pub fn rotate_key(origin, identifier : Did, public_key: PublicKey) -> DispatchResult{
            let sender = ensure_signed(origin)?;

            // ensure the caller is a validator account
            ensure!(Self::is_caller_validator(&sender), Error::<T>::NotAValidator);

            //reject if the user does not already have DID registered
            ensure!(DIDs::<T>::contains_key(&identifier), Error::<T>::DIDDoesNotExist);

            // ensure the public key is not already linked to a DID
            ensure!(!RLookup::<T>::contains_key(Self::get_accountid_from_pubkey(&public_key)), Error::<T>::PublicKeyRegistered);

            // fetch the existing DID document
            let (did_doc, last_updated_block) = Self::get_did_details(identifier.clone())?;

            // Remove previous lookup of pubkey to DID
            RLookup::<T>::remove(Self::get_accountid_from_pubkey(&did_doc.public_key));

            // Store the previous key to history
            let mut prev_keys = Self::get_prev_key_details(identifier.clone())?;

            prev_keys.push((Self::get_accountid_from_pubkey(&did_doc.public_key), last_updated_block));

            PrevKeys::<T>::insert(identifier.clone(), prev_keys);

            let current_block_no = <frame_system::Module<T>>::block_number();

            // modify the public_key of the did doc
            DIDs::<T>::insert(identifier.clone(), (DidStruct{ public_key, ..did_doc }, current_block_no));
            Lookup::<T>::insert(identifier.clone(), Self::get_accountid_from_pubkey(&public_key));
            RLookup::<T>::insert(Self::get_accountid_from_pubkey(&public_key), identifier.clone());

            // create key updated event
            Self::deposit_event(Event::DidKeyUpdated(identifier));
            Ok(())
        }

        /// Updates DID metadata on the chain
        /// origin - the origin of the transaction
        #[weight = 1]
        pub fn update_metadata(origin, identifier: Did, metadata: Vec<u8>) -> DispatchResult{
            let sender = ensure_signed(origin)?;

            // ensure the caller is a validator account
            ensure!(Self::is_caller_validator(&sender), Error::<T>::NotAValidator);

            //reject if the user does not already have DID registered
            ensure!(DIDs::<T>::contains_key(&identifier), Error::<T>::DIDDoesNotExist);

            // fetch the existing DID document
            let (did_doc, block_number) = Self::get_did_details(identifier.clone())?;

            // modify the public_key of the did doc
            DIDs::<T>::insert(identifier.clone(), (DidStruct{ metadata: metadata, ..did_doc }, block_number));

            // create metadata updated event
            Self::deposit_event(Event::DidMetadataUpdated(identifier));
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as DID {
        // the map for storing did information
        DIDs: map hasher(blake2_128_concat) Did => Option<(DidStruct, T::BlockNumber)>;
        // map to enable lookup from did to account id
        Lookup: map hasher(blake2_128_concat) Did => Option<T::AccountId>;
        // map to enable reverse lookup
        RLookup : map hasher(blake2_128_concat) T::AccountId => Did;
        // map to store history of key rotation
        PrevKeys : map hasher(blake2_128_concat) Did => Option<Vec<(T::AccountId, T::BlockNumber)>>;
        // map to store account balances
        Account get(fn account):
            map hasher(blake2_128_concat) Did => AccountInfo<T::Index, T::AccountData>;
    }
    add_extra_genesis {
        config(dids): Vec<DidStruct>;
        build(|config: &GenesisConfig | {
            <Module<T>>::initialize_did(&config.dids)
        })
    }
}

impl<T: Config> Module<T> {
    // Function to check if an Account is included in the validator list
    pub fn is_caller_validator(caller: &T::AccountId) -> bool {
        let did_to_check = Self::get_did_from_account_id(caller);
        validator_set::Module::<T>::is_did_validator(did_to_check)
    }

    // Function to get nonce from did
    pub fn get_nonce_from_did(identifier: Did) -> T::Index {
        let account_details = Account::<T>::get(identifier);
        account_details.nonce
    }

    // Function to check if did which is going to be created is valid or not
    pub fn is_did_valid(identifier: Did) -> bool {
        let did_colon: [u8; 4] = [100, 105, 100, 58];
        let did_all_zeros: [u8; 32] = [0; 32];
        let did_four_zeros: [u8; 4] = [0; 4];
        let mut did_four_seg = identifier.chunks_exact(4);
        !identifier.is_empty()
            && identifier.ne(&did_all_zeros)
            && did_four_seg.next().eq(&Some(&did_colon))
            && !did_four_seg.next().eq(&Some(&did_four_zeros))
    }

    // get the details of the pubkey attached to the DID
    pub fn get_did_details(identifier: Did) -> Result<(DidStruct, T::BlockNumber), DispatchError> {
        // fetch did details and last updated block
        if let Some((did_doc, last_updated_block)) = DIDs::<T>::get(identifier) {
            Ok((did_doc, last_updated_block))
        } else {
            fail!(Error::<T>::DIDDoesNotExist)
        }
    }

    // get the details of the previous keys attached to the DID
    pub fn get_prev_key_details(
        identifier: Did,
    ) -> Result<Vec<(T::AccountId, T::BlockNumber)>, DispatchError> {
        // fetch did details and last updated block
        if let Some(prev_key_list) = PrevKeys::<T>::get(identifier) {
            Ok(prev_key_list)
        } else {
            Ok(vec![])
        }
    }

    // // NOTE : Not used currently, since the publickey is AccountID, depend on frame system signing checks
    // // for now
    // pub fn verify_signature_from_did(did : Did, message : &[u8], signature : &DiDSignature)
    // -> Result<bool, DispatchError> {
    // 	// cannot verify dids not in storage
    // 	ensure!(DIDs::contains_key(&did), Error::<T>::DIDDoesNotExist);
    // 	// fetch DID details from storage
    // 	let did_doc = DIDs::get(&did);
    // 	// verify signature
    // 	Self::verify_signature_from_pubkey(&did_doc.public_key, message, signature)
    // }

    // // NOTE : Not used currently, since the publickey is AccountID, depend on frame system signing checks
    // // for now
    // pub fn verify_signature_from_pubkey(pk : &PublicKey, message : &[u8], signature : &DiDSignature)
    // -> Result<bool, DispatchError> {
    // 	Ok(signature.verify(message, pk))
    // }

    // Simple type conversion between sr25519::Public and AccountId
    // Should not panic for any valid sr25519 - need to make more robust to check for valid publicKey
    pub fn get_accountid_from_pubkey(pk: &PublicKey) -> T::AccountId {
        //convert a publickey to an accountId
        // TODO : Need a better way to handle the option failing?
        T::AccountId::decode(&mut &pk[..]).unwrap_or_default()
    }

    // Check if the given DID is registered or not
    pub fn did_registered(x: &Did) -> bool {
        Lookup::<T>::contains_key(x)
    }

    // // NOTE : Not used currently, since the publickey is AccountID, depend on frame system signing checks
    // // for now
    // pub fn verify_signature_from_did(did : Did, message : &[u8], signature : &DiDSignature)
    // -> Result<bool, DispatchError> {
    // 	// cannot verify dids not in storage
    // 	ensure!(DIDs::contains_key(&did), Error::<T>::DIDDoesNotExist);
    // 	// fetch DID details from storage
    // 	let did_doc = DIDs::get(&did);
    // 	// verify signature
    // 	Self::verify_signature_from_pubkey(&did_doc.public_key, message, signature)
    // }

    // // NOTE : Not used currently, since the publickey is AccountID, depend on frame system signing checks
    // // for now
    // pub fn verify_signature_from_pubkey(pk : &PublicKey, message : &[u8], signature : &DiDSignature)
    // -> Result<bool, DispatchError> {
    // 	Ok(signature.verify(message, pk))
    // }

    pub fn on_created_account(_who: Did) {
        // T::OnNewAccount::on_new_account(&who);
        // Self::deposit_event(RawEvent::NewAccount(who));
    }

    fn on_killed_account(_who: Did) {
        // T::OnKilledAccount::on_killed_account(&who);
        // Self::deposit_event(RawEvent::KilledAccount(who));
    }

    // return the DID of an account for a given DID
    // no checks implemented here, assuming the caller checks for existence of DID
    // before this function is called
    pub fn get_did_from_account_id(x: &T::AccountId) -> Did {
        RLookup::<T>::get(x)
    }

    // check if an accountID is mapped to a DID
    pub fn does_did_exist(x: &T::AccountId) -> bool {
        RLookup::<T>::contains_key(x)
    }

    // return the AccountID for given DID, if it exists
    // the function returns result and should be handled by the caller
    pub fn get_accountid_from_did(x: &Did) -> Result<T::AccountId, DispatchError> {
        if let Some(account_id) = Lookup::<T>::get(x) {
            Ok(account_id)
        } else {
            fail!(Error::<T>::DIDDoesNotExist)
        }
    }

    fn initialize_did(dids: &Vec<DidStruct>) {
        for did in dids.iter() {
            let block_no: T::BlockNumber = 0u32.into();
            DIDs::<T>::insert(
                did.identifier.clone(),
                (
                    DidStruct {
                        identifier: did.identifier.clone(),
                        public_key: did.public_key,
                        metadata: vec![],
                    },
                    block_no,
                ),
            );
            Lookup::<T>::insert(
                did.identifier.clone(),
                Self::get_accountid_from_pubkey(&did.public_key),
            );
            RLookup::<T>::insert(
                Self::get_accountid_from_pubkey(&did.public_key),
                did.identifier.clone(),
            );
        }
    }
}

/// DIDResolve trait to enable easy verification and lookup from other pallets
/// Need to add more function to this as needed in other pallets
pub trait DidResolve<AccountId> {
    // return if an accountId is mapped to a DID
    fn did_exists(x: &AccountId) -> bool;
    // convert accountId to DID
    fn get_did_from_account_id(k: &AccountId) -> Did;
}

impl<T: Config> DidResolve<T::AccountId> for Module<T> {
    fn did_exists(x: &T::AccountId) -> bool {
        RLookup::<T>::contains_key(x)
    }

    fn get_did_from_account_id(k: &T::AccountId) -> Did {
        RLookup::<T>::get(k)
    }
}

// implement the lookup trait to fetch the accountid of the
// did from storage
impl<T: Config> StaticLookup for Module<T>
where
    MultiAddress<T::AccountId>: Codec,
{
    type Source = MultiAddress<T::AccountId>;
    type Target = T::AccountId;

    fn lookup(x: Self::Source) -> Result<Self::Target, LookupError> {
        match x {
            // Return if the source is accountId
            MultiAddress::Id(id) => Ok(id),
            // Fetch the accountId from storage if did is passed
            MultiAddress::Did(did) => Lookup::<T>::get(did).ok_or(LookupError),
        }
    }

    fn unlookup(x: Self::Target) -> Self::Source {
        MultiAddress::Id(x)
    }
}

// Implement StoredMap for a simple single-item, kill-account-on-remove system. This works fine for
// storing a single item which is required to not be empty/default for the account to exist.
// Anything more complex will need more sophisticated logic.
impl<T: Config> StoredMap<T::AccountId, T::AccountData> for Module<T> {
    fn get(k: &T::AccountId) -> T::AccountData {
        let did = Self::get_did_from_account_id(k);
        Account::<T>::get(did).data
    }

    fn insert(_: &T::AccountId, data: T::AccountData) -> Result<(), StoredMapError> {
        // Need to fix this, hardcoding since this function is only called during genesis
        // at genesis, the storage for lookup is not loaded
        // TODO : Enable dynamic lookup at genesis block time
        let did = *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let existed = Account::<T>::contains_key(did);
        let r = Account::<T>::mutate(did, |a| a.data = data);
        if !existed {
            Self::on_created_account(did.clone());
        }
        Ok(r)
    }
    fn remove(k: &T::AccountId) -> Result<(), StoredMapError> {
        // kill_account(k)
        Self::mutate_exists(k, |x| *x = None)
    }
    fn mutate<R>(
        k: &T::AccountId,
        f: impl FnOnce(&mut T::AccountData) -> R,
    ) -> Result<R, StoredMapError> {
        let did = Self::get_did_from_account_id(k);
        let existed = Account::<T>::contains_key(did);
        let r = Account::<T>::mutate(did, |a| f(&mut a.data));
        if !existed {
            Self::on_created_account(did.clone());
        }
        Ok(r)
    }
    fn mutate_exists<R>(
        k: &T::AccountId,
        f: impl FnOnce(&mut Option<T::AccountData>) -> R,
    ) -> Result<R, StoredMapError> {
        Self::try_mutate_exists(k, |x| -> Result<R, StoredMapError> { Ok(f(x)) })
    }
    fn try_mutate_exists<R, E>(
        k: &T::AccountId,
        f: impl FnOnce(&mut Option<T::AccountData>) -> Result<R, E>,
    ) -> Result<R, E> {
        let did = Self::get_did_from_account_id(k);
        Account::<T>::try_mutate_exists(did, |maybe_value| {
            let existed = maybe_value.is_some();
            let (maybe_prefix, mut maybe_data) = split_inner(maybe_value.take(), |account| {
                ((account.nonce, account.refcount), account.data)
            });
            f(&mut maybe_data).map(|result| {
                *maybe_value = maybe_data.map(|data| {
                    let (nonce, refcount) = maybe_prefix.unwrap_or_default();
                    AccountInfo {
                        nonce,
                        refcount,
                        data,
                    }
                });
                (existed, maybe_value.is_some(), result)
            })
        })
        .map(|(existed, exists, v)| {
            if !existed && exists {
                Self::on_created_account(did.clone());
            } else if existed && !exists {
                Self::on_killed_account(did.clone());
            }
            v
        })
    }
}
