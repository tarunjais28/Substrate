// Tests for Schema module
use super::*;
use did;
use frame_support::{assert_noop, assert_ok, parameter_types};
use sp_core::{sr25519, H256};
use sp_runtime::{testing::Header, traits::BlakeTwo256};
use std::str::FromStr;

use crate as schema;
use crate::Config;
use validator_set;

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
        Schema: schema::{Module, Call, Storage, Event<T>},
        Did: did::{Module, Call, Storage, Event, Config},
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

impl Config for Test {
    type Event = Event;
}

impl did::Config for Test {
    type Event = Event;
}

impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

type DidStruct = did::DidStruct;

const VALIDATOR_ACCOUNT: u64 = 0;
const NON_VALIDATOR_ACCOUNT: u64 = 2;
const VALIDATOR_DID: [u8; 32] = *b"Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const VALIDATOR_PUBKEY: sr25519::Public = sr25519::Public([0; 32]);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
fn new_test_ext() -> sp_io::TestExternalities {
    let mut o = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    did::GenesisConfig {
        dids: vec![DidStruct {
            identifier: VALIDATOR_DID,
            public_key: VALIDATOR_PUBKEY,
            metadata: vec![],
        }],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    validator_set::GenesisConfig {
        validators: vec![VALIDATOR_DID],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    o.into()
}

#[test]
fn genesis_setup_works() {
    new_test_ext().execute_with(|| {
        let (did_doc, block_number) = Did::get_did_details(VALIDATOR_DID.clone()).unwrap();
        assert_eq!(did_doc.identifier, VALIDATOR_DID);
        assert_eq!(did_doc.public_key, VALIDATOR_PUBKEY);
        assert_eq!(block_number, 0);
    })
}

#[test]
#[should_panic(expected = "NotAValidator")]
fn non_validator_should_not_add_schema() {
    new_test_ext().execute_with(|| {
        assert_ok!(Schema::add(
            Origin::signed(NON_VALIDATOR_ACCOUNT),
            H256::from_str("D04B98F48E8F8BCC15C6AE5AC050801CD6DCFD428FB5F9E65C4E16E7807340FA")
                .unwrap(),
            vec![]
        ));
    })
}

#[test]
fn test_add_new_schema() {
    new_test_ext().execute_with(|| {
        let schema_hash =
            H256::from_str("D04B98F48E8F8BCC15C6AE5AC050801CD6DCFD428FB5F9E65C4E16E7807340FA")
                .unwrap();
        assert_ok!(Schema::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            schema_hash,
            "TestSchema".as_bytes().to_vec()
        ));

        // ensure the stored data matches
        let (origin_acc, schema_data) = SCHEMA::<Test>::get(schema_hash).unwrap();
        assert_eq!(origin_acc, VALIDATOR_DID);
        assert_eq!(schema_data, "TestSchema".as_bytes().to_vec());
    })
}

#[test]
#[should_panic(expected = "SchemaAlreadyExists")]
fn test_repeat_same_schema() {
    new_test_ext().execute_with(|| {
        let schema_hash =
            H256::from_str("D04B98F48E8F8BCC15C6AE5AC050801CD6DCFD428FB5F9E65C4E16E7807340FA")
                .unwrap();

        assert_ok!(Schema::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            schema_hash,
            "TestSchema".as_bytes().to_vec()
        ));

        assert_ok!(Schema::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            schema_hash,
            "TestSchema".as_bytes().to_vec()
        ));
    })
}
