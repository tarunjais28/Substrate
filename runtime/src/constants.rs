// Currency
pub mod currency {
    pub type Balance = u128;
    pub const MUI: Balance = 1_000_000;
    pub const FRANNIE: Balance = MUI / 1_000_000; // Smallest unit of MUI

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 20 * MUI + (bytes as Balance) * 100 * FRANNIE
    }
}

// Time and blocks.
pub mod time {
    // use primitives::v0::{Moment, BlockNumber};
    pub type BlockNumber = u32;
    pub type Moment = u32;
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    // 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}
