#![cfg_attr(not(feature = "std"), no_std)]
/// The VC pallet issues list of VCs that empowers any user to perfom permitted operations.
use frame_support::{
    codec::{Decode, Encode},
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure, fail,
    traits::EnsureOrigin,
    weights::Weight,
    StorageMap,
    traits::Get,
};
use frame_system::{self, ensure_signed};
use sp_core::sr25519;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, Verify},
    DispatchError,
};
use sp_std::{prelude::*, vec};
use sr25519::Signature;

#[cfg(test)]
mod tests;
mod migration;

pub mod structs;
pub use crate::structs::*;

// describe DID type, not importing from did pallet to avoid circular dependency
pub type Did = [u8; 32];
pub type VCid = [u8; 32];
pub type VCHash = Vec<u8>;
pub type PublicKey = sr25519::Public;

pub trait Config: frame_system::Config + validator_set::Config + did::Config {
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
    /// Origin from which approvals must come.
    type ApproveOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
    trait Store for Module<T: Config> as VC {
        /// the map for storing VC information
        pub VCs: map hasher(blake2_128_concat) VCid => Option<(VC<T::Hash>, VCStatus)>;
        /// map to enable lookup from Did to VCids
        pub Lookup: map hasher(blake2_128_concat) Did => Vec<VCid>;
        /// map to enable reverse lookup from VCid to Did
        pub RLookup: map hasher(blake2_128_concat) VCid => Did;
        /// the map for storing history of VC
        pub VCHistory: map hasher(blake2_128_concat) VCid => Option<(VCStatus,T::BlockNumber)>;
        /// map for vc id and approvers list
        pub VCApproverList: map hasher(blake2_128_concat) VCid => Vec<Did>;
        /// The current version of the pallet
        PalletVersion: VCPalletVersion = VCPalletVersion::V1_0_0
    }
    // add_extra_genesis {
    //     config(init_vcs): Vec<InitialVCs>;
    //     build(|config: &GenesisConfig | {
    //         <Module<T>>::initialize_vcs(&config.init_vcs)
    //     })
    // }
}

decl_event!(
    pub enum Event {
        /// Given VC is validated
        VCValidated(VCid),
        /// Updated VC status flag
        VCStatusUpdated(VCid, VCStatus),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Unable to decode the VC
        InvalidVC,
        /// VC properties verification failed
        VCPropertiesNotVerified,
        /// The given VCId does not exist on chain
        VCIdDoesNotExist,
        /// The operation is permitted only for issuer & validator
        NotAValidatorNorIssuer,
        /// VC is not owned by the given DID
        DidNotRegisteredWithVC,
        /// VC is already used, can't reused
        VCAlreadyUsed,
        /// VC status is Inactive, cant be use it
        VCIsNotActive,
        /// Linked VC does not exist
        LinkedVCNotFound,
        /// The given type of VC should be signed by the owner of respective TokenVC
        VCNotSignedByTokenVCOwner,
        /// VC Already Exist
        VCAlreadyExists,
        /// Either signature is invalid or signer is not a valid issuer 
        InvalidSignature,
        /// The issuer has already approved the VC
        DuplicateSignature
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        type Error = Error<T>;

        /// Adds a member to the membership set
        #[weight = 1]
        pub fn store(origin, vc_hex: VCHash) -> DispatchResult {
            // Extracting vc from encoded vc byte array
            let vc: VC<T::Hash> = Self::get_vc(&vc_hex)?;

            match vc.vc_type {
                VCType::TokenVC => {
                    // Check if the origin of the call is approved orgin or not
                    <T as Config>::ApproveOrigin::ensure_origin(origin)?;
                    // Check if owner's did is registered or not
                    let _ = did::Module::<T>::get_accountid_from_did(&vc.owner)?;
                }
                VCType::SlashTokens | VCType::MintTokens | VCType::TokenTransferVC => {
                    // Validating owner of slash or token vc is one of the issuers or not
                    Self::validate_vcs(&vc)?;
                }
            }

            // Generating vc_id from vc to emit in the event
            let vc_id: VCid = *BlakeTwo256::hash_of(&vc).as_fixed_bytes();
            // storing hash
            Self::store_vc(vc.owner, vc, vc_id)?;
            Self::deposit_event(Event::VCValidated(vc_id));
            Ok(())
        }

        /// Update signature of vc_hash to update status as Active or Inactive
        ///
        /// This function will set vc status as Active only if all issuers's signatures are verified
        #[weight = 1]
        fn add_signature(origin, vc_id: VCid, sign: Signature) -> DispatchResult {
            // Ensure caller is signed account
            let senders_acccount_id = ensure_signed(origin)?;

            Self::validate_updater(&senders_acccount_id, &vc_id)?;

            let (mut vc, _) = if let Some(vcs_details) = VCs::<T>::get(vc_id) {
                (vcs_details.0, vcs_details.1)
            } else {
                fail!(Error::<T>::VCIdDoesNotExist)
            };

            Self::validate_sign(&vc, sign.clone(), vc_id)?;

            vc.signatures.push(sign);

            Self::update_vc_and_status(vc_id, vc)?;
            Ok(())
        }

        /// Update status of vc_hash wheather it is active or inactive
        #[weight = 1]
        fn update_status(origin, vc_id: VCid, vc_status: VCStatus) -> DispatchResult {
            // Ensure caller is signed account
            let senders_acccount_id = ensure_signed(origin)?;

            Self::validate_updater(&senders_acccount_id, &vc_id)?;

            Self::update_vc_status(vc_id, vc_status)?;

            Ok(())
        }

        fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migration::migrate::<T>()
		}
    }
}

impl<T: Config> Module<T> {
    /// Decoding VC from encoded bytes
    pub fn get_vc<E: codec::Decode>(mut vc_bytes: &[u8]) -> Result<E, DispatchError> {
        let vc: E = match Decode::decode(&mut vc_bytes) {
            Ok(vc) => vc,
            Err(_) => fail!(Error::<T>::InvalidVC),
        };
        Ok(vc)
    }

    /// Validate updater
    fn validate_updater(
        senders_acccount_id: &T::AccountId,
        vc_id: &VCid,
    ) -> Result<(), DispatchError> {
        let senders_did = did::Module::<T>::get_did_from_account_id(senders_acccount_id);
        // Ensure either sender is one of the issuer or member of validator set
        if let Some((vc, _)) = VCs::<T>::get(vc_id) {
            if !vc.issuers.contains(&senders_did)
                && !validator_set::Module::<T>::is_did_validator(senders_did)
            {
                fail!(Error::<T>::NotAValidatorNorIssuer);
            }
        };
        Ok(())
    }

    /// Validate slash/token vc
    fn validate_vcs(vc: &VC<T::Hash>) -> Result<(), DispatchError> {
        match vc.vc_type {
            // derive slash/token vc
            VCType::SlashTokens | VCType::MintTokens => {
                let slash_or_mint: SlashMintTokens =
                    Self::get_vc::<SlashMintTokens>(&vc.vc_property)?;

                let (token_vc_struct, _) =
                    if let Some((vc_struct, vc_status)) = VCs::<T>::get(&slash_or_mint.vc_id) {
                        (vc_struct, vc_status)
                    } else {
                        fail!(Error::<T>::LinkedVCNotFound);
                    };

                ensure!(
                    vc.issuers.contains(&token_vc_struct.owner),
                    Error::<T>::VCNotSignedByTokenVCOwner
                );
            }
            VCType::TokenTransferVC => {
                // derive Transfer Tokens
                let transfer_tokens: TokenTransferVC =
                    Self::get_vc::<TokenTransferVC>(&vc.vc_property)?;

                let (token_vc_struct, _) =
                    if let Some((vc_struct, vc_status)) = VCs::<T>::get(&transfer_tokens.vc_id) {
                        (vc_struct, vc_status)
                    } else {
                        fail!(Error::<T>::LinkedVCNotFound);
                    };

                ensure!(
                    vc.issuers.contains(&token_vc_struct.owner),
                    Error::<T>::VCNotSignedByTokenVCOwner
                );
            }
            _ => (),
        }

        Ok(())
    }

    // // load initial list of validators from genesis
    // fn initialize_vcs(init_vcs: &Vec<InitialVCs>) {
    //     for init_vc in init_vcs.iter() {
    //         let block_no: T::BlockNumber = 0u32.into();

    //         VCs::<T>::insert(init_vc.identifier.clone(), (init_vc.vcs.clone(), block_no));

    //         let account_id = did::Module::<T>::get_accountid_from_pubkey(&init_vc.public_key);
    //         for vc in init_vc.vcs.iter() {
    //             Lookup::<T>::insert(vc, account_id.clone());
    //         }

    //         RLookup::<T>::insert(account_id.clone(), init_vc.vcs.clone());
    //         Members::put(init_vc.vcs.clone());
    //         DIDs::<T>::insert(account_id, init_vc.identifier);
    //     }
    // }

    /// Validating VC
    pub fn get_vc_status(vc: &VC<T::Hash>) -> Result<VCStatus, DispatchError> {
        let hash = T::Hashing::hash_of(&(&vc.vc_type, &vc.vc_property, &vc.owner, &vc.issuers));
        // ensure the valid hash
        ensure!(vc.hash.eq(&hash), Error::<T>::VCPropertiesNotVerified);

        // checking for duplicate issuers
        let mut issuers = vc.issuers.clone();
        let org_issuer_count = issuers.len();
        issuers.sort();
        issuers.dedup();
        if org_issuer_count != issuers.len() {
            fail!(Error::<T>::DuplicateSignature);
        }

        // checking for duplicate signatures
        let signatures = vc.signatures.clone();
        for i in 0..(signatures.len() - 1) {
            for j in (i + 1)..signatures.len() {
                if signatures[i] == signatures[j] {
                    fail!(Error::<T>::DuplicateSignature);
                }
            }
        }

        // ensure the caller has all issuers' signature
        if vc.issuers.len() != vc.signatures.len() {
            return Ok(VCStatus::Inactive);
        } else {
            let mut verified_count: usize = 0;
            for issuer in vc.issuers.iter() {
                let (issuer_details, _) = did::Module::<T>::get_did_details(*issuer)?;
                for signature in vc.signatures.iter() {
                    if signature.verify(vc.hash.as_ref(), &issuer_details.public_key) {
                        verified_count += 1;
                    }
                }
            }
            if verified_count != vc.signatures.len() {
                return Ok(VCStatus::Inactive);
            }
        }
        Ok(VCStatus::Active)
    }

    /// Store VC
    fn store_vc(identifier: Did, vc: VC<T::Hash>, vc_id: VCid) -> Result<(), DispatchError> {
        let current_block_no = <frame_system::Module<T>>::block_number();
        let vc_status = Self::get_vc_status(&vc)?;

        // Check if vc already exists
        ensure!(!RLookup::contains_key(&vc_id), Error::<T>::VCAlreadyExists);
        
        Self::set_approved_issuers(vc_id, &vc)?;

        VCs::<T>::insert(vc_id, (vc, vc_status));
        RLookup::insert(vc_id, identifier);

        if Lookup::contains_key(&identifier) {
            let mut vc_ids = Lookup::get(identifier);
            vc_ids.push(vc_id);
            Lookup::insert(identifier, vc_ids);
        } else {
            Lookup::insert(identifier, vec![vc_id]);
        }

        VCHistory::<T>::insert(vc_id, (vc_status, current_block_no));

        Ok(())
    }

    /// Update VC from storage
    fn update_vc_status(vc_id: VCid, status: VCStatus) -> Result<(), DispatchError> {
        if let Some(vcs_details) = VCs::<T>::get(&vc_id) {
            VCs::<T>::insert(vc_id, (vcs_details.0, status));
        } else {
            fail!(Error::<T>::VCIdDoesNotExist);
        }

        if let Some(vc_history) = VCHistory::<T>::get(&vc_id) {
            VCHistory::<T>::insert(vc_id, (status, vc_history.1));
        }
        Self::deposit_event(Event::VCStatusUpdated(vc_id, status));
        Ok(())
    }

    // Update VC and vc_status from storage
    fn update_vc_and_status(vc_id: VCid, updated_vc: VC<T::Hash>) -> Result<(), DispatchError> {
        let status = Self::get_vc_status(&updated_vc)?;
        VCs::<T>::insert(vc_id, (updated_vc, status));

        if let Some(vc_history) = VCHistory::<T>::get(&vc_id) {
            VCHistory::<T>::insert(vc_id, (status, vc_history.1));
        }

        Self::deposit_event(Event::VCStatusUpdated(vc_id, status));
        Ok(())
    }

    /// Update vc's is_used flag to true
    pub fn set_is_used_flag(vc_id: VCid) {
        if let Some((mut vc, status)) = VCs::<T>::get(&vc_id) {
            vc.is_vc_used = true;
            VCs::<T>::insert(vc_id, (vc, status));
        }
    }

    // Validate sign
    fn validate_sign(vc: &VC<T::Hash>, sign: Signature, vc_id: VCid) -> Result<(), DispatchError> {
        let mut is_sign_valid = false;
        let mut vc_approver_list = VCApproverList::get(vc_id);
        for issuer in vc.issuers.iter() {
            let (issuer_details, _) = did::Module::<T>::get_did_details(*issuer)?;
            if sign.verify(vc.hash.as_ref(), &issuer_details.public_key) {
                if vc_approver_list.contains(&issuer_details.identifier) {
                    fail!(Error::<T>::DuplicateSignature);
                }
                vc_approver_list.push(issuer_details.identifier);
                is_sign_valid = true;
            }
        }
        if !is_sign_valid {
            fail!(Error::<T>::InvalidSignature);
        }
        VCApproverList::insert(vc_id, vc_approver_list);
        Ok(())
    }

    fn set_approved_issuers(vc_id: VCid, vc: &VC<T::Hash>) -> Result<(), DispatchError> {
        let mut vc_approver_list = VCApproverList::get(vc_id);
        let signatures = vc.signatures.clone();
        // Check approved signatures
        for i in 0..signatures.len() {
            let sign = &signatures[i];
            let mut is_sign_valid = false;
            for issuer in vc.issuers.iter() {
                let (issuer_details, _) = did::Module::<T>::get_did_details(*issuer)?;
                if sign.verify(vc.hash.as_ref(), &issuer_details.public_key) {
                    if vc_approver_list.contains(&issuer_details.identifier) {
                        fail!(Error::<T>::DuplicateSignature);
                    }
                    is_sign_valid = true;
                    vc_approver_list.push(issuer_details.identifier);
                }
            }
            if !is_sign_valid {
                fail!(Error::<T>::InvalidSignature);
            }
        }
        VCApproverList::insert(vc_id, vc_approver_list);
        Ok(())
    }
}
