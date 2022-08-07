#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::traits::EnsureOrigin;
/// The validator set pallet stores and manages the list of validators. These are DIDs that are permitted to perform
/// restricted actions. The validator_set pallet can be used to whitelist origin in any pallet
/// New members are added to the validator set pallet using the collective pallet (voting) or sudo (temporary)
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
};
use frame_system::{self as system};
use sp_std::prelude::*;

#[cfg(test)]
mod tests;

/// A maximum number of members. When membership reaches this number, no new members may join.
pub const MAX_MEMBERS: usize = 100;

// describe DID type, not importing from did pallet to avoid circular dependency
// TODO : move commonly used traits and types to a separate common module
pub type Did = [u8; 32];

pub trait Config: system::Config {
    type Event: From<Event> + Into<<Self as system::Config>::Event>;
    /// Origin from which approvals must come.
    type ApproveOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
    trait Store for Module<T: Config> as ValidatorSet {
        // simple list of all Dids permitted to perform validator actions
        Members get(fn members): Vec<Did>;
    }
    add_extra_genesis {
        config(validators): Vec<Did>;
        build(|config: &GenesisConfig | {
            <Module<T>>::initialize_validators(&config.validators)
        })
    }
}

decl_event!(
    pub enum Event {
        /// Added a member
        MemberAdded(Did),
        /// Removed a member
        MemberRemoved(Did),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Cannot join as a member because you are already a member
        AlreadyMember,
        /// Cannot give up membership because you are not currently a member
        NotMember,
        /// Cannot add another member because the limit is already reached
        MembershipLimitReached,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        type Error = Error<T>;

        /// Adds a member to the membership set
        #[weight = 1]
        fn add_member(origin, new_member : Did) -> DispatchResult {
            // let sender = ensure_signed(origin)?;

            // Check if the origin of the call is approved orgin or not
            T::ApproveOrigin::ensure_origin(origin)?;

            let mut members = Members::get();

            // ensure max member count is not reached
            ensure!(members.len() < MAX_MEMBERS, Error::<T>::MembershipLimitReached);

            match members.binary_search(&new_member) {
                // If the search succeeds, the caller is already a member, so just return error
                Ok(_) => Err(Error::<T>::AlreadyMember.into()),
                // If the search fails, the caller is not a member and we learned the index where
                // they should be inserted
                Err(index) => {
                    members.insert(index, new_member.clone());
                    Members::put(members);
                    Self::deposit_event(Event::MemberAdded(new_member));
                    Ok(())
                }
            }
        }

        /// Removes a member.
        #[weight = 1]
        fn remove_member(origin, old_member: Did) -> DispatchResult {
            // Check if the origin of the call is allowed orgin or not
            T::ApproveOrigin::ensure_origin(origin)?;

            let mut members = Members::get();

            // We have to find out where, in the sorted vec the member is, if anywhere.
            match members.binary_search(&old_member) {
                // If the search succeeds, the caller is a member, so remove her
                Ok(index) => {
                members.remove(index);
                Members::put(members);
                Self::deposit_event(Event::MemberRemoved(old_member));
                Ok(())
                },
                // If the search fails, the caller is not a member, so just return
                Err(_) => Err(Error::<T>::NotMember.into()),
            }
        }
    }
}

impl<T: Config> Module<T> {
    // function to check if a DID is a member of the validator set
    pub fn is_did_validator(x: Did) -> bool {
        Members::get().contains(&x)
    }

    // load initial list of validators from genesis
    fn initialize_validators(validators: &Vec<Did>) {
        let mut initial_data: Vec<Did> = vec![];
        for validator in validators.iter() {
            initial_data.push(validator.clone());
        }
        Members::put(initial_data);
    }
}
