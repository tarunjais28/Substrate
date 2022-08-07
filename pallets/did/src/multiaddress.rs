use codec::{Decode, Encode};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use sp_std::{self};

// Use this struct for the account lookup
// This struct can have the value of either rawbytes or accountid
// This is necessary to compile all other pallets that depend on the accountID field
// Once all pallets have been ported to the custom DID format we can remove the dependency
// on this struct and lookup trait in general
#[derive(Encode, Decode, PartialEq, Eq, Clone, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum MultiAddress<AccountId> {
    //type for regular pubkey accountid
    Id(AccountId),
    //type for lookup to the did identifier - referencing the did type from the did module
    Did([u8; 32]),
}

#[cfg(feature = "std")]
impl<AccountId> std::fmt::Display for MultiAddress<AccountId>
where
    AccountId: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MultiAddress::Did(inner) => write!(f, "{}", HexDisplay::from(inner)),
            MultiAddress::Id(_inner) => write!(f, "{}", self),
        }
    }
}

// Create a MultiAddress object from an accountid passed
impl<AccountId> From<AccountId> for MultiAddress<AccountId> {
    fn from(x: AccountId) -> Self {
        MultiAddress::Id(x)
    }
}

// The default option to select when creating a Multiaddress
// The current default is set to accountid, but once we migrate all pallets
// to use did signing, we can move default to did
impl<AccountId: Default> Default for MultiAddress<AccountId> {
    fn default() -> Self {
        MultiAddress::Id(Default::default())
    }
}
