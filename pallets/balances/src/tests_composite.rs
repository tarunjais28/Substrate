// This file is part of Substrate.

// Copyright (C) 2018-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

#![cfg(test)]

use crate::{self as pallet_balances, decl_tests, Config, Module};
use did;
use frame_benchmarking::account;
use frame_support::parameter_types;
use frame_support::weights::{DispatchInfo, IdentityFee, Weight};
use pallet_transaction_payment::CurrencyAdapter;
use sp_core::{sr25519, H256};
use sp_io;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use validator_set;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        Did: did::{Module, Call, Storage, Event, Config},
        ValidatorSet: validator_set::{Module, Call, Storage, Event, Config},

});

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub static ExistentialDeposit: u64 = 0;
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = BlockWeights;
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = super::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}
parameter_types! {
    pub const TransactionByteFee: u64 = 1;
}
impl pallet_transaction_payment::Config for Test {
    type OnChargeTransaction = CurrencyAdapter<Module<Test>, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<u64>;
    type FeeMultiplierUpdate = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}
impl Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Test>;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
    type DidResolution = Did;
}

impl did::Config for Test {
    type Event = Event;
}
type DidStruct = did::DidStruct;
impl validator_set::Config for Test {
    type Event = Event;
    type ApproveOrigin = frame_system::EnsureRoot<u64>;
}

pub struct ExtBuilder {
    existential_deposit: u64,
    monied: bool,
}
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            existential_deposit: 1,
            monied: false,
        }
    }
}
impl ExtBuilder {
    pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
        self.existential_deposit = existential_deposit;
        self
    }
    pub fn monied(mut self, monied: bool) -> Self {
        self.monied = monied;
        self
    }
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        self.set_associated_consts();
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        did::GenesisConfig {
            dids: vec![
                DidStruct {
                    identifier: *b"Alice\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: sr25519::Public([0; 32]),
                    metadata: vec![],
                },
                DidStruct {
                    identifier: *b"Alice2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: sr25519::Public([2; 32]),
                    metadata: vec![],
                },
                DidStruct {
                    identifier: *b"Alice3\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: sr25519::Public(account("recipient", 0, 0)),
                    metadata: vec![],
                },
            ],
        }
        .assimilate_storage::<Test>(&mut t)
        .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: if self.monied {
                vec![
                    (1, 10 * self.existential_deposit),
                    (2, 20 * self.existential_deposit),
                    (3, 30 * self.existential_deposit),
                    (4, 40 * self.existential_deposit),
                    (12, 10 * self.existential_deposit),
                ]
            } else {
                vec![]
            },
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

decl_tests! { Test, ExtBuilder, EXISTENTIAL_DEPOSIT }
