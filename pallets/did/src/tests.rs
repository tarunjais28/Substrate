use super::*;
use crate as did;
use crate::Config;
use frame_support::{
    assert_ok, parameter_types,
    traits::{OnFinalize, OnInitialize},
};
use sp_core::{sr25519, H256};
use sp_runtime::{testing::Header, traits::BlakeTwo256};
use validator_set;

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

impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

const VALIDATOR_ACCOUNT: u64 = 0;
const NON_VALIDATOR_ACCOUNT: u64 = 2;
const VALIDATOR_DID: [u8; 32] = *b"did:ssid:Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const VALIDATOR_PUBKEY: sr25519::Public = sr25519::Public([0; 32]);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
fn new_test_ext() -> sp_io::TestExternalities {
    let mut o = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    super::GenesisConfig {
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
    new_test_ext().execute_with(|| {
        assert_eq!(DIDs::<Test>::contains_key(VALIDATOR_DID.clone()), true);
        assert_eq!(Lookup::<Test>::contains_key(VALIDATOR_DID.clone()), true);
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&VALIDATOR_PUBKEY)),
            true
        );

        let (did_doc, block_number) = Did::get_did_details(VALIDATOR_DID.clone()).unwrap();
        assert_eq!(did_doc.identifier, VALIDATOR_DID);
        assert_eq!(did_doc.public_key, VALIDATOR_PUBKEY);
        assert_eq!(block_number, 0);
    })
}

#[test]
#[should_panic]
fn non_validator_adds_did() {
    new_test_ext().execute_with(|| {
        let identifier = *b"Alice2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([0; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(NON_VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata
        ));
    })
}

#[test]
fn test_add_did() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Bob\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([2; 32]);
        let metadata = "metadata".as_bytes().to_vec();

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        assert_eq!(DIDs::<Test>::contains_key(identifier.clone()), true);
        assert_eq!(Lookup::<Test>::contains_key(identifier.clone()), true);
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key)),
            true
        );

        let (did_doc, _block_number) = Did::get_did_details(identifier.clone()).unwrap();
        assert_eq!(did_doc.identifier, identifier);
        assert_eq!(did_doc.public_key, public_key);
        assert_eq!(did_doc.metadata, metadata);

        let did_lookup = RLookup::<Test>::get(Did::get_accountid_from_pubkey(&public_key));
        assert_eq!(did_lookup, identifier.clone());
    })
}

#[test]
#[should_panic]
fn test_add_existing_did() {
    new_test_ext().execute_with(|| {
        // Adding the DID initialised at the time of genesis, so this test should fail
        let identifier = *b"did:ssid:Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([2; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));
    })
}

#[test]
#[should_panic]
fn test_add_existing_pubkey() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([3; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        let identifier = *b"did:ssid:Alicx2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([3; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));
    })
}

#[test]
fn test_remove_did() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([3; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        assert_ok!(Did::remove(
            Origin::signed(VALIDATOR_ACCOUNT),
            identifier.clone()
        ));

        assert_eq!(DIDs::<Test>::contains_key(identifier.clone()), false);
        assert_eq!(Lookup::<Test>::contains_key(identifier.clone()), false);
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key)),
            false
        );
    })
}

#[test]
fn test_rotate_key() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([3; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        let public_key2 = sr25519::Public([4; 32]);

        run_to_block(3);

        assert_ok!(Did::rotate_key(
            Origin::signed(VALIDATOR_ACCOUNT),
            identifier.clone(),
            public_key2
        ));

        assert_eq!(DIDs::<Test>::contains_key(identifier.clone()), true);
        assert_eq!(Lookup::<Test>::contains_key(identifier.clone()), true);

        // Ensure only a singly pubkey is mapped to a DID -  inspired from toufeeq's testing
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key)),
            false
        );
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key2)),
            true
        );

        let (did_doc, block_number) = Did::get_did_details(identifier.clone()).unwrap();
        assert_eq!(did_doc.identifier, identifier);
        assert_eq!(did_doc.public_key, public_key2);
        assert_eq!(did_doc.metadata, metadata);
        assert_eq!(block_number, 3);

        // check the rotated key has been added to the history of the DID
        assert_eq!(PrevKeys::<Test>::contains_key(identifier.clone()), true);
        let prev_key_list = Did::get_prev_key_details(identifier.clone()).unwrap();
        assert_eq!(prev_key_list.is_empty(), false);
        assert_eq!(prev_key_list.len(), 1);

        let (last_pub_key, block_number) = prev_key_list.first().cloned().unwrap();
        assert_eq!(last_pub_key, Did::get_accountid_from_pubkey(&public_key));
        assert_eq!(block_number, 0);
    })
}

#[test]
fn test_rotate_key_history() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([3; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        let public_key2 = sr25519::Public([4; 32]);

        run_to_block(3);

        assert_ok!(Did::rotate_key(
            Origin::signed(VALIDATOR_ACCOUNT),
            identifier.clone(),
            public_key2
        ));

        run_to_block(8);

        let public_key3 = sr25519::Public([7; 32]);

        assert_ok!(Did::rotate_key(
            Origin::signed(VALIDATOR_ACCOUNT),
            identifier.clone(),
            public_key3
        ));

        assert_eq!(DIDs::<Test>::contains_key(identifier.clone()), true);
        assert_eq!(Lookup::<Test>::contains_key(identifier.clone()), true);

        // Ensure only a singly pubkey is mapped to a DID -  inspired from toufeeq's testing
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key)),
            false
        );
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key2)),
            false
        );
        assert_eq!(
            RLookup::<Test>::contains_key(Did::get_accountid_from_pubkey(&public_key3)),
            true
        );

        let (did_doc, block_number) = Did::get_did_details(identifier.clone()).unwrap();
        assert_eq!(did_doc.identifier, identifier);
        assert_eq!(did_doc.public_key, public_key3);
        assert_eq!(did_doc.metadata, metadata);
        assert_eq!(block_number, 8);

        // check the rotated key has been added to the history of the DID
        assert_eq!(PrevKeys::<Test>::contains_key(identifier.clone()), true);
        let prev_key_list = Did::get_prev_key_details(identifier.clone()).unwrap();
        assert_eq!(prev_key_list.is_empty(), false);
        assert_eq!(prev_key_list.len(), 2);

        let (last_pub_key, block_number) = prev_key_list[0];
        assert_eq!(last_pub_key, Did::get_accountid_from_pubkey(&public_key));
        assert_eq!(block_number, 0);

        let (last_pub_key2, block_number2) = prev_key_list[1];
        assert_eq!(last_pub_key2, Did::get_accountid_from_pubkey(&public_key2));
        assert_eq!(block_number2, 3);
    })
}

#[test]
#[should_panic]
fn test_rotate_did_for_non_existent_did() {
    new_test_ext().execute_with(|| {
        let identifier = *b"did:ssid:Alicx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let public_key = sr25519::Public([5; 32]);
        let metadata = vec![];

        assert_ok!(Did::add(
            Origin::signed(VALIDATOR_ACCOUNT),
            public_key,
            identifier,
            metadata.clone()
        ));

        let identifier2 = *b"Alice2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

        assert_ok!(Did::rotate_key(
            Origin::signed(VALIDATOR_ACCOUNT),
            identifier2.clone(),
            public_key
        ));
    })
}

#[test]
fn test_did_validation() {
    new_test_ext().execute_with(|| {
        // without did: prefix
        let without_did_colon = *b"Alicx\0\0\0\0\0\0\0\0\\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(!Did::is_did_valid(without_did_colon));

        // zero did
        let zero_did = [0; 32];
        assert!(!Did::is_did_valid(zero_did));

        // zero after did: prefix
        let zero_after_did_colon = *b"did:\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(!Did::is_did_valid(zero_after_did_colon));

        // space followed by zeros
        let space_followed_by_zero =
            *b" \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(!Did::is_did_valid(space_followed_by_zero));

        // space followed by correct did
        let space_followed_correct_did = *b" did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(!Did::is_did_valid(space_followed_correct_did));

        // correct did
        let correct_did = *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(Did::is_did_valid(correct_did));
    })
}
