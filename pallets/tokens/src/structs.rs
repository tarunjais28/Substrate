use super::*;

/// An index to a block.
pub type BlockNumber = u32;

/// A single lock on a balance. There can be many of these on an account and
/// they "overlap", so the same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance> {
    /// An identifier for this lock. Only one lock may be in existence for each
    /// identifier.
    pub id: LockIdentifier,
    /// The amount which the free balance may not drop below when this lock is
    /// in effect.
    pub amount: Balance,
}

/// Information of an account.
#[derive(Clone, Eq, PartialEq, Default, RuntimeDebug, Encode, Decode)]
pub struct TokenAccountInfo<Index, TokenAccountData> {
    /// The number of transactions this account has sent.
    pub nonce: Index,
    /// The additional data that belongs to this account. Used to store the balance(s) in a lot of
    /// chains.
    pub data: TokenAccountData,
}

/// balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct TokenAccountData {
    /// Non-reserved part of the balance. There may still be restrictions on
    /// this, but it is the total pool what may in principle be transferred,
    /// reserved.
    ///
    /// This is the only balance that matters in terms of most operations on
    /// tokens.
    pub free: TokenBalance,
    /// Balance which is reserved and may not be used at all.
    ///
    /// This can still get slashed, but gets slashed last of all.
    ///
    /// This balance is a 'reserve' balance that other subsystems use in order
    /// to set aside tokens that are still 'owned' by the account holder, but
    /// which are suspendable.
    pub reserved: TokenBalance,
    /// The amount that `free` may not drop below when withdrawing.
    pub frozen: TokenBalance,
}

impl TokenAccountData {
    /// The amount that this account's free balance may not be reduced beyond.
    pub fn frozen(&self) -> TokenBalance {
        self.frozen
    }
    /// The total balance in this account including any that is reserved and
    /// ignoring any frozen.
    pub fn total(&self) -> TokenBalance {
        self.free.saturating_add(self.reserved)
    }
}

/// currency information.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct TokenDetails {
    pub token_name: Vec<u8>,
    pub currency_code: Vec<u8>,
    pub decimal: u8,
    pub block_number: BlockNumber,
}

/// Utility type for managing upgrades/migrations.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq)]
pub enum StorageVersion {
    V1_0_0,
    V2_0_0,
    V3_0_0,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct V1TokenDetails {
    pub token_name: Vec<u8>,
    pub currency_code: Vec<u8>,
    pub decimal: u8,
}
