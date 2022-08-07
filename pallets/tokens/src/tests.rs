#![cfg(test)]

use super::*;
use crate::{self as tokens, Config};
use balances;
use did;
use frame_support::{
    assert_noop, assert_ok, ord_parameter_types, parameter_types, traits::StorageMapShim,
};
use frame_system::EnsureSignedBy;
use sp_core::{sr25519, Pair, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, Hash, IdentityLookup},
    Perbill,
};
use validator_set;
use vc;

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
        Tokens: tokens::{Module, Call, Storage, Event},
        ValidatorSet: validator_set::{Module, Call, Storage, Event, Config},
        Balances: balances::{Module, Call, Storage, Event<T>, Config<T>},
        VC: vc::{Module, Call, Storage, Event},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const SS58Prefix: u8 = 42;
}

type AccountId = u64;
impl frame_system::Config for Test {
    type Origin = Origin;
    type BlockWeights = ();
    type BlockLength = ();
    type Call = tests::Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

type CurrencyId = u32;
pub type Balance = u64;

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
    pub const MaxLocks: u32 = 50;
}

impl balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        balances::Account<Test>,
        frame_system::Provider<Test>,
        u64,
        balances::AccountData<u64>,
    >;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
    type DidResolution = DIDModule;
}

parameter_types! {
    // the minimum reserve amount required to create a new token
    pub const TreasuryReserveAmount: Balance = TREASURY_RESERVE_AMOUNT as u64; //10 million MUI - consider 6decimal places
}

ord_parameter_types! {
    pub const CouncilElectedUser: u64 = BOB_ACCOUNT_ID;
}

impl Config for Test {
    type Event = Event;
    type Amount = i64;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type Currency = Balances;
    type TreasuryReserve = TreasuryReserveAmount;
}

impl vc::Config for Test {
    type Event = Event;
    type ApproveOrigin = EnsureSignedBy<CouncilElectedUser, u64>;
}

impl did::Config for Test {
    type Event = Event;
}

impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

type DIDModule = did::Module<Test>;
type DidStruct = did::DidStruct;

pub const TEST_TOKEN_ID: CurrencyId = 1;
pub const ALICE: did::Did = *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
pub const BOB: did::Did = *b"did:ssid:bob\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
pub const DAVE: did::Did = *b"did:ssid:dave\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
pub const ALICE_ACCOUNT_ID: u64 = 2077282123132384724;
pub const BOB_ACCOUNT_ID: u64 = 7166219960988249998;
pub const DAVE_ACCOUNT_ID: u64 = 13620103657161844528;
pub const INITIAL_BALANCE: Balance = 100_000_000_000_000; // 100 million MUI
pub const TREASURY_RESERVE_AMOUNT: Balance = 10_000_000_000_000; //10 million MUI - consider 6decimal places
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

pub struct ExtBuilder {
    endowed_accounts: Vec<(did::Did, CurrencyId, Balance)>,
    treasury_genesis: bool,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![],
            treasury_genesis: false,
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let currency_code = convert_to_array::<8>("OTH".into());

        super::GenesisConfig {
            endowed_accounts: vec![(BOB, currency_code, INITIAL_BALANCE.into())],
        }
        .assimilate_storage::<Test>(&mut t)
        .unwrap();

        did::GenesisConfig {
            dids: vec![
                DidStruct {
                    identifier: ALICE,
                    public_key: sr25519::Pair::from_seed(&ALICE_SEED).public(),
                    metadata: vec![],
                },
                DidStruct {
                    identifier: BOB,
                    public_key: sr25519::Pair::from_seed(&BOB_SEED).public(),
                    metadata: vec![],
                },
                DidStruct {
                    identifier: DAVE,
                    public_key: sr25519::Pair::from_seed(&DAVE_SEED).public(),
                    metadata: vec![],
                },
            ],
        }
        .assimilate_storage::<Test>(&mut t)
        .unwrap();

        validator_set::GenesisConfig {
            validators: vec![ALICE],
        }
        .assimilate_storage::<Test>(&mut t)
        .unwrap();

        balances::GenesisConfig::<Test> {
            balances: vec![
                (BOB_ACCOUNT_ID, INITIAL_BALANCE),
                (ALICE_ACCOUNT_ID, INITIAL_BALANCE),
                (DAVE_ACCOUNT_ID, INITIAL_BALANCE),
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

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
fn genesis_config_works() {
    ExtBuilder::default().build().execute_with(|| {
        let (did_doc, block_number) = DIDModule::get_did_details(ALICE.clone()).unwrap();
        assert_eq!(did_doc.identifier, ALICE);
        assert_eq!(
            did_doc.public_key,
            sr25519::Pair::from_seed(&ALICE_SEED).public()
        );
        assert_eq!(block_number, 0);
    });
}

#[test]
fn only_vc_owner_can_issue_token() {
    ExtBuilder::default().build().execute_with(|| {
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];
        // issue token failed due to non-registered account
        assert_noop!(
            Tokens::issue_token(Origin::signed(0), vc_id, token_amount),
            vc::Error::<Test>::DidNotRegisteredWithVC
        );
    });
}

#[test]
fn issue_token_works() {
    ExtBuilder::default().build().execute_with(|| {
        let reservable_balance: u128 = 1000000;
        let currency_code: CurrencyCode = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: reservable_balance,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];

        let token_amount: u128 = 5_000_000;
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        // check balance has been reserved correctly
        assert_eq!(
            Balances::free_balance(BOB_ACCOUNT_ID),
            INITIAL_BALANCE - reservable_balance as u64
        );
        assert_eq!(
            Balances::reserved_balance(BOB_ACCOUNT_ID),
            reservable_balance as u64
        );

        // check created token details
        assert_eq!(Tokens::total_issuance(currency_code), token_amount);
        assert_eq!(
            Tokens::token_data(currency_code).unwrap(),
            TokenDetails {
                token_name: "test".into(),
                currency_code: "OTH".into(),
                decimal: 6,
                block_number: 1,
            }
        );

        // check entire token supply is credited to the creator account
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount
        );

        // check if the token owner/issuer is correct
        assert_eq!(
            Tokens::token_issuer(currency_code),
            DIDModule::get_did_from_account_id(&BOB_ACCOUNT_ID)
        );

        // checking slash token vc works after being used
        assert_noop!(
            Tokens::issue_token(Origin::signed(BOB_ACCOUNT_ID), vc_id, token_amount),
            vc::Error::<Test>::VCAlreadyUsed
        );
    });
}

#[test]
fn test_transfer_token_works() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());

        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];

        let token_amount: u128 = 5_000_000;
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let transfer_amount: u128 = 1_000_000;
        assert_ok!(Tokens::transfer(
            Origin::signed(BOB_ACCOUNT_ID),
            DAVE_ACCOUNT_ID,
            currency_code,
            transfer_amount,
        ));

        // check balance transfer worked correctly
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - transfer_amount
        );
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &DAVE_ACCOUNT_ID),
            transfer_amount
        );
        assert_eq!(Tokens::total_issuance(currency_code), token_amount);

        // cannot transfer more than balance
        assert_noop!(
            Tokens::transfer(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE_ACCOUNT_ID,
                currency_code,
                TREASURY_RESERVE_AMOUNT.into()
            ),
            Error::<Test>::BalanceTooLow
        );
    });
}

#[test]
fn test_withdraw_reserve_works() {
    ExtBuilder::default().build().execute_with(|| {
        let reservable_balance: u128 = 1000000;
        let currency_code: CurrencyCode = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: reservable_balance,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            5_000_000
        ));

        assert_ok!(Tokens::withdraw_reserved(
            Origin::signed(BOB_ACCOUNT_ID),
            DAVE_ACCOUNT_ID,
            BOB_ACCOUNT_ID,
            1000000
        ));

        // check balance has been credited correctly
        assert_eq!(
            Balances::free_balance(BOB_ACCOUNT_ID),
            INITIAL_BALANCE - reservable_balance as u64
        );
        assert_eq!(
            Balances::reserved_balance(BOB_ACCOUNT_ID),
            reservable_balance as u64 - 1000000
        );
        assert_eq!(
            Balances::total_balance(&BOB_ACCOUNT_ID),
            INITIAL_BALANCE - 1000000
        );
        assert_eq!(
            Balances::free_balance(DAVE_ACCOUNT_ID),
            INITIAL_BALANCE + 1000000
        );

        // check created token details
        assert_eq!(Tokens::total_issuance(currency_code), 5000000);
        assert_eq!(
            Tokens::token_data(currency_code).unwrap(),
            TokenDetails {
                token_name: "test".into(),
                currency_code: "OTH".into(),
                decimal: 6,
                block_number: 1,
            }
        );

        // check entire token supply is credited to the creator account
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            5000000
        );
        assert_eq!(
            Tokens::token_issuer(currency_code),
            DIDModule::get_did_from_account_id(&BOB_ACCOUNT_ID)
        );
    });
}

#[test]
fn test_get_currency_id() {
    ExtBuilder::default().build().execute_with(|| {
        // derive currency id first time
        assert_eq!(Tokens::get_currency_id(), 1);
        // Set currency id
        Tokens::set_currency_id(1);
        // derive currency id second time
        assert_eq!(Tokens::get_currency_id(), 2);
    });
}

#[test]
fn test_slash_token() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code: CurrencyCode = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner: BOB,
            issuers: vec![BOB],
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];

        let token_amount: u128 = 5_000_000;
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let slash_amount: u128 = 1_000_000;
        let slash_vc = vc::SlashMintTokens {
            vc_id,
            currency_code,
            amount: slash_amount,
        };

        let slash_vc: [u8; 128] = convert_to_array::<128>(slash_vc.encode());
        let vc_type = vc::VCType::SlashTokens;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = DAVE;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &slash_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: slash_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(DAVE_ACCOUNT_ID),
            vc_struct.encode()
        ));
        let vc_id = vc::Lookup::get(&DAVE)[0];

        assert_ok!(Tokens::slash_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id));

        // checking correctness of free balance after slash
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - slash_amount
        );

        // checking slash token vc works after being used
        assert_noop!(
            Tokens::slash_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id),
            vc::Error::<Test>::VCAlreadyUsed
        );
    });
}

#[test]
fn test_mint_token() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code: CurrencyCode = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];

        let token_amount: u128 = 5_000_000;
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let mint_amount: u128 = 1_000_000;
        let mint_vc = vc::SlashMintTokens {
            vc_id,
            currency_code,
            amount: mint_amount,
        };

        let mint_vc: [u8; 128] = convert_to_array::<128>(mint_vc.encode());
        let vc_type = vc::VCType::MintTokens;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = DAVE;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &mint_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: mint_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(DAVE_ACCOUNT_ID),
            vc_struct.encode()
        ));
        let vc_id = vc::Lookup::get(&DAVE)[0];

        assert_ok!(Tokens::mint_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id));

        // checking correctness of free balance after mint
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount + mint_amount
        );
        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount + mint_amount
        );

        // checking mint token vc works after being used
        assert_noop!(
            Tokens::mint_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id),
            vc::Error::<Test>::VCAlreadyUsed
        );
    });
}

#[test]
fn test_transfer_token() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code: CurrencyCode = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = BOB;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let vc_id = vc::Lookup::get(&BOB)[0];

        let token_amount: u128 = 5_000_000;
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let transfer_amount: u128 = 1_000_000;
        let transfer_vc = vc::TokenTransferVC {
            vc_id,
            currency_code,
            amount: transfer_amount,
        };

        let transfer_vc: [u8; 128] = convert_to_array::<128>(transfer_vc.encode());
        let vc_type = vc::VCType::TokenTransferVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&BOB_SEED);
        let owner = DAVE;
        let issuers = vec![BOB];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &transfer_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: transfer_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(DAVE_ACCOUNT_ID),
            vc_struct.encode()
        ));
        let vc_id = vc::Lookup::get(&DAVE)[0];

        assert_ok!(Tokens::transfer_token(
            Origin::signed(DAVE_ACCOUNT_ID),
            vc_id,
            ALICE_ACCOUNT_ID
        ));

        // checking amount transfered
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &ALICE_ACCOUNT_ID),
            transfer_amount
        );

        // checking correctness of free balance after transfer
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - transfer_amount
        );

        assert_eq!(Tokens::total_issuance(currency_code), token_amount);

        // checking transfer token vc works after being used
        assert_noop!(
            Tokens::transfer_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id, ALICE_ACCOUNT_ID),
            vc::Error::<Test>::VCAlreadyUsed
        );
    });
}

#[test]
fn test_decimal_and_ccy_code() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];
        // issue token
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let account_details = Accounts::<Test>::get(BOB, currency_code);
        assert_eq!(account_details.data.free, token_amount);
        assert_eq!(account_details.data.reserved, 0);
        assert_eq!(account_details.data.frozen, 0);
    });
}

#[test]
fn test_ccy_code_exists() {
    ExtBuilder::default().build().execute_with(|| {
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        // First time tokens will be issued
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test-2".into()),
            reservable_balance: 2_000_000,
            decimal: 6,
            currency_code: convert_to_array::<8>("OTH".into()),
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = DAVE;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&DAVE)[0];
        // Second time tokens will not be issued as currency_code already registered
        assert_noop!(
            Tokens::issue_token(Origin::signed(DAVE_ACCOUNT_ID), vc_id, token_amount),
            Error::<Test>::CurrencyCodeAlreadyRegistered
        );
    });
}

#[test]
fn test_set_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount: u128 = 1_000_000;

        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            )
        );

        // checking amount transfered
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &DAVE_ACCOUNT_ID),
            new_amount
        );

        // checking correctness of free balance after transfer
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - new_amount
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}


#[test]
fn test_set_whole_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));


        let new_amount = token_amount;
        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            )
        );

        // checking amount transfered
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &DAVE_ACCOUNT_ID),
            new_amount
        );

        // checking correctness of free balance after transfer
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - new_amount
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}

#[test]
fn test_set_balance_greater_amount() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount = 6_000_000;
        assert_noop!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
            Error::<Test>::TokenAmountOverflow
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}


#[test]
fn test_set_balance_less_than_existing() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount = 4_000_000;
        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
        );

        let new_amount = 1_000_000;
        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
        );

        // checking amount transfered
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &DAVE_ACCOUNT_ID),
            new_amount
        );

        // checking correctness of free balance after transfer
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount - new_amount
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}

#[test]
fn test_set_balance_zero() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        // First time tokens will be issued
        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount = 4_000_000;
        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
        );

        let new_amount = 0;
        assert_ok!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
        );

        // checking amount transfered
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &DAVE_ACCOUNT_ID),
            new_amount
        );

        // checking correctness of free balance after transfer
        assert_eq!(
            Tokens::free_balance(TEST_TOKEN_ID, &BOB_ACCOUNT_ID),
            token_amount
        );


        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}

#[test]
fn test_set_balance_token_owner() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount = 1_000_000;
        assert_noop!(
            Tokens::set_balance(
                Origin::signed(BOB_ACCOUNT_ID),
                BOB,
                currency_code,
                new_amount,
            ),
            Error::<Test>::NotAllowed
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}

#[test]
fn test_set_balance_not_token_owner() {
    ExtBuilder::default().build().execute_with(|| {
        let currency_code = convert_to_array::<8>("OTH".into());
        let token_vc = vc::TokenVC {
            token_name: convert_to_array::<16>("test".into()),
            reservable_balance: 1_000_000,
            decimal: 6,
            currency_code,
        };

        let token_vc: [u8; 128] = convert_to_array::<128>(token_vc.encode());
        let vc_type = vc::VCType::TokenVC;
        let pair: sr25519::Pair = sr25519::Pair::from_seed(&ALICE_SEED);
        let owner = BOB;
        let issuers = vec![ALICE];
        let hash = BlakeTwo256::hash_of(&(&vc_type, &token_vc, &owner, &issuers));
        let signature = pair.sign(hash.as_ref());

        let vc_struct: vc::VC<H256> = vc::VC {
            hash,
            signatures: vec![signature],
            vc_type,
            owner,
            issuers,
            is_vc_used: false,
            vc_property: token_vc,
        };

        assert_ok!(VC::store(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_struct.encode()
        ));

        let token_amount: u128 = 5_000_000;
        let vc_id = vc::Lookup::get(&BOB)[0];

        assert_ok!(Tokens::issue_token(
            Origin::signed(BOB_ACCOUNT_ID),
            vc_id,
            token_amount
        ));

        let new_amount = 1_000_000;
        assert_noop!(
            Tokens::set_balance(
                Origin::signed(DAVE_ACCOUNT_ID),
                DAVE,
                currency_code,
                new_amount,
            ),
            Error::<Test>::NotAllowed
        );

        assert_eq!(
            Tokens::total_issuance(currency_code),
            token_amount
        );
    });
}