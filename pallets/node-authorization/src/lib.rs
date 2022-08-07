// This file is part of Substrate.

// Copyright (C) 2019-2020 Parity Technologies (UK) Ltd.
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

//! # Node authorization pallet
//!
//! This pallet manages a configurable set of nodes for a permissioned network.
//! Each node is dentified by a PeerId (i.e. Vec<u8>). It provides two ways to
//! authorize a node,
//!
//! - a set of well known nodes across different organizations in which the
//! connections are allowed.
//! - users can claim the ownership for each node, then manage the connections of
//! the node.
//!
//! A node must have an owner. The owner can additionally change the connections
//! for the node. Only one user is allowed to claim a specific node. To eliminate
//! false claim, the maintainer of the node should claim it before even starting the
//! node. This pallet uses offchain worker to set reserved nodes, if the node is not
//! an authority, make sure to enable offchain worker with the right CLI flag. The
//! node can be lagged with the latest block, in this case you need to disable offchain
//! worker and manually set reserved nodes when starting it.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use did;
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{EnsureOrigin, Get},
    weights::{DispatchClass, Weight},
};
use frame_system::ensure_signed;
use sp_core::OpaquePeerId as PeerId;
use sp_std::{collections::btree_set::BTreeSet, iter::FromIterator, prelude::*};

pub type Did = [u8; 32];

pub trait WeightInfo {
    fn add_well_known_node() -> Weight;
    fn remove_well_known_node() -> Weight;
    fn swap_well_known_node() -> Weight;
    fn reset_well_known_nodes() -> Weight;
    fn claim_node() -> Weight;
    fn remove_claim() -> Weight;
    fn transfer_node() -> Weight;
    fn add_connections() -> Weight;
    fn remove_connections() -> Weight;
}

impl WeightInfo for () {
    fn add_well_known_node() -> Weight {
        50_000_000
    }
    fn remove_well_known_node() -> Weight {
        50_000_000
    }
    fn swap_well_known_node() -> Weight {
        50_000_000
    }
    fn reset_well_known_nodes() -> Weight {
        50_000_000
    }
    fn claim_node() -> Weight {
        50_000_000
    }
    fn remove_claim() -> Weight {
        50_000_000
    }
    fn transfer_node() -> Weight {
        50_000_000
    }
    fn add_connections() -> Weight {
        50_000_000
    }
    fn remove_connections() -> Weight {
        50_000_000
    }
}

pub trait Config: frame_system::Config + did::Config {
    /// The event type of this module.
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

    /// The maximum number of well known nodes that are allowed to set
    type MaxWellKnownNodes: Get<u32>;

    /// The maximum length in bytes of PeerId
    type MaxPeerIdLength: Get<u32>;

    /// The origin which can add a well known node.
    type AddOrigin: EnsureOrigin<Self::Origin>;

    /// The origin which can remove a well known node.
    type RemoveOrigin: EnsureOrigin<Self::Origin>;

    /// The origin which can swap the well known nodes.
    type SwapOrigin: EnsureOrigin<Self::Origin>;

    /// The origin which can reset the well known nodes.
    type ResetOrigin: EnsureOrigin<Self::Origin>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as NodeAuthorization {
        /// The set of well known nodes. This is stored sorted (just by value).
        pub WellKnownNodes get(fn well_known_nodes): BTreeSet<PeerId>;
        /// A map that maintains the ownership of each node.
        pub Owners get(fn owners):
            map hasher(blake2_128_concat) PeerId => Did;
        /// The additional adapative connections of each node.
        pub AdditionalConnections get(fn additional_connection):
            map hasher(blake2_128_concat) PeerId => BTreeSet<PeerId>;
    }
    add_extra_genesis {
        config(nodes): Vec<(PeerId, Did)>;
        build(|config: &GenesisConfig| {
            <Module<T>>::initialize_nodes(&config.nodes)
        })
    }
}

decl_event! {
    pub enum Event
    {
        /// The given well known node was added.
        NodeAdded(PeerId, Did),
        /// The given well known node was removed.
        NodeRemoved(PeerId),
        /// The given well known node was swapped; first item was removed,
        /// the latter was added.
        NodeSwapped(PeerId, PeerId),
        /// The given well known nodes were reset.
        NodesReset(Vec<(PeerId, Did)>),
        /// The given node was claimed by a user.
        NodeClaimed(PeerId, Did),
        /// The given claim was removed by its owner.
        ClaimRemoved(PeerId, Did),
        /// The node was transferred to another account.
        NodeTransferred(PeerId, Did),
        /// The allowed connections were added to a node.
        ConnectionsAdded(PeerId, Vec<PeerId>),
        /// The allowed connections were removed from a node.
        ConnectionsRemoved(PeerId, Vec<PeerId>),
    }
}

decl_error! {
    /// Error for the node authorization module.
    pub enum Error for Module<T: Config> {
        /// The PeerId is too long.
        PeerIdTooLong,
        /// Too many well known nodes.
        TooManyNodes,
        /// The node is already joined in the list.
        AlreadyJoined,
        /// The node doesn't exist in the list.
        NotExist,
        /// The node is already claimed by a user.
        AlreadyClaimed,
        /// The node hasn't been claimed yet.
        NotClaimed,
        /// You are not the owner of the node.
        NotOwner,
        /// No permisson to perform specific operation.
        PermissionDenied,
        /// Given DID is not registered.
        DIDNotRegistered
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        /// The maximum number of authorized well known nodes
        const MaxWellKnownNodes: u32 = T::MaxWellKnownNodes::get();

        /// The maximum length in bytes of PeerId
        const MaxPeerIdLength: u32 = T::MaxPeerIdLength::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Add a node to the set of well known nodes. If the node is already claimed, the owner
        /// will be updated and keep the existing additional connection unchanged.
        ///
        /// May only be called from `T::AddOrigin`.
        ///
        /// - `node`: identifier of the node.
        #[weight = (T::WeightInfo::add_well_known_node(), DispatchClass::Operational)]
        pub fn add_well_known_node(origin, node: PeerId, owner: Did) {
            T::AddOrigin::ensure_origin(origin)?;
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(did::Module::<T>::did_registered(&owner), Error::<T>::DIDNotRegistered);
            let mut nodes = WellKnownNodes::get();
            ensure!(nodes.len() < T::MaxWellKnownNodes::get() as usize, Error::<T>::TooManyNodes);
            ensure!(!nodes.contains(&node), Error::<T>::AlreadyJoined);

            nodes.insert(node.clone());

            WellKnownNodes::put(&nodes);
            Owners::insert(&node, &owner);

            Self::deposit_event(Event::NodeAdded(node, owner));
        }

        /// Remove a node from the set of well known nodes. The ownership and additional
        /// connections of the node will also be removed.
        ///
        /// May only be called from `T::RemoveOrigin`.
        ///
        /// - `node`: identifier of the node.
        #[weight = (T::WeightInfo::remove_well_known_node(), DispatchClass::Operational)]
        pub fn remove_well_known_node(origin, node: PeerId) {
            T::RemoveOrigin::ensure_origin(origin)?;
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);

            let mut nodes = WellKnownNodes::get();
            ensure!(nodes.contains(&node), Error::<T>::NotExist);

            nodes.remove(&node);

            WellKnownNodes::put(&nodes);
            Owners::remove(&node);
            AdditionalConnections::remove(&node);

            Self::deposit_event(Event::NodeRemoved(node));
        }

        /// Swap a well known node to another. Both the ownership and additional connections
        /// stay untouched.
        ///
        /// May only be called from `T::SwapOrigin`.
        ///
        /// - `remove`: the node which will be moved out from the list.
        /// - `add`: the node which will be put in the list.
        #[weight = (T::WeightInfo::swap_well_known_node(), DispatchClass::Operational)]
        pub fn swap_well_known_node(origin, remove: PeerId, add: PeerId) {
            T::SwapOrigin::ensure_origin(origin)?;
            ensure!(remove.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(add.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);

            if remove == add { return Ok(()) }

            let mut nodes = WellKnownNodes::get();
            ensure!(nodes.contains(&remove), Error::<T>::NotExist);
            ensure!(!nodes.contains(&add), Error::<T>::AlreadyJoined);

            nodes.remove(&remove);
            nodes.insert(add.clone());

            WellKnownNodes::put(&nodes);
            Owners::swap(&remove, &add);
            AdditionalConnections::swap(&remove, &add);

            Self::deposit_event(Event::NodeSwapped(remove, add));
        }

        /// Reset all the well known nodes. This will not remove the ownership and additional
        /// connections for the removed nodes. The node owner can perform further cleaning if
        /// they decide to leave the network.
        ///
        /// May only be called from `T::ResetOrigin`.
        ///
        /// - `nodes`: the new nodes for the allow list.
        #[weight = (T::WeightInfo::reset_well_known_nodes(), DispatchClass::Operational)]
        pub fn reset_well_known_nodes(origin, nodes: Vec<(PeerId, Did)>) {
            T::ResetOrigin::ensure_origin(origin)?;
            ensure!(nodes.len() < T::MaxWellKnownNodes::get() as usize, Error::<T>::TooManyNodes);

            Self::initialize_nodes(&nodes);

            Self::deposit_event(Event::NodesReset(nodes));
        }

        /// A given node can be claimed by anyone. The owner should be the first to know its
        /// PeerId, so claim it right away!
        ///
        /// - `node`: identifier of the node.
        #[weight = T::WeightInfo::claim_node()]
        pub fn claim_node(origin, node: PeerId) {
            let sender = ensure_signed(origin)?;
            let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(!Owners::contains_key(&node),Error::<T>::AlreadyClaimed);
            ensure!(did::Module::<T>::did_registered(&sender_did), Error::<T>::DIDNotRegistered);

            Owners::insert(&node, &sender_did);
            Self::deposit_event(Event::NodeClaimed(node, sender_did));
        }

        /// A claim can be removed by its owner and get back the reservation. The additional
        /// connections are also removed. You can't remove a claim on well known nodes, as it
        /// needs to reach consensus among the network participants.
        ///
        /// - `node`: identifier of the node.
        #[weight = T::WeightInfo::remove_claim()]
        pub fn remove_claim(origin, node: PeerId) {
            let sender = ensure_signed(origin)?;
            let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(Owners::contains_key(&node), Error::<T>::NotClaimed);
            ensure!(Owners::get(&node) == sender_did, Error::<T>::NotOwner);
            ensure!(!WellKnownNodes::get().contains(&node), Error::<T>::PermissionDenied);

            Owners::remove(&node);
            AdditionalConnections::remove(&node);

            Self::deposit_event(Event::ClaimRemoved(node, sender_did));
        }

        /// A node can be transferred to a new owner.
        ///
        /// - `node`: identifier of the node.
        /// - `owner`: new owner of the node.
        #[weight = T::WeightInfo::transfer_node()]
        pub fn transfer_node(origin, node: PeerId, owner: Did) {
            let sender = ensure_signed(origin)?;
            let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(Owners::contains_key(&node), Error::<T>::NotClaimed);
            ensure!(Owners::get(&node) == sender_did, Error::<T>::NotOwner);
            ensure!(did::Module::<T>::did_registered(&owner), Error::<T>::DIDNotRegistered);

            Owners::insert(&node, &owner);

            Self::deposit_event(Event::NodeTransferred(node, owner));
        }

        /// Add additional connections to a given node.
        ///
        /// - `node`: identifier of the node.
        /// - `connections`: additonal nodes from which the connections are allowed.
        #[weight = T::WeightInfo::add_connections()]
        pub fn add_connections(
            origin,
            node: PeerId,
            connections: Vec<PeerId>
        ) {
            let sender = ensure_signed(origin)?;
            let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(Owners::contains_key(&node), Error::<T>::NotClaimed);
            ensure!(Owners::get(&node) == sender_did, Error::<T>::NotOwner);

            let mut nodes = AdditionalConnections::get(&node);

            for add_node in connections.iter() {
                if *add_node == node {
                    continue;
                }
                nodes.insert(add_node.clone());
            }

            AdditionalConnections::insert(&node, nodes);

            Self::deposit_event(Event::ConnectionsAdded(node, connections));
        }

        /// Remove additional connections of a given node.
        ///
        /// - `node`: identifier of the node.
        /// - `connections`: additonal nodes from which the connections are not allowed anymore.
        #[weight = T::WeightInfo::remove_connections()]
        pub fn remove_connections(
            origin,
            node: PeerId,
            connections: Vec<PeerId>
        ) {
            let sender = ensure_signed(origin)?;
            let sender_did = did::Module::<T>::get_did_from_account_id(&sender);
            ensure!(node.0.len() < T::MaxPeerIdLength::get() as usize, Error::<T>::PeerIdTooLong);
            ensure!(Owners::contains_key(&node), Error::<T>::NotClaimed);
            ensure!(Owners::get(&node) == sender_did, Error::<T>::NotOwner);

            let mut nodes = AdditionalConnections::get(&node);

            for remove_node in connections.iter() {
                nodes.remove(remove_node);
            }

            AdditionalConnections::insert(&node, nodes);

            Self::deposit_event(Event::ConnectionsRemoved(node, connections));
        }

        /// Set reserved node every block. It may not be enabled depends on the offchain
        /// worker settings when starting the node.
        fn offchain_worker(now: T::BlockNumber) {
            let network_state = sp_io::offchain::network_state();
            match network_state {
                Err(_) => debug::error!("Error: failed to get network state of node at {:?}", now),
                Ok(state) => {
                    let encoded_peer = state.peer_id.0;
                    match Decode::decode(&mut &encoded_peer[..]) {
                        Err(_) => debug::error!("Error: failed to decode PeerId at {:?}", now),
                        Ok(node) => sp_io::offchain::set_authorized_nodes(
                            Self::get_authorized_nodes(&PeerId(node)),
                            true
                        )
                    }
                }
            }
        }
    }
}

impl<T: Config> Module<T> {
    fn initialize_nodes(nodes: &Vec<(PeerId, Did)>) {
        let peer_ids = nodes
            .iter()
            .map(|item| item.0.clone())
            .collect::<BTreeSet<PeerId>>();
        WellKnownNodes::put(&peer_ids);
        //// Not checking if given Did(s) are registered or not in initialization
        for (node, who) in nodes.iter() {
            Owners::insert(node, who);
        }
    }

    fn get_authorized_nodes(node: &PeerId) -> Vec<PeerId> {
        let mut nodes = AdditionalConnections::get(node);

        let mut well_known_nodes = WellKnownNodes::get();
        if well_known_nodes.contains(node) {
            well_known_nodes.remove(node);
            nodes.extend(well_known_nodes);
        }

        Vec::from_iter(nodes)
    }
}
