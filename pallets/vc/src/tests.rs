use super::*;
use crate::{self as verified_credentials, Config};
use did;
use frame_support::{assert_noop, assert_ok, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use sp_core::{sr25519, Pair, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::TryInto;
use validator_set;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

ord_parameter_types! {
    pub const ValidAccount: u64 = BOB_ACCOUNT_ID;
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        VC: verified_credentials::{Module, Call, Storage, Event},
        ValidatorSet: validator_set::{Module, Call, Storage, Event, Config},
        Did: did::{Module, Call, Storage, Event, Config},
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
    type ApproveOrigin = EnsureSignedBy<ValidAccount, u64>;
}

impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = EnsureSignedBy<ValidAccount, u64>;
}

impl did::Config for Test {
    type Event = Event;
}

const NON_VALIDATOR_ACCOUNT: u64 = 2;
const ALICE: did::Did = *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const BOB: did::Did = *b"did:ssid:bob\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const DAVE: did::Did = *b"did:ssid:dave\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const EVE: did::Did = *b"did:ssid:eve\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const ALICE_ACCOUNT_ID: u64 = 2077282123132384724;
const BOB_ACCOUNT_ID: u64 = 7166219960988249998;
const ALICE_SEED: [u8; 32] = [
    229, 190, 154, 80, 146, 184, 27, 202, 100, 190, 129, 210, 18, 231, 242, 249, 235, 161, 131,
    187, 122, 144, 149, 79, 123, 118, 54, 31, 110, 219, 92, 10,
];
const BOB_SEED: [u8; 32] = [
    57, 143, 12, 40, 249, 136, 133, 224, 70, 51, 61, 74, 65, 193, 156, 238, 76, 55, 54, 138, 152,
    50, 198, 80, 47, 108, 253, 24, 46, 42, 239, 137,
];
const DAVE_SEED: [u8; 32] = [
    134, 128, 32, 174, 6, 135, 221, 167, 213, 117, 101, 9, 58, 105, 9, 2, 17, 68, 152, 69, 167,
    225, 20, 83, 97, 40, 0, 182, 99, 48, 114, 70,
];
const EVE_SEED: [u8; 32] = [
    120, 106, 208, 226, 223, 69, 111, 228, 61, 209, 249, 30, 188, 162, 46, 35, 91, 193, 98, 224,
    187, 141, 83, 198, 51, 232, 200, 91, 42, 246, 139, 122,
];

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
fn new_test_ext() -> sp_io::TestExternalities {
    let mut o = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
    // let vc: Vec<u8> = vec![
    //     65, 108, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //     0, 0, 0, 0, 65, 108, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //     0, 0, 0, 0, 0, 0, 0, 0, 252, 48, 108, 23, 89, 14, 11, 176, 20, 20, 127, 49, 19, 39, 250,
    //     139, 21, 93, 151, 92, 223, 184, 79, 121, 58, 205, 209, 146, 217, 162, 229, 89, 0, 188, 126,
    //     39, 174, 121, 218, 194, 177, 241, 52, 236, 12, 23, 109, 17, 189, 18, 217, 32, 212, 161, 19,
    //     146, 184, 151, 105, 127, 117, 166, 226, 194, 8, 32, 11, 234, 171, 157, 238, 181, 242, 28,
    //     186, 194, 248, 72, 187, 234, 120, 167, 193, 252, 153, 117, 74, 171, 197, 5, 51, 34, 153,
    //     238, 130, 187, 73, 62, 218, 54, 238, 25, 197, 216, 225, 168, 235, 123, 41, 83, 37, 67, 253,
    //     185, 1, 168, 184, 49, 226, 45, 108, 219, 108, 88, 130, 255, 152, 119, 130, 1,
    // ];

    // super::GenesisConfig {
    //     init_vcs: vec![InitialVCs {
    //         identifier: ALICE,
    //         public_key: pair.public(),
    //         vcs: vec![vc],
    //     }],
    // }
    // .assimilate_storage::<Test>(&mut o)
    // .unwrap();

    validator_set::GenesisConfig {
        validators: vec![ALICE],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    did::GenesisConfig {
        dids: vec![did::DidStruct {
            identifier: ALICE,
            public_key: pair.public(),
            metadata: Vec::default(),
        }],
    }
    .assimilate_storage::<Test>(&mut o)
    .unwrap();

    o.into()
}

// #[test]
// fn test_genesis_worked() {
//     new_test_ext().execute_with(|| {
//         assert!(VCs::<Test>::contains_key(ALICE));
//         assert!(DIDs::<Test>::contains_key(ALICE_ACCOUNT_ID));
//         assert!(RLookup::<Test>::contains_key(ALICE_ACCOUNT_ID));

//         let genesis_vc: Vec<u8> = vec![
//             65, 108, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0, 0, 0, 0, 65, 108, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 252, 48, 108, 23, 89, 14, 11, 176, 20, 20, 127, 49,
//             19, 39, 250, 139, 21, 93, 151, 92, 223, 184, 79, 121, 58, 205, 209, 146, 217, 162, 229,
//             89, 0, 188, 126, 39, 174, 121, 218, 194, 177, 241, 52, 236, 12, 23, 109, 17, 189, 18,
//             217, 32, 212, 161, 19, 146, 184, 151, 105, 127, 117, 166, 226, 194, 8, 32, 11, 234,
//             171, 157, 238, 181, 242, 28, 186, 194, 248, 72, 187, 234, 120, 167, 193, 252, 153, 117,
//             74, 171, 197, 5, 51, 34, 153, 238, 130, 187, 73, 62, 218, 54, 238, 25, 197, 216, 225,
//             168, 235, 123, 41, 83, 37, 67, 253, 185, 1, 168, 184, 49, 226, 45, 108, 219, 108, 88,
//             130, 255, 152, 119, 130, 1,
//         ];
//         assert!(Members::get().contains(&genesis_vc));
//         assert!(Lookup::<Test>::contains_key(genesis_vc));
//     })
// }

fn convert_to_array<const N: usize>(mut v: Vec<u8>) -> [u8; N] {
    if v.len() != N {
        for _ in v.len()..N {
            v.push(0);
        }
    }
    v.try_into().unwrap_or_else(|v: Vec<u8>| {
        panic!("Expected a Vec of length {} but it was {}", N, v.len())
    })
}

#[test]
fn test_store() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = *BlakeTwo256::hash_of(&vc).as_fixed_bytes();
        let did = RLookup::get(vc_id);
        assert_eq!(did, BOB);
        assert_eq!(Lookup::get(did), vec![vc_id]);
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Active))
        );
        assert_eq!(VCHistory::<Test>::get(vc_id), Some((VCStatus::Active, 0)));
    })
}

#[test]
fn test_update_status() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = Lookup::get(&BOB)[0];
        // Updating status flag
        assert_ok!(VC::update_status(
            Origin::signed(ALICE_ACCOUNT_ID),
            vc_id,
            VCStatus::Inactive
        ));

        assert_eq!(VCs::<Test>::get(vc_id), Some((vc, VCStatus::Inactive)));
    })
}

#[test]
fn test_store_vc_with_different_account() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(NON_VALIDATOR_ACCOUNT), vc.encode()),
            DispatchError::BadOrigin
        );
    })
}

#[test]
fn test_store_vc_with_wrong_hash() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        // Wrong Hash
        let hash = H256::zero();
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner: BOB,
            issuers: vec![BOB],
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::VCPropertiesNotVerified
        );
    })
}

#[test]
fn test_store_vc_with_wrong_signature() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let wrong_hash = H256::zero();
        let signature = pair.sign(wrong_hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::InvalidSignature
        );
    })
}

#[test]
fn test_store_vc_less_approvers() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB, ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let wrong_hash = H256::zero();
        let signature = pair.sign(wrong_hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::InvalidSignature
        );
    })
}

#[test]
fn test_update_status_sender() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = Lookup::get(&BOB)[0];
        let non_issuer = 0;

        // Updating status flag with non issuer account
        assert_noop!(
            VC::update_status(Origin::signed(non_issuer), vc_id, VCStatus::Inactive),
            Error::<Test>::NotAValidatorNorIssuer
        );

        // Updating status flag with non validator account
        assert_noop!(
            VC::update_status(
                Origin::signed(NON_VALIDATOR_ACCOUNT),
                vc_id,
                VCStatus::Inactive
            ),
            Error::<Test>::NotAValidatorNorIssuer
        );
    })
}

#[test]
fn test_add_signature() {
    new_test_ext().execute_with(|| {
        let bob_pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let bob_public_key = bob_pair.public();

        let dave_pair: sr25519::Pair = sr25519::Pair::from_seed(&DAVE_SEED);
        let dave_public_key = dave_pair.public();

        let eve_pair: sr25519::Pair = sr25519::Pair::from_seed(&EVE_SEED);
        let eve_public_key = eve_pair.public();

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB, DAVE, EVE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let bob_sign = bob_pair.sign(hash.as_ref());
        let dave_sign = dave_pair.sign(hash.as_ref());
        let eve_sign = eve_pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign.clone()],
            vc_type: vc_type.clone(),
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        // creating BOB's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            bob_public_key,
            BOB,
            Vec::new()
        ));

        // creating DAVE's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            dave_public_key,
            DAVE,
            Vec::new()
        ));

        // creating EVE's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            eve_public_key,
            EVE,
            Vec::new()
        ));

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = Lookup::get(&BOB)[0];

        // vc_status = Inactive as only one issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Inactive))
        );

        // updating DAVE's signature
        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign.clone(), dave_sign.clone()],
            vc_type: vc_type.clone(),
            owner: BOB,
            issuers: vec![BOB, DAVE, EVE],
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::add_signature(
            Origin::signed(ALICE_ACCOUNT_ID),
            vc_id,
            dave_sign.clone()
        ));

        // vc_status = Inactive as only two issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Inactive))
        );

        // updating EVE's signature
        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign, dave_sign, eve_sign.clone()],
            vc_type,
            owner: BOB,
            issuers: vec![BOB, DAVE, EVE],
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::add_signature(
            Origin::signed(ALICE_ACCOUNT_ID),
            vc_id,
            eve_sign
        ));

        // vc_status = Active as only all issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Active))
        );
    })
}

#[test]
fn test_add_signature_with_one_of_the_signers() {
    new_test_ext().execute_with(|| {
        let bob_pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let bob_public_key = bob_pair.public();

        let dave_pair: sr25519::Pair = sr25519::Pair::from_seed(&DAVE_SEED);
        let dave_public_key = dave_pair.public();

        let eve_pair: sr25519::Pair = sr25519::Pair::from_seed(&EVE_SEED);
        let eve_public_key = eve_pair.public();

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB, DAVE, EVE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let bob_sign = bob_pair.sign(hash.as_ref());
        // signed by Dave's public key
        let dave_sign = dave_pair.sign(hash.as_ref());
        // signed by Eve's public key
        let eve_sign = eve_pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign.clone()],
            vc_type: vc_type.clone(),
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        // creating BOB's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            bob_public_key,
            BOB,
            Vec::new()
        ));

        // creating DAVE's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            dave_public_key,
            DAVE,
            Vec::new()
        ));

        // creating EVE's did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            eve_public_key,
            EVE,
            Vec::new()
        ));

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = Lookup::get(&BOB)[0];

        // vc_status = Inactive as only one issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Inactive))
        );

        // updating DAVE's signature
        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign.clone(), dave_sign.clone()],
            vc_type: vc_type.clone(),
            owner: BOB,
            issuers: vec![BOB, DAVE, EVE],
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::add_signature(
            Origin::signed(ALICE_ACCOUNT_ID),
            vc_id,
            dave_sign.clone()
        ));

        // vc_status = Inactive as only two issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Inactive))
        );

        // updating EVE's signature
        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![bob_sign, dave_sign, eve_sign.clone()],
            vc_type,
            owner: BOB,
            issuers: vec![BOB, DAVE, EVE],
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::add_signature(
            Origin::signed(ALICE_ACCOUNT_ID),
            vc_id,
            eve_sign
        ));

        // vc_status = Active as only all issuer signed
        assert_eq!(
            VCs::<Test>::get(vc_id),
            Some((vc.clone(), VCStatus::Active))
        );
    })
}

#[test]
fn test_set_is_used_flag() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        // Adding did
        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));

        let vc_id = Lookup::get(&BOB)[0];

        // set vc is_used flag as true
        VC::set_is_used_flag(vc_id);
        let vc_details = VCs::<Test>::get(vc_id).unwrap();
        assert!(vc_details.0.is_vc_used);
    })
}

#[test]
fn test_duplicate_issuers_signatures() {
    new_test_ext().execute_with(|| {
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        // case when duplicate signatures are present
        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature.clone(), signature.clone()],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::DuplicateSignature
        );

        // case when duplicate issuers are present
        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB, BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::DuplicateSignature
        );
    })
}


#[test]
fn test_add_duplicate_issuer_signatures() {
    new_test_ext().execute_with(|| {

        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let public_key = pair.public();

        let dave_pair: sr25519::Pair = sr25519::Pair::from_seed(&DAVE_SEED);

        let token_vc = TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        // case when duplicate signatures are present
        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = VCType::TokenVC;
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());
        let duplicate_signature = pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature.clone(), duplicate_signature.clone()],
            vc_type: vc_type.clone(),
            owner,
            issuers: issuers.clone(),
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(Did::add(
            Origin::signed(ALICE_ACCOUNT_ID),
            public_key,
            BOB,
            Vec::new()
        ));

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::DuplicateSignature
        );


        let dave_sign = dave_pair.sign(hash.as_ref());

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature.clone(), dave_sign],
            vc_type: vc_type.clone(),
            owner,
            issuers: issuers.clone(),
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_noop!(
            VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()),
            Error::<Test>::InvalidSignature
        );

        let vc: verified_credentials::VC<H256> = verified_credentials::VC {
            hash,
            signatures: vec![signature.clone()],
            vc_type,
            owner,
            issuers,
            is_vc_used: true,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(Origin::signed(BOB_ACCOUNT_ID), vc.encode()));
        
        let vc_id = Lookup::get(&BOB)[0];

        assert_noop!(
            VC::add_signature(
                Origin::signed(ALICE_ACCOUNT_ID),
                vc_id,
                duplicate_signature
            ),
            Error::<Test>::DuplicateSignature,
        );
    })
}
