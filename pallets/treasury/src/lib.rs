#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use did::Did;
use frame_support::traits::EnsureOrigin;
use frame_support::traits::{Currency, Get, Imbalance, OnUnbalanced, ReservableCurrency};
use frame_support::weights::Weight;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::ensure_signed;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, StaticLookup, Zero},
    ModuleId, Permill, RuntimeDebug,
};
use sp_std::prelude::*;

type BalanceOf<T, I> =
    <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T, I> = <<T as Config<I>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::PositiveImbalance;
type NegativeImbalanceOf<T, I> = <<T as Config<I>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub trait Config<I = DefaultInstance>: frame_system::Config + did::Config {
    /// The treasury's module id, used for deriving its sovereign account ID.
    type ModuleId: Get<ModuleId>;

    /// The staking balance.
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// Origin from which approvals must come.
    type ApproveOrigin: EnsureOrigin<Self::Origin>;

    /// Origin from which rejections must come.
    type RejectOrigin: EnsureOrigin<Self::Origin>;

    /// The overarching event type.
    type Event: From<Event<Self, I>> + Into<<Self as frame_system::Config>::Event>;

    /// Handler for the unbalanced decrease when slashing for a rejected proposal or bounty.
    type OnSlash: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

    /// Fraction of a proposal's value that should be bonded in order to place the proposal.
    /// An accepted proposal gets these back. A rejected proposal does not.
    type ProposalBond: Get<Permill>;

    /// Minimum amount of funds that should be placed in a deposit for making a proposal.
    type ProposalBondMinimum: Get<BalanceOf<Self, I>>;

    /// Period between successive spends.
    type SpendPeriod: Get<Self::BlockNumber>;

    /// Percentage of spare funds (if any) that are burnt per spend period.
    type Burn: Get<Permill>;

    /// Maximum acceptable reason length.
    type MaximumReasonLength: Get<u32>;

    /// Handler for the unbalanced decrease when treasury funds are burned.
    type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

    // // Weight information for extrinsics in this pallet.
    // type WeightInfo: ();
}

/// An index of a proposal. Just a `u32`.
pub type ProposalIndex = u32;

/// A spending proposal.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Proposal<Did, Balance> {
    // do not change the order of the variables, leads to memory leak due to a substrate issue
    /// The account proposing it.
    proposer: Did,
    /// The account to whom the payment should be made if the proposal is accepted.
    beneficiary: Did,
    /// The (total) amount that should be paid if the proposal is accepted.
    value: Balance,
    /// The amount held on deposit (reserved) for making this proposal.
    bond: Balance,
}

//// An open tipping "motion". Retains all details of a tip including information on the finder
//// and the members who have voted.
// #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
// pub struct OpenTip<
// 	AccountId: Parameter,
// 	Balance: Parameter,
// 	BlockNumber: Parameter,
// 	Hash: Parameter,
// > {
// 	/// The hash of the reason for the tip. The reason should be a human-readable UTF-8 encoded string. A URL would be
// 	/// sensible.
// 	reason: Hash,
// 	/// The account to be tipped.
// 	who: AccountId,
// 	/// The account who began this tip.
// 	finder: AccountId,
// 	/// The amount held on deposit for this tip.
// 	deposit: Balance,
// 	/// The block number at which this tip will close if `Some`. If `None`, then no closing is
// 	/// scheduled.
// 	closes: Option<BlockNumber>,
// 	/// The members who have voted for this tip. Sorted by AccountId.
// 	tips: Vec<(AccountId, Balance)>,
// 	/// Whether this tip should result in the finder taking a fee.
// 	finders_fee: bool,
// }

decl_storage! {
    trait Store for Module<T: Config<I>, I: Instance=DefaultInstance> as Treasury {
        /// Number of proposals that have been made.
        ProposalCount get(fn proposal_count): ProposalIndex;

        /// Proposals that have been made.
        Proposals get(fn proposals):
            map hasher(blake2_128_concat) ProposalIndex
            => Option<Proposal<Did, BalanceOf<T, I>>>;

        /// Proposal indices that have been approved but not yet awarded.
        Approvals get(fn approvals): Vec<ProposalIndex>;

        /// Simple preimage lookup from the reason's hash to the original data. Again, has an
        /// insecure enumerable hash since the key is guaranteed to be the result of a secure hash.
        pub Reasons get(fn reasons): map hasher(identity) T::Hash => Option<Vec<u8>>;

    }
    add_extra_genesis {
        build(|_config| {
            // Create Treasury account
            let account_id = <Module<T, I>>::account_id();
            let min = T::Currency::minimum_balance();
            if T::Currency::free_balance(&account_id) < min {
                let _ = T::Currency::make_free_balance_be(
                    &account_id,
                    min,
                );
            }
        });
    }
}

decl_event!(
    pub enum Event<T, I=DefaultInstance>
    where
        Balance = BalanceOf<T, I>,
        Did = Did,
        <T as frame_system::Config>::Hash,
    {
        /// New proposal. \[proposal_index\]
        Proposed(ProposalIndex),
        /// We have ended a spend period and will now allocate funds. \[budget_remaining\]
        Spending(Balance),
        /// Some funds have been allocated. \[proposal_index, award, beneficiary\]
        Awarded(ProposalIndex, Balance, Did),
        /// A proposal was rejected; funds were slashed. \[proposal_index, slashed\]
        Rejected(ProposalIndex, Balance),
        /// Some of our funds have been burnt. \[burn\]
        Burnt(Balance),
        /// Spending has finished; this is the amount that rolls over until next spend.
        /// \[budget_remaining\]
        Rollover(Balance),
        /// Some funds have been deposited. \[deposit\]
        Deposit(Balance),
        /// A new tip suggestion has been opened. \[tip_hash\]
        NewTip(Hash),
    }
);

decl_error! {
    /// Error for the treasury module.
    pub enum Error for Module<T: Config<I>, I: Instance> {
        /// Proposer's balance is too low.
        InsufficientProposersBalance,
        /// No proposal or bounty at that index.
        InvalidIndex,
        /// The reason given is just too big.
        ReasonTooBig,
        /// The tip was already found/started.
        AlreadyKnown,
        /// The tip hash is unknown.
        UnknownTip,
        /// The account attempting to retract the tip is not the finder of the tip.
        NotFinder,
        /// The tip cannot be claimed/closed because there are not enough tippers yet.
        StillOpen,
        /// The tip cannot be claimed/closed because it's still in the countdown period.
        Premature,
        /// The bounty status is unexpected.
        UnexpectedStatus,
        /// Require bounty curator.
        RequireCurator,
        /// Invalid bounty value.
        InvalidValue,
        /// Invalid bounty fee.
        InvalidFee,
        /// A bounty payout is pending.
        /// To cancel the bounty, you must unassign and slash the curator.
        PendingPayout,
    }
}

decl_module! {
    pub struct Module<T: Config<I>, I: Instance=DefaultInstance>
        for enum Call
        where origin: T::Origin
    {
        /// Fraction of a proposal's value that should be bonded in order to place the proposal.
        /// An accepted proposal gets these back. A rejected proposal does not.
        const ProposalBond: Permill = T::ProposalBond::get();

        /// Minimum amount of funds that should be placed in a deposit for making a proposal.
        const ProposalBondMinimum: BalanceOf<T, I> = T::ProposalBondMinimum::get();

        /// Period between successive spends.
        const SpendPeriod: T::BlockNumber = T::SpendPeriod::get();

        /// Percentage of spare funds (if any) that are burnt per spend period.
        const Burn: Permill = T::Burn::get();

        /// The period for which a tip remains open after is has achieved threshold tippers.
        // const TipCountdown: T::BlockNumber = T::TipCountdown::get();

        /// The amount of the final tip which goes to the original reporter of the tip.
        // const TipFindersFee: Percent = T::TipFindersFee::get();

        /// The amount held on deposit for placing a tip report.
        // const TipReportDepositBase: BalanceOf<T, I> = T::TipReportDepositBase::get();

        /// The amount held on deposit per byte within the tip report reason or bounty description.
        // const DataDepositPerByte: BalanceOf<T, I> = T::DataDepositPerByte::get();

        /// The treasury's module id, used for deriving its sovereign account ID.
        const ModuleId: ModuleId = T::ModuleId::get();

        /// The amount held on deposit for placing a bounty proposal.
        // const BountyDepositBase: BalanceOf<T, I> = T::BountyDepositBase::get();

        /// The delay period for which a bounty beneficiary need to wait before claim the payout.
        // const BountyDepositPayoutDelay: T::BlockNumber = T::BountyDepositPayoutDelay::get();

        /// Percentage of the curator fee that will be reserved upfront as deposit for bounty curator.
        // const BountyCuratorDeposit: Permill = T::BountyCuratorDeposit::get();

        // const BountyValueMinimum: BalanceOf<T, I> = T::BountyValueMinimum::get();

        /// Maximum acceptable reason length.
        const MaximumReasonLength: u32 = T::MaximumReasonLength::get();

        type Error = Error<T, I>;

        fn deposit_event() = default;

        /// Put forward a suggestion for spending. A deposit proportional to the value
        /// is reserved and slashed if the proposal is rejected. It is returned once the
        /// proposal is awarded.
        ///
        /// # <weight>
        /// - Complexity: O(1)
        /// - DbReads: `ProposalCount`, `origin account`
        /// - DbWrites: `ProposalCount`, `Proposals`, `origin account`
        /// # </weight>
        #[weight = 1]
        fn propose_spend(
            origin,
            #[compact] value: BalanceOf<T, I>,
            beneficiary: <T::Lookup as StaticLookup>::Source
        ) {
            let proposer = ensure_signed(origin)?;
            let beneficiary = T::Lookup::lookup(beneficiary)?;

            let bond = Self::calculate_bond(value);
            T::Currency::reserve(&proposer, bond)
                .map_err(|_| Error::<T, I>::InsufficientProposersBalance)?;

            let c = Self::proposal_count();
            <ProposalCount<I>>::put(c + 1);

            // convert to DID before inserting to storage
            ensure!(did::Module::<T>::does_did_exist(&beneficiary), did::Error::<T>::DIDDoesNotExist);
            let proposer_did = did::Module::<T>::get_did_from_account_id(&proposer);
            let beneficiary_did = did::Module::<T>::get_did_from_account_id(&beneficiary);

            // debug::info!("Proposer DID : {:?}", HexDisplay::from(&proposer_did));
            // debug::info!("B DID : {:?}", HexDisplay::from(&beneficiary_did));

            <Proposals<T, I>>::insert(c, Proposal {
                proposer : proposer_did,
                value : value,
                beneficiary : beneficiary_did,
                bond : bond
            });

            Self::deposit_event(RawEvent::Proposed(c));
        }

        /// Reject a proposed spend. The original deposit will be slashed.
        ///
        /// May only be called from `T::RejectOrigin`.
        ///
        /// # <weight>
        /// - Complexity: O(1)
        /// - DbReads: `Proposals`, `rejected proposer account`
        /// - DbWrites: `Proposals`, `rejected proposer account`
        /// # </weight>
        #[weight = 1]
        fn reject_proposal(origin, #[compact] proposal_id: ProposalIndex) {
            T::RejectOrigin::ensure_origin(origin)?;

            let proposal = <Proposals<T, I>>::take(&proposal_id).ok_or(Error::<T, I>::InvalidIndex)?;
            let value = proposal.bond;

            // convert to accountId from DID
            let proposer_account_id : T::AccountId = did::Module::<T>::get_accountid_from_did(&proposal.proposer).unwrap();

            let imbalance = T::Currency::slash_reserved(&proposer_account_id, value).0;
            T::OnSlash::on_unbalanced(imbalance);

            Self::deposit_event(Event::<T, I>::Rejected(proposal_id, value));
        }

        /// Approve a proposal. At a later time, the proposal will be allocated to the beneficiary
        /// and the original deposit will be returned.
        ///
        /// May only be called from `T::ApproveOrigin`.
        ///
        /// # <weight>
        /// - Complexity: O(1).
        /// - DbReads: `Proposals`, `Approvals`
        /// - DbWrite: `Approvals`
        /// # </weight>
        #[weight = 1]
        fn approve_proposal(origin, #[compact] proposal_id: ProposalIndex) {
            <T as Config<I>>::ApproveOrigin::ensure_origin(origin)?;

            ensure!(<Proposals<T, I>>::contains_key(proposal_id), Error::<T, I>::InvalidIndex);
            Approvals::<I>::append(proposal_id);
        }

        /// # <weight>
        /// - Complexity: `O(A)` where `A` is the number of approvals
        /// - Db reads and writes: `Approvals`, `pot account data`
        /// - Db reads and writes per approval:
        ///   `Proposals`, `proposer account data`, `beneficiary account data`
        /// - The weight is overestimated if some approvals got missed.
        /// # </weight>
        fn on_initialize(n: T::BlockNumber) -> Weight {
            // Check to see if we should spend some funds!
            if (n % T::SpendPeriod::get()).is_zero() {
                Self::spend_funds()
            } else {
                0
            }
        }
    }
}

impl<T: Config<I>, I: Instance> Module<T, I> {
    // Add public immutables and private mutables.

    /// The account ID of the treasury pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// The needed bond for a proposal whose spend is `value`.
    fn calculate_bond(value: BalanceOf<T, I>) -> BalanceOf<T, I> {
        T::ProposalBondMinimum::get().max(T::ProposalBond::get() * value)
    }

    /// Spend some money! returns number of approvals before spend.
    fn spend_funds() -> Weight {
        let mut total_weight: Weight = Zero::zero();

        let mut budget_remaining = Self::pot();
        Self::deposit_event(RawEvent::Spending(budget_remaining));
        let account_id = Self::account_id();

        let mut missed_any = false;
        let mut imbalance = <PositiveImbalanceOf<T, I>>::zero();
        let proposals_len = Approvals::<I>::mutate(|v| {
            let proposals_approvals_len = v.len() as u32;
            v.retain(|&index| {
                // Should always be true, but shouldn't panic if false or we're screwed.
                if let Some(p) = Self::proposals(index) {
                    if p.value <= budget_remaining {
                        budget_remaining -= p.value;
                        <Proposals<T, I>>::remove(index);

                        // convert to accountId from DID
                        let proposer_account_id: T::AccountId =
                            did::Module::<T>::get_accountid_from_did(&p.proposer).unwrap();

                        // return their deposit.
                        let _ = T::Currency::unreserve(&proposer_account_id, p.bond);

                        let beneficiary_account_id: T::AccountId =
                            did::Module::<T>::get_accountid_from_did(&p.beneficiary).unwrap();

                        // provide the allocation.
                        imbalance.subsume(T::Currency::deposit_creating(
                            &beneficiary_account_id,
                            p.value,
                        ));

                        Self::deposit_event(RawEvent::Awarded(index, p.value, p.beneficiary));
                        false
                    } else {
                        missed_any = true;
                        true
                    }
                } else {
                    false
                }
            });
            proposals_approvals_len
        });

        total_weight += 1;

        if !missed_any {
            // burn some proportion of the remaining budget if we run a surplus.
            let burn = (T::Burn::get() * budget_remaining).min(budget_remaining);
            budget_remaining -= burn;

            let (debit, credit) = T::Currency::pair(burn);
            imbalance.subsume(debit);
            T::BurnDestination::on_unbalanced(credit);
            Self::deposit_event(RawEvent::Burnt(burn))
        }

        // Must never be an error, but better to be safe.
        // proof: budget_remaining is account free balance minus ED;
        // Thus we can't spend more than account free balance minus ED;
        // Thus account is kept alive; qed;
        // if let Err(problem) = T::Currency::settle(
        // 	&account_id,
        // 	imbalance,
        // 	WithdrawReasons::TRANSFER,
        // 	KeepAlive
        // ) {
        // 	print("Inconsistent state - couldn't settle imbalance for funds spent by treasury");
        // 	// Nothing else to do here.
        // 	drop(problem);
        // }

        Self::deposit_event(RawEvent::Rollover(budget_remaining));

        total_weight
    }

    /// Return the amount of money in the pot.
    // The existential deposit is not part of the pot so treasury account never gets deleted.
    fn pot() -> BalanceOf<T, I> {
        T::Currency::free_balance(&Self::account_id())
            // Must never be less than 0 but better be safe.
            .saturating_sub(T::Currency::minimum_balance())
    }
}

impl<T: Config<I>, I: Instance> OnUnbalanced<NegativeImbalanceOf<T, I>> for Module<T, I> {
    fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T, I>) {
        let numeric_amount = amount.peek();

        // Must resolve into existing but better to be safe.
        let _ = T::Currency::resolve_creating(&Self::account_id(), amount);

        Self::deposit_event(RawEvent::Deposit(numeric_amount));
    }
}
