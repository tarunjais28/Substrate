use metablockchain_runtime::{
    did::DidStruct, AccountId, AuraConfig, BalancesConfig, CouncilConfig, DidConfig, GenesisConfig,
    GrandpaConfig, NodeAuthorizationConfig, Signature, SudoConfig, SystemConfig,
    ValidatorSetConfig, WASM_BINARY,
};
use sc_service::{ChainType, Properties};
use serde_json::map::Map;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::crypto::Ss58Codec;
use sp_core::OpaquePeerId; // A struct wraps Vec<u8>, represents as our `PeerId`.
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Create a public key from an SS58 address
fn pubkey_from_ss58<T: Public>(ss58: &str) -> T {
    Ss58Codec::from_string(ss58).unwrap()
}

/// Create an account id from a SS58 address
fn account_id_from_ss58<T: Public>(ss58: &str) -> AccountId
where
    AccountPublic: From<T>,
{
    AccountPublic::from(pubkey_from_ss58::<T>(ss58)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

// specify chain properties
fn get_common_properties_map() -> Properties {
    let mut properties = Map::new();
    properties.insert("tokenSymbol".into(), "MUI".into());
    properties.insert("tokenDecimals".into(), 6.into());
    properties
}

pub fn development_config() -> Result<ChainSpec, String> {
    //let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Metablockchain-Development",
        // ID
        "dev",
        ChainType::Development,
        || {
            GenesisBuilder {
                // Initial PoA authorities
                initial_authorities: vec![authority_keys_from_seed("Alice")],
                // Sudo account
                root_key: get_account_id_from_seed::<sr25519::Public>("Alice"),
                // Pre-funded accounts
                endowed_accounts: vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    // get_account_id_from_seed::<sr25519::Public>("Bob"),
                    // get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    // get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                ],
                initial_validators: vec![*b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"],
                initial_dids: vec![DidStruct {
                    identifier: *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: get_from_seed::<sr25519::Public>("Alice"),
                    metadata: vec![],
                }],
                initial_nodes: vec![], // development chain does not need nodes
                initial_collective_members: vec![],
                _enable_println: true,
            }
            .build()
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(get_common_properties_map()),
        // Extensions
        None,
    ))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    //let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Metablockchain Testnet",
        // ID
        "testnet",
        ChainType::Local,
        || {
            GenesisBuilder {
                // Initial PoA authorities
                initial_authorities: vec![authority_keys_from_seed("Alice")],
                // Sudo account
                root_key: account_id_from_ss58::<sr25519::Public>(
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                ),
                // Pre-funded accounts
                endowed_accounts: vec![account_id_from_ss58::<sr25519::Public>(
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                )],
                initial_validators: vec![*b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"],
                initial_dids: vec![DidStruct {
                    identifier: *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: pubkey_from_ss58::<sr25519::Public>(
                        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                    ),
                    metadata: vec![],
                }],
                initial_nodes: vec![
                    (
                        OpaquePeerId(
                            bs58::decode("12D3KooWBmAwcd4PJNJvfV89HwE48nwkRmAgo8Vy3uQEyNNHBox2")
                                .into_vec()
                                .unwrap(),
                        ),
                        *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    ),
                    (
                        OpaquePeerId(
                            bs58::decode("12D3KooWQYV9dGMFoRzNStwpXztXaBUjtPqi6aU76ZgUriHhKust")
                                .into_vec()
                                .unwrap(),
                        ),
                        *b"did:ssid:swn2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    ),
                ],
                initial_collective_members: vec![],
                _enable_println: true,
            }
            .build()
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(get_common_properties_map()),
        // Extensions
        None,
    ))
}

pub fn mainnet_config() -> Result<ChainSpec, String> {
    //let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Metablockchain Mainnet",
        // ID
        "mainnet",
        ChainType::Live,
        || {
            GenesisBuilder {
                // Initial PoA authorities
                initial_authorities: vec![authority_keys_from_seed("Alice")],
                // Sudo account
                root_key: account_id_from_ss58::<sr25519::Public>(
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                ),
                // Pre-funded accounts
                endowed_accounts: vec![account_id_from_ss58::<sr25519::Public>(
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                )],
                initial_validators: vec![*b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"],
                initial_dids: vec![DidStruct {
                    identifier: *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    public_key: pubkey_from_ss58::<sr25519::Public>(
                        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                    ),
                    metadata: vec![],
                }],
                initial_nodes: vec![
                    (
                        OpaquePeerId(
                            bs58::decode("12D3KooWBmAwcd4PJNJvfV89HwE48nwkRmAgo8Vy3uQEyNNHBox2")
                                .into_vec()
                                .unwrap(),
                        ),
                        *b"did:ssid:swn\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    ),
                    (
                        OpaquePeerId(
                            bs58::decode("12D3KooWQYV9dGMFoRzNStwpXztXaBUjtPqi6aU76ZgUriHhKust")
                                .into_vec()
                                .unwrap(),
                        ),
                        *b"did:ssid:swn2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                    ),
                ],
                initial_collective_members: vec![],
                _enable_println: true,
            }
            .build()
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(get_common_properties_map()),
        // Extensions
        None,
    ))
}

// Genesis Builder for metablockchain runtime
struct GenesisBuilder {
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    initial_validators: Vec<[u8; 32]>,
    initial_dids: Vec<DidStruct>,
    initial_nodes: Vec<(OpaquePeerId, [u8; 32])>,
    initial_collective_members: Vec<[u8; 32]>,
    _enable_println: bool,
}

impl GenesisBuilder {
    fn build(self) -> GenesisConfig {
        GenesisConfig {
            frame_system: Some(SystemConfig {
                // Add Wasm runtime to storage.
                code: WASM_BINARY.unwrap().to_vec(),
                changes_trie_config: Default::default(),
            }),
            pallet_aura: Some(AuraConfig {
                authorities: self
                    .initial_authorities
                    .iter()
                    .map(|x| (x.0.clone()))
                    .collect(),
            }),
            pallet_grandpa: Some(GrandpaConfig {
                authorities: self
                    .initial_authorities
                    .iter()
                    .map(|x| (x.1.clone(), 1))
                    .collect(),
            }),
            pallet_sudo: Some(SudoConfig {
                // Assign network admin rights.
                key: self.root_key,
            }),
            node_authorization: Some(NodeAuthorizationConfig {
                nodes: self.initial_nodes,
            }),
            validator_set: Some(ValidatorSetConfig {
                validators: self.initial_validators,
            }),
            did: Some(DidConfig {
                dids: self.initial_dids,
            }),
            balances: Some(BalancesConfig {
                // Configure endowed accounts with initial balance of 1Billion MUI
                balances: self
                    .endowed_accounts
                    .iter()
                    .cloned()
                    .map(|k| (k, 1000000000000000))
                    .collect(),
            }),
            collective: Some(CouncilConfig {
                members: self.initial_collective_members,
                // phantom: Default::default(),
            }),
        }
    }
}
