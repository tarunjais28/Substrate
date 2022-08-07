use std::hash::Hash;

use crate as collective;
use crate::Config;
use did::{self, DidStruct};
use frame_support::{
    assert_noop, assert_ok, parameter_types,
    traits::{OnFinalize, OnInitialize},
    weights::Weight,
};
use sp_core::{sr25519, H256};
use sp_runtime::{testing::Header, traits::BlakeTwo256};
use sudo;
pub use validator_set;

// Tests for Schema module
use super::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type BlockNumber = u32;

const MILLISECS_PER_BLOCK: u64 = 5000;
const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
const VALIDATOR_DID: [u8; 32] = *b"Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const IDENTIFIER1: [u8; 32] = *b"Dave\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const IDENTIFIER2: [u8; 32] = *b"Eve\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const VALIDATOR_ACCOUNT: u64 = 0;
const NON_VALIDATOR_ACCOUNT: u64 = 2;
const VALIDATOR_PUBKEY: sr25519::Public = sr25519::Public([0; 32]);
const IDENTIFIER1_PUBKEY: sr25519::Public = sr25519::Public([1; 32]);
const IDENTIFIER2_PUBKEY: sr25519::Public = sr25519::Public([2; 32]);

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Council: collective::{Module, Call, Storage, Origin<T>, Event<T>, Config},
        Did: did::{Module, Call, Storage, Event, Config},
        ValidatorSet: validator_set::{Module, Call, Storage, Event, Config},
        Sudo: sudo::{Module, Call, Config<T>, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const CouncilMotionDuration: BlockNumber = 5 * MINUTES;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = Did;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

impl sudo::Config for Test {
    type Event = Event;
    type Call = Call;
}

impl did::Config for Test {
    type Event = Event;
}

impl Config for Test {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = collective::PrimeDefaultVote;
}

impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
fn new_test_ext(root_key: u64) -> sp_io::TestExternalities {
    let mut o = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    super::GenesisConfig {
        members: vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    validator_set::GenesisConfig {
        validators: vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    did::GenesisConfig {
        dids: vec![
            DidStruct {
                identifier: VALIDATOR_DID,
                public_key: VALIDATOR_PUBKEY,
                metadata: vec![],
            },
            DidStruct {
                identifier: IDENTIFIER1,
                public_key: IDENTIFIER1_PUBKEY,
                metadata: vec![],
            },
            DidStruct {
                identifier: IDENTIFIER2,
                public_key: IDENTIFIER2_PUBKEY,
                metadata: vec![],
            },
        ],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    sudo::GenesisConfig::<Test> { key: root_key }
        .assimilate_storage(&mut o)
        .unwrap();

    o.into()
}

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
    }
}

#[test]
fn test_genesis_worked() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        assert_eq!(
            Members::get(),
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2]
        )
    })
}

#[test]
fn test_prime_default_vote() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Case when prime vote is yes
        assert!(super::PrimeDefaultVote::default_vote(Some(true), 3, 2, 100));

        // Case when prime vote is no
        assert!(!super::PrimeDefaultVote::default_vote(
            Some(false),
            3,
            2,
            100
        ));

        // Case when prime vote is null
        assert!(!super::PrimeDefaultVote::default_vote(None, 3, 2, 100));
    })
}

#[test]
fn test_more_than_majority_then_prime_default_vote() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Case when both priority and majority votes are yes
        assert!(super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            Some(true),
            3,
            2,
            5
        ));

        // Case when both priority and majority votes are no
        assert!(!super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            Some(false),
            2,
            3,
            5
        ));

        // Case when priority is yes and majority votes are no
        assert!(super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            Some(true),
            2,
            3,
            5
        ));

        // Case when priority is no and majority votes are yes
        assert!(super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            Some(false),
            3,
            2,
            5
        ));

        // Case when priority is null and majority votes are yes
        assert!(super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            None, 3, 2, 5
        ));

        // Case when priority is null and majority votes are no
        assert!(!super::MoreThanMajorityThenPrimeDefaultVote::default_vote(
            None, 2, 3, 5
        ));
    })
}

#[test]
fn test_set_members() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        let public_key = sr25519::Public([2; 32]);
        let metadata = "metadata".as_bytes().to_vec();

        // Check for non sudo account
        assert_noop!(
            Council::set_members(
                Origin::signed(VALIDATOR_ACCOUNT),
                vec![VALIDATOR_DID],
                Some(IDENTIFIER1),
                10
            ),
            DispatchError::BadOrigin
        );

        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(IDENTIFIER1),
            10,
        )));

        let users = Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call);

        // Checking for privileged users
        assert_ok!(users);

        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
                IDENTIFIER1
            ))),
            100
        ));

        // Checking with non sudo account = 1
        assert_noop!(
            Council::propose(
                Origin::signed(1),
                3,
                Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
                    IDENTIFIER1
                ))),
                100
            ),
            DispatchError::Module {
                index: 1,
                error: 0,
                message: Some("NotMember")
            }
        );
    })
}

#[test]
fn test_proposals() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));

        // Check for wrong proposal length
        assert_noop!(
            Council::propose(Origin::signed(VALIDATOR_ACCOUNT), 3, proposal.clone(), 10),
            DispatchError::Module {
                index: 1,
                error: 9,
                message: Some("WrongProposalLength")
            }
        );

        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal.clone(),
            100
        ));

        // Check for duplicate proposal
        assert_noop!(
            Council::propose(Origin::signed(VALIDATOR_ACCOUNT), 3, proposal, 100),
            DispatchError::Module {
                index: 1,
                error: 1,
                message: Some("DuplicateProposal")
            }
        );
    })
}

#[test]
fn test_vote() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));

        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];

        // Duplicate vote
        assert_noop!(
            Council::vote(Origin::signed(VALIDATOR_ACCOUNT), proposal_hash, 0, true),
            DispatchError::Module {
                index: 1,
                error: 4,
                message: Some("DuplicateVote")
            }
        );

        // Vote with unregistered member
        assert_noop!(
            Council::vote(Origin::signed(2), proposal_hash, 0, true),
            DispatchError::Module {
                index: 1,
                error: 0,
                message: Some("NotMember")
            }
        );

        // Case when proposal is missing
        assert_noop!(
            Council::vote(Origin::signed(VALIDATOR_ACCOUNT), H256::zero(), 1, true),
            DispatchError::Module {
                index: 1,
                error: 2,
                message: Some("ProposalMissing")
            }
        );
    })
}

#[test]
fn test_is_member() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        let (did_doc, _) = Did::get_did_details(IDENTIFIER1).unwrap();

        assert!(Module::<Test>::is_member(did_doc.identifier));
    })
}

#[test]
fn test_close() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));
        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];

        // Case when only one user voted
        assert_noop!(
            Council::close(
                Origin::signed(VALIDATOR_ACCOUNT),
                proposal_hash,
                0,
                100,
                100
            ),
            DispatchError::Module {
                index: 1,
                error: 6,
                message: Some("TooEarly")
            }
        );

        // Case when proposal is missing
        assert_noop!(
            Council::close(Origin::signed(VALIDATOR_ACCOUNT), H256::zero(), 1, 100, 100),
            DispatchError::Module {
                index: 1,
                error: 2,
                message: Some("ProposalMissing")
            }
        );

        // Case when wrong index is provided
        assert_noop!(
            Council::close(
                Origin::signed(VALIDATOR_ACCOUNT),
                proposal_hash,
                1,
                100,
                100
            ),
            DispatchError::Module {
                index: 1,
                error: 3,
                message: Some("WrongIndex")
            }
        );
    })
}

#[test]
fn test_validate_and_get_proposal() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Case when proposal is missing
        assert_noop!(
            Council::validate_and_get_proposal(&H256::zero(), 100, 100),
            DispatchError::Module {
                index: 1,
                error: 2,
                message: Some("ProposalMissing")
            }
        );

        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));
        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];

        assert_ok!(Council::validate_and_get_proposal(&proposal_hash, 100, 100));

        // Check for wrong proposal length
        assert_noop!(
            Council::validate_and_get_proposal(&proposal_hash, 10, 100),
            DispatchError::Module {
                index: 1,
                error: 9,
                message: Some("WrongProposalLength")
            }
        );
    })
}

#[test]
fn test_do_disapprove_proposal() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));
        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];

        assert_eq!(Council::do_disapprove_proposal(proposal_hash), 1);
    })
}

#[test]
fn test_remove_proposal() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));
        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];

        assert_eq!(Council::remove_proposal(proposal_hash), 1);
    })
}

#[test]
fn test_do_approve_proposal() {
    new_test_ext(VALIDATOR_ACCOUNT).execute_with(|| {
        // Giving sudo privilege
        let call = Box::new(Call::Council(collective::Call::<Test>::set_members(
            vec![VALIDATOR_DID, IDENTIFIER1, IDENTIFIER2],
            Some(VALIDATOR_DID),
            10,
        )));

        // Checking for privileged users
        assert_ok!(Sudo::sudo(Origin::signed(VALIDATOR_ACCOUNT), call));

        let proposal = Box::new(Call::ValidatorSet(validator_set::Call::<Test>::add_member(
            IDENTIFIER1,
        )));
        assert_ok!(Council::propose(
            Origin::signed(VALIDATOR_ACCOUNT),
            3,
            proposal,
            100
        ));

        let proposal_hash = Proposals::<Test>::get()[0];
        let votes = Voting::<Test>::get(proposal_hash).unwrap();
        let proposal_call =
            Call::ValidatorSet(validator_set::Call::<Test>::add_member(VALIDATOR_DID));

        assert_eq!(
            Council::do_approve_proposal(3, votes, proposal_hash, proposal_call),
            (1, 1)
        );
    })
}
