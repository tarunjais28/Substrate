use frame_support::{assert_noop, parameter_types};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BadOrigin, BlakeTwo256, IdentityLookup},
};

use crate as validator_set;
use crate::Config;

// Tests for Schema module
use super::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        ValidatorSet: validator_set::{Module, Call, Storage, Event, Config},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
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
    type Lookup = IdentityLookup<Self::AccountId>;
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

impl Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

const NON_VALIDATOR_ACCOUNT: u64 = 2;
const VALIDATOR_DID: [u8; 32] = *b"Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const DID_TEST: [u8; 32] = *b"Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
fn new_test_ext() -> sp_io::TestExternalities {
    let mut o = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    super::GenesisConfig {
        validators: vec![VALIDATOR_DID],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    o.into()
}

#[test]
fn genesis_setup_works() {
    new_test_ext().execute_with(|| {
        let did_list = Members::get();
        assert_eq!(did_list.len(), 1);
        assert_eq!(did_list[0], VALIDATOR_DID)
    })
}

#[test]
fn test_regular_call_not_permitted() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ValidatorSet::add_member(Origin::signed(NON_VALIDATOR_ACCOUNT), DID_TEST),
            BadOrigin
        );
    })
}
