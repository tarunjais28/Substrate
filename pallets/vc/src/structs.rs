use super::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct VC<Hash> {
    pub hash: Hash,
    pub owner: Did,
    pub issuers: Vec<Did>,
    pub signatures: Vec<Signature>,
    pub is_vc_used: bool,
    pub vc_type: VCType,
    pub vc_property: [u8; 128],
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VCType {
    TokenVC,
    SlashTokens,
    MintTokens,
    TokenTransferVC,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenVC {
    pub token_name: [u8; 16],
    pub reservable_balance: u128,
    pub decimal: u8,
    pub currency_code: [u8; 8],
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlashMintTokens {
    pub vc_id: VCid,
    pub currency_code: [u8; 8],
    pub amount: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenTransferVC {
    pub vc_id: VCid,
    pub currency_code: [u8; 8],
    pub amount: u128,
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VCStatus {
    Active,
    Inactive,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitialVCs {
    pub identifier: Did,
    pub public_key: PublicKey,
    pub vcs: Vec<VCHash>,
}

pub trait HasVCId {
    fn vc_id(&self) -> VCid;
}

impl HasVCId for SlashMintTokens {
    fn vc_id(&self) -> VCid {
        self.vc_id
    }
}

impl HasVCId for TokenTransferVC {
    fn vc_id(&self) -> VCid {
        self.vc_id
    }
}

/// Utility type for managing upgrades/migrations.
#[derive(codec::Encode, codec::Decode, Clone, frame_support::RuntimeDebug, PartialEq)]
pub enum VCPalletVersion {
	V1_0_0,
	V2_0_0,
}
