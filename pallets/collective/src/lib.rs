#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]
#![allow(dead_code, unused_imports, unused_variables)]
use sp_core::u32_trait::Value as U32;
use sp_io::storage;
use sp_runtime::{traits::Hash, RuntimeDebug};
use sp_std::{prelude::*, result, vec};

// Tests for collective module
mod sudo;
#[cfg(test)]
mod tests;

use did;
use frame_support::{
    codec::{Decode, Encode},
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::{
        DispatchError, DispatchResult, DispatchResultWithPostInfo, Dispatchable, Parameter,
        PostDispatchInfo,
    },
    ensure,
    traits::{ChangeMembers, EnsureOrigin, Get, InitializeMembers},
    weights::{
        // DispatchClass,
        GetDispatchInfo,
        Weight,
    },
};
use frame_system::{self as system, ensure_root, ensure_signed};

pub type Did = [u8; 32];
/// Simple index type for proposal counting.
pub type ProposalIndex = u32;

/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each member may
/// vote exactly once, therefore also the number of votes for any given motion.
pub type MemberCount = u32;

/// Default voting strategy when a member is inactive.
pub trait DefaultVote {
    /// Get the default voting strategy, given:
    ///
    /// - Whether the prime member voted Aye.
    /// - Raw number of yes votes.
    /// - Raw number of no votes.
    /// - Total number of member count.
    fn default_vote(
        prime_vote: Option<bool>,
        yes_votes: MemberCount,
        no_votes: MemberCount,
        len: MemberCount,
    ) -> bool;
}

/// Set the prime member's vote as the default vote.
pub struct PrimeDefaultVote;

impl DefaultVote for PrimeDefaultVote {
    fn default_vote(
        prime_vote: Option<bool>,
        _yes_votes: MemberCount,
        _no_votes: MemberCount,
        _len: MemberCount,
    ) -> bool {
        prime_vote.unwrap_or(false)
    }
}

/// First see if yes vote are over majority of the whole collective. If so, set the default vote
/// as yes. Otherwise, use the prime meber's vote as the default vote.
pub struct MoreThanMajorityThenPrimeDefaultVote;

impl DefaultVote for MoreThanMajorityThenPrimeDefaultVote {
    fn default_vote(
        prime_vote: Option<bool>,
        yes_votes: MemberCount,
        _no_votes: MemberCount,
        len: MemberCount,
    ) -> bool {
        let more_than_majority = yes_votes * 2 > len;
        more_than_majority || prime_vote.unwrap_or(false)
    }
}

pub trait Config: frame_system::Config + did::Config {
    /// The outer origin type.
    type Origin: From<RawOrigin<Self::AccountId>>;

    // type DidResolution: DidResolve<Self::AccountId>;
    /// The outer call dispatch type.
    type Proposal: Parameter
        + Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
        + From<frame_system::Call<Self>>
        + GetDispatchInfo;

    /// The outer event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The time-out for council motions.
    type MotionDuration: Get<Self::BlockNumber>;

    /// Maximum number of proposals allowed to be active in parallel.
    type MaxProposals: Get<ProposalIndex>;

    /// The maximum number of members supported by the pallet. Used for weight estimation.
    ///
    /// NOTE:
    /// + Benchmarks will need to be re-run and weights adjusted if this changes.
    /// + This pallet assumes that dependents keep to the limit without enforcing it.
    type MaxMembers: Get<MemberCount>;

    /// Default vote strategy of this collective.
    type DefaultVote: DefaultVote;
}

/// Origin for the collective module.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub enum RawOrigin<Did> {
    /// It has been condoned by a given number of members of the collective from a given total.
    Members(MemberCount, MemberCount),
    /// It has been condoned by a single member of the collective.
    Member(Did),
    //// Dummy to manage the fact we have instancing.
    // _Phantom(sp_std::marker::PhantomData),
}

/// Origin for the collective module.
pub type Origin<T> = RawOrigin<<T as frame_system::Config>::AccountId>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Info for keeping track of a motion being voted on.
pub struct Votes<Did, BlockNumber> {
    /// The proposal's unique index.
    index: ProposalIndex,
    /// The number of approval votes that are needed to pass the motion.
    threshold: MemberCount,
    /// The current set of voters that approved it.
    ayes: Vec<Did>,
    /// The current set of voters that rejected it.
    nays: Vec<Did>,
    /// The hard end time of this vote.
    end: BlockNumber,
}

decl_storage! {
    trait Store for Module<T: Config> as Collective {
        /// The hashes of the active proposals.
        pub Proposals get(fn proposals): Vec<T::Hash>;
        /// Actual proposal for a given hash, if it's current.
        pub ProposalOf get(fn proposal_of):
            map hasher(identity) T::Hash => Option<<T as Config>::Proposal>;
        /// Votes on a given proposal, if it is ongoing.
        pub Voting get(fn voting):
            map hasher(identity) T::Hash => Option<Votes<Did, T::BlockNumber>>;
        /// Proposals so far.
        pub ProposalCount get(fn proposal_count): u32;
        /// The current members of the collective. This is stored sorted (just by value).
        pub Members get(fn members): Vec<Did>;
        /// The prime member that helps determine the default vote behavior in case of absentations.
        pub Prime get(fn prime): Option<Did>;
    }
    add_extra_genesis {
        // config(phantom): sp_std::marker::PhantomData<I>;
        config(members): Vec<Did>;
        build(|config: &GenesisConfig | {
            <Module<T>>::initialize_members(&config.members)
        })
    }
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Config>::Hash,
        // <T as frame_system::Trait>::AccountId,
    {
        /// A motion (given hash) has been proposed (by given account) with a threshold (given
        /// `MemberCount`).
        /// \[account, proposal_index, proposal_hash, threshold\]
        Proposed(Did, ProposalIndex, Hash, MemberCount),
        /// A motion (given hash) has been voted on by given account, leaving
        /// a tally (yes votes and no votes given respectively as `MemberCount`).
        /// \[account, proposal_hash, voted, yes, no\]
        Voted(Did, Hash, bool, MemberCount, MemberCount),
        /// A motion was approved by the required threshold.
        /// \[proposal_hash\]
        Approved(Hash),
        /// A motion was not approved by the required threshold.
        /// \[proposal_hash\]
        Disapproved(Hash),
        /// A motion was executed; result will be `Ok` if it returned without error.
        /// \[proposal_hash, result\]
        Executed(Hash, DispatchResult),
        /// A single member did some action; result will be `Ok` if it returned without error.
        /// \[proposal_hash, result\]
        MemberExecuted(Hash, DispatchResult),
        /// A proposal was closed because its threshold was reached or after its duration was up.
        /// \[proposal_hash, yes, no\]
        Closed(Hash, MemberCount, MemberCount),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Account is not a member
        NotMember,
        /// Duplicate proposals not allowed
        DuplicateProposal,
        /// Proposal must exist
        ProposalMissing,
        /// Mismatched index
        WrongIndex,
        /// Duplicate vote ignored
        DuplicateVote,
        /// Members are already initialized!
        AlreadyInitialized,
        /// The close call was made too early, before the end of the voting.
        TooEarly,
        /// There can only be a maximum of `MaxProposals` active proposals.
        TooManyProposals,
        /// The given weight bound for the proposal was too low.
        WrongProposalWeight,
        /// The given length bound for the proposal was too low.
        WrongProposalLength,
        /// One of the given members doesn't not have a valid registered DID
        MemberDIDNotRegistered
    }
}

/// Return the weight of a dispatch call result as an `Option`.
///
/// Will return the weight regardless of what the state of the result is.
fn get_result_weight(result: DispatchResultWithPostInfo) -> Option<Weight> {
    match result {
        Ok(post_info) => post_info.actual_weight,
        Err(err) => err.post_info.actual_weight,
    }
}

// Note that councillor operations are assigned to the operational class.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Set the collective's membership.
        ///
        /// - `new_members`: The new member list. Be nice to the chain and provide it sorted.
        /// - `prime`: The prime member whose vote sets the default.
        /// - `old_count`: The upper bound for the previous number of members in storage.
        ///                Used for weight estimation.
        ///
        /// Requires root origin.
        ///
        /// NOTE: Does not enforce the expected `MaxMembers` limit on the amount of members, but
        ///       the weight estimations rely on it to estimate dispatchable weight.
        ///
        /// # <weight>
        /// ## Weight
        /// - `O(MP + N)` where:
        ///   - `M` old-members-count (code- and governance-bounded)
        ///   - `N` new-members-count (code- and governance-bounded)
        ///   - `P` proposals-count (code-bounded)
        /// - DB:
        ///   - 1 storage mutation (codec `O(M)` read, `O(N)` write) for reading and writing the members
        ///   - 1 storage read (codec `O(P)`) for reading the proposals
        ///   - `P` storage mutations (codec `O(M)`) for updating the votes for each proposal
        ///   - 1 storage write (codec `O(1)`) for deleting the old `prime` and setting the new one
        /// # </weight>
        #[weight = 1]
        fn set_members(origin,
            new_members: Vec<Did>,
            prime: Option<Did>,
            old_count: MemberCount,
        ) -> DispatchResult {
            ensure_root(origin)?;
            if new_members.len() > T::MaxMembers::get() as usize {
                debug::error!(
                    "New members count exceeds maximum amount of members expected. (expected: {}, actual: {})",
                    T::MaxMembers::get(),
                    new_members.len()
                );
            }

            let old = Members::get();
            if old.len() > old_count as usize {
                debug::warn!(
                    "Wrong count used to estimate set_members weight. (expected: {}, actual: {})",
                    old_count,
                    old.len()
                );
            }
            for member in new_members.iter() {
                ensure!(did::Module::<T>::did_registered(member), Error::<T>::MemberDIDNotRegistered);
            }
            let mut new_members = new_members;
            new_members.sort();
            // <Self as ChangeMembers<T::AccountId>>::set_members_sorted(&new_members, &old);
            Members::put(new_members);
            // Prime::<T, I>::set(prime);
            Prime::set(prime);

            Ok(())
        }

        /// Dispatch a proposal from a member using the `Member` origin.
        ///
        /// Origin must be a member of the collective.
        ///
        /// # <weight>
        /// ## Weight
        /// - `O(M + P)` where `M` members-count (code-bounded) and `P` complexity of dispatching `proposal`
        /// - DB: 1 read (codec `O(M)`) + DB access of `proposal`
        /// - 1 event
        /// # </weight>
        #[weight = 1]
        fn execute(origin,
            proposal: Box<<T as Config>::Proposal>,
            #[compact] length_bound: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let who_did = did::Module::<T>::get_did_from_account_id(&who);
            let members = Self::members();
            ensure!(members.contains(&who_did), Error::<T>::NotMember);
            let proposal_len = proposal.using_encoded(|x| x.len());
            ensure!(proposal_len <= length_bound as usize, Error::<T>::WrongProposalLength);

            let proposal_hash = T::Hashing::hash_of(&proposal);
            let result = proposal.dispatch(RawOrigin::Member(who).into());
            Self::deposit_event(
                RawEvent::MemberExecuted(proposal_hash, result.map(|_| ()).map_err(|e| e.error))
            );

            Ok(())
        }

        /// Add a new proposal to either be voted on or executed directly.
        ///
        /// Requires the sender to be member.
        ///
        /// `threshold` determines whether `proposal` is executed directly (`threshold < 2`)
        /// or put up for voting.
        ///
        /// # <weight>
        /// ## Weight
        /// - `O(B + M + P1)` or `O(B + M + P2)` where:
        ///   - `B` is `proposal` size in bytes (length-fee-bounded)
        ///   - `M` is members-count (code- and governance-bounded)
        ///   - branching is influenced by `threshold` where:
        ///     - `P1` is proposal execution complexity (`threshold < 2`)
        ///     - `P2` is proposals-count (code-bounded) (`threshold >= 2`)
        /// - DB:
        ///   - 1 storage read `is_member` (codec `O(M)`)
        ///   - 1 storage read `ProposalOf::contains_key` (codec `O(1)`)
        ///   - DB accesses influenced by `threshold`:
        ///     - EITHER storage accesses done by `proposal` (`threshold < 2`)
        ///     - OR proposal insertion (`threshold <= 2`)
        ///       - 1 storage mutation `Proposals` (codec `O(P2)`)
        ///       - 1 storage mutation `ProposalCount` (codec `O(1)`)
        ///       - 1 storage write `ProposalOf` (codec `O(B)`)
        ///       - 1 storage write `Voting` (codec `O(M)`)
        ///   - 1 event
        /// # </weight>
        #[weight = 1]
        fn propose(origin,
            #[compact] threshold: MemberCount,
            proposal: Box<T::Proposal>,
            #[compact] length_bound: u32
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let who_did = did::Module::<T>::get_did_from_account_id(&who);
            let members = Self::members();
            ensure!(members.contains(&who_did), Error::<T>::NotMember);

            let proposal_len = proposal.using_encoded(|x| x.len());
            ensure!(proposal_len <= length_bound as usize, Error::<T>::WrongProposalLength);
            let proposal_hash = T::Hashing::hash_of(&proposal);
            ensure!(!<ProposalOf<T>>::contains_key(proposal_hash), Error::<T>::DuplicateProposal);

            if threshold < 2 {
                let seats = Self::members().len() as MemberCount;
                let result = proposal.dispatch(RawOrigin::Members(1, seats).into());
                Self::deposit_event(
                    RawEvent::Executed(proposal_hash, result.map(|_| ()).map_err(|e| e.error))
                );

                Ok(())
            } else {
                let active_proposals =
                    <Proposals<T>>::try_mutate(|proposals| -> Result<usize, DispatchError> {
                        proposals.push(proposal_hash);
                        ensure!(
                            proposals.len() <= T::MaxProposals::get() as usize,
                            Error::<T>::TooManyProposals
                        );
                        Ok(proposals.len())
                    })?;
                let index = Self::proposal_count();
                <ProposalCount>::mutate(|i| *i += 1);
                <ProposalOf<T>>::insert(proposal_hash, *proposal);
                let end = system::Module::<T>::block_number() + T::MotionDuration::get();
                let votes = Votes { index, threshold, ayes: vec![who_did], nays: vec![], end };
                <Voting<T>>::insert(proposal_hash, votes);

                Self::deposit_event(RawEvent::Proposed(who_did, index, proposal_hash, threshold));

                Ok(())
            }
        }

        /// Add an aye or nay vote for the sender to the given proposal.
        ///
        /// Requires the sender to be a member.
        ///
        /// # <weight>
        /// ## Weight
        /// - `O(M)` where `M` is members-count (code- and governance-bounded)
        /// - DB:
        ///   - 1 storage read `Members` (codec `O(M)`)
        ///   - 1 storage mutation `Voting` (codec `O(M)`)
        /// - 1 event
        /// # </weight>
        #[weight = 1]
        fn vote(origin,
            proposal: T::Hash,
            #[compact] index: ProposalIndex,
            approve: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let who_did = did::Module::<T>::get_did_from_account_id(&who);
            let members = Self::members();
            ensure!(members.contains(&who_did), Error::<T>::NotMember);

            let mut voting = Self::voting(&proposal).ok_or(Error::<T>::ProposalMissing)?;
            ensure!(voting.index == index, Error::<T>::WrongIndex);

            let position_yes = voting.ayes.iter().position(|a| a == &who_did);
            let position_no = voting.nays.iter().position(|a| a == &who_did);

            if approve {
                if position_yes.is_none() {
                    voting.ayes.push(who_did);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_no {
                    voting.nays.swap_remove(pos);
                }
            } else {
                if position_no.is_none() {
                    voting.nays.push(who_did);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_yes {
                    voting.ayes.swap_remove(pos);
                }
            }

            let yes_votes = voting.ayes.len() as MemberCount;
            let no_votes = voting.nays.len() as MemberCount;

            Self::deposit_event(RawEvent::Voted(who_did, proposal, approve, yes_votes, no_votes));

            Voting::<T>::insert(&proposal, voting);

            Ok(())
        }

        /// Close a vote that is either approved, disapproved or whose voting period has ended.
        ///
        /// May be called by any signed account in order to finish voting and close the proposal.
        ///
        /// If called before the end of the voting period it will only close the vote if it is
        /// has enough votes to be approved or disapproved.
        ///
        /// If called after the end of the voting period abstentions are counted as rejections
        /// unless there is a prime member set and the prime member cast an approval.
        ///
        /// + `proposal_weight_bound`: The maximum amount of weight consumed by executing the closed proposal.
        /// + `length_bound`: The upper bound for the length of the proposal in storage. Checked via
        ///                   `storage::read` so it is `size_of::<u32>() == 4` larger than the pure length.
        ///
        /// # <weight>
        /// ## Weight
        /// - `O(B + M + P1 + P2)` where:
        ///   - `B` is `proposal` size in bytes (length-fee-bounded)
        ///   - `M` is members-count (code- and governance-bounded)
        ///   - `P1` is the complexity of `proposal` preimage.
        ///   - `P2` is proposal-count (code-bounded)
        /// - DB:
        ///  - 2 storage reads (`Members`: codec `O(M)`, `Prime`: codec `O(1)`)
        ///  - 3 mutations (`Voting`: codec `O(M)`, `ProposalOf`: codec `O(B)`, `Proposals`: codec `O(P2)`)
        ///  - any mutations done while executing `proposal` (`P1`)
        /// - up to 3 events
        /// # </weight>
        #[weight = 1]
        fn close(origin,
            proposal_hash: T::Hash,
            #[compact] index: ProposalIndex,
            #[compact] proposal_weight_bound: Weight,
            #[compact] length_bound: u32
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let voting = Self::voting(&proposal_hash).ok_or(Error::<T>::ProposalMissing)?;
            ensure!(voting.index == index, Error::<T>::WrongIndex);

            let mut no_votes = voting.nays.len() as MemberCount;
            let mut yes_votes = voting.ayes.len() as MemberCount;
            let seats = Self::members().len() as MemberCount;
            let approved = yes_votes >= voting.threshold;
            let disapproved = seats.saturating_sub(no_votes) < voting.threshold;
            // Allow (dis-)approving the proposal as soon as there are enough votes.
            if approved {
                let (proposal, len) = Self::validate_and_get_proposal(
                    &proposal_hash,
                    length_bound,
                    proposal_weight_bound
                )?;
                Self::deposit_event(RawEvent::Closed(proposal_hash, yes_votes, no_votes));
                let (proposal_weight, proposal_count) =
                    Self::do_approve_proposal(seats, voting, proposal_hash, proposal);
                return Ok(());
            } else if disapproved {
                Self::deposit_event(RawEvent::Closed(proposal_hash, yes_votes, no_votes));
                let proposal_count = Self::do_disapprove_proposal(proposal_hash);
                return Ok(());
            }

            // Only allow actual closing of the proposal after the voting period has ended.
            ensure!(system::Module::<T>::block_number() >= voting.end, Error::<T>::TooEarly);

            let prime_vote = Self::prime().map(|who| voting.ayes.iter().any(|a| a == &who));

            // default voting strategy.
            let default = T::DefaultVote::default_vote(prime_vote, yes_votes, no_votes, seats);

            let abstentions = seats - (yes_votes + no_votes);
            match default {
                true => yes_votes += abstentions,
                false => no_votes += abstentions,
            }
            let approved = yes_votes >= voting.threshold;

            if approved {
                let (proposal, len) = Self::validate_and_get_proposal(
                    &proposal_hash,
                    length_bound,
                    proposal_weight_bound
                )?;
                Self::deposit_event(RawEvent::Closed(proposal_hash, yes_votes, no_votes));
                let (proposal_weight, proposal_count) =
                    Self::do_approve_proposal(seats, voting, proposal_hash, proposal);
                return Ok(());
            } else {
                Self::deposit_event(RawEvent::Closed(proposal_hash, yes_votes, no_votes));
                let proposal_count = Self::do_disapprove_proposal(proposal_hash);
                return Ok(());
            }
        }

        /// Disapprove a proposal, close, and remove it from the system, regardless of its current state.
        ///
        /// Must be called by the Root origin.
        ///
        /// Parameters:
        /// * `proposal_hash`: The hash of the proposal that should be disapproved.
        ///
        /// # <weight>
        /// Complexity: O(P) where P is the number of max proposals
        /// DB Weight:
        /// * Reads: Proposals
        /// * Writes: Voting, Proposals, ProposalOf
        /// # </weight>
        #[weight = 1]
        fn disapprove_proposal(origin, proposal_hash: T::Hash) -> DispatchResult {
            ensure_root(origin)?;
            let proposal_count = Self::do_disapprove_proposal(proposal_hash);
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    /// Check whether `who` is a member of the collective.
    pub fn is_member(who: Did) -> bool {
        // Note: The dispatchables *do not* use this to check membership so make sure
        // to update those if this is changed.
        Self::members().contains(&who)
    }

    /// Ensure that the right proposal bounds were passed and get the proposal from storage.
    ///
    /// Checks the length in storage via `storage::read` which adds an extra `size_of::<u32>() == 4`
    /// to the length.
    fn validate_and_get_proposal(
        hash: &T::Hash,
        length_bound: u32,
        weight_bound: Weight,
    ) -> Result<(<T as Config>::Proposal, usize), DispatchError> {
        let key = ProposalOf::<T>::hashed_key_for(hash);
        // read the length of the proposal storage entry directly
        let proposal_len =
            storage::read(&key, &mut [0; 0], 0).ok_or(Error::<T>::ProposalMissing)?;
        ensure!(
            proposal_len <= length_bound,
            Error::<T>::WrongProposalLength
        );
        let proposal = ProposalOf::<T>::get(hash).ok_or(Error::<T>::ProposalMissing)?;
        let proposal_weight = proposal.get_dispatch_info().weight;
        ensure!(
            proposal_weight <= weight_bound,
            Error::<T>::WrongProposalWeight
        );
        Ok((proposal, proposal_len as usize))
    }

    /// Weight:
    /// If `approved`:
    /// - the weight of `proposal` preimage.
    /// - two events deposited.
    /// - two removals, one mutation.
    /// - computation and i/o `O(P + L)` where:
    ///   - `P` is number of active proposals,
    ///   - `L` is the encoded length of `proposal` preimage.
    ///
    /// If not `approved`:
    /// - one event deposited.
    /// Two removals, one mutation.
    /// Computation and i/o `O(P)` where:
    /// - `P` is number of active proposals
    fn do_approve_proposal(
        seats: MemberCount,
        voting: Votes<Did, T::BlockNumber>,
        proposal_hash: T::Hash,
        proposal: <T as Config>::Proposal,
    ) -> (Weight, u32) {
        Self::deposit_event(RawEvent::Approved(proposal_hash));

        let dispatch_weight = proposal.get_dispatch_info().weight;
        let origin = RawOrigin::Members(voting.threshold, seats).into();
        let result = proposal.dispatch(origin);
        Self::deposit_event(RawEvent::Executed(
            proposal_hash,
            result.map(|_| ()).map_err(|e| e.error),
        ));
        // default to the dispatch info weight for safety
        let proposal_weight = get_result_weight(result).unwrap_or(dispatch_weight); // P1

        let proposal_count = Self::remove_proposal(proposal_hash);
        (proposal_weight, proposal_count)
    }

    fn do_disapprove_proposal(proposal_hash: T::Hash) -> u32 {
        // disapproved
        Self::deposit_event(RawEvent::Disapproved(proposal_hash));
        Self::remove_proposal(proposal_hash)
    }

    // Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
    fn remove_proposal(proposal_hash: T::Hash) -> u32 {
        // remove proposal and vote
        ProposalOf::<T>::remove(&proposal_hash);
        Voting::<T>::remove(&proposal_hash);
        let num_proposals = Proposals::<T>::mutate(|proposals| {
            proposals.retain(|h| h != &proposal_hash);
            proposals.len() + 1 // calculate weight based on original length
        });
        num_proposals as u32
    }
}

// impl<T: Trait<I>, I: Instance> ChangeMembers<T::AccountId> for Module<T, I> {
// 	/// Update the members of the collective. Votes are updated and the prime is reset.
// 	///
// 	/// NOTE: Does not enforce the expected `MaxMembers` limit on the amount of members, but
// 	///       the weight estimations rely on it to estimate dispatchable weight.
// 	///
// 	/// # <weight>
// 	/// ## Weight
// 	/// - `O(MP + N)`
// 	///   - where `M` old-members-count (governance-bounded)
// 	///   - where `N` new-members-count (governance-bounded)
// 	///   - where `P` proposals-count
// 	/// - DB:
// 	///   - 1 storage read (codec `O(P)`) for reading the proposals
// 	///   - `P` storage mutations for updating the votes (codec `O(M)`)
// 	///   - 1 storage write (codec `O(N)`) for storing the new members
// 	///   - 1 storage write (codec `O(1)`) for deleting the old prime
// 	/// # </weight>
// 	fn change_members_sorted(
// 		_incoming: &[T::AccountId],
// 		outgoing: &[T::AccountId],
// 		new: &[T::AccountId],
// 	) {
// 		if new.len() > T::MaxMembers::get() as usize {
// 			debug::error!(
// 				"New members count exceeds maximum amount of members expected. (expected: {}, actual: {})",
// 				T::MaxMembers::get(),
// 				new.len()
// 			);
// 		}
// 		// remove accounts from all current voting in motions.
// 		let mut outgoing = outgoing.to_vec();
// 		outgoing.sort();
// 		for h in Self::proposals().into_iter() {
// 			<Voting<T, I>>::mutate(h, |v|
// 				if let Some(mut votes) = v.take() {
// 					votes.ayes = votes.ayes.into_iter()
// 						.filter(|i| outgoing.binary_search(i).is_err())
// 						.collect();
// 					votes.nays = votes.nays.into_iter()
// 						.filter(|i| outgoing.binary_search(i).is_err())
// 						.collect();
// 					*v = Some(votes);
// 				}
// 			);
// 		}
// 		Members::<T, I>::put(new);
// 		Prime::<T, I>::kill();
// 	}

// 	fn set_prime(prime: Option<T::AccountId>) {
// 		Prime::<T, I>::set(prime);
// 	}
// }

impl<T: Config> InitializeMembers<Did> for Module<T> {
    fn initialize_members(members: &[Did]) {
        if !members.is_empty() {
            assert!(
                <Members>::get().is_empty(),
                "Members are already initialized!"
            );
            <Members>::put(members);
        }
    }
}

/// Ensure that the origin `o` represents at least `n` members. Returns `Ok` or an `Err`
/// otherwise.
pub fn ensure_members<OuterOrigin, AccountId>(
    o: OuterOrigin,
    n: MemberCount,
) -> result::Result<MemberCount, &'static str>
where
    OuterOrigin: Into<result::Result<RawOrigin<AccountId>, OuterOrigin>>,
{
    match o.into() {
        Ok(RawOrigin::Members(x, _)) if x >= n => Ok(n),
        _ => Err("bad origin: expected to be a threshold number of members"),
    }
}

pub struct EnsureMember<AccountId>(sp_std::marker::PhantomData<AccountId>);
impl<O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>, AccountId: Default>
    EnsureOrigin<O> for EnsureMember<AccountId>
{
    type Success = AccountId;
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::Member(id) => Ok(id),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::Members(1u32, 0u32))
    }
}

pub struct EnsureMembers<N: U32, AccountId>(sp_std::marker::PhantomData<(N, AccountId)>);
impl<O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>, N: U32, AccountId>
    EnsureOrigin<O> for EnsureMembers<N, AccountId>
{
    type Success = (MemberCount, MemberCount);
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::Members(n, m) if n >= N::VALUE => Ok((n, m)),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::Members(1u32, 0u32))
    }
}

pub struct EnsureProportionMoreThan<N: U32, D: U32, AccountId>(
    sp_std::marker::PhantomData<(N, D, AccountId)>,
);
impl<
        O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>,
        N: U32,
        D: U32,
        AccountId,
    > EnsureOrigin<O> for EnsureProportionMoreThan<N, D, AccountId>
{
    type Success = ();
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::Members(n, m) if n * D::VALUE > N::VALUE * m => Ok(()),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::Members(1u32, 0u32))
    }
}

pub struct EnsureProportionAtLeast<N: U32, D: U32, AccountId>(
    sp_std::marker::PhantomData<(N, D, AccountId)>,
);
impl<
        O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>,
        N: U32,
        D: U32,
        AccountId,
    > EnsureOrigin<O> for EnsureProportionAtLeast<N, D, AccountId>
{
    type Success = ();
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::Members(n, m) if n * D::VALUE >= N::VALUE * m => Ok(()),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::Members(0u32, 0u32))
    }
}
