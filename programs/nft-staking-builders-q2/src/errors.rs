use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Incorrect MPL program")]
    IncorrectMplProgram,

    #[msg("Already staked")]
    AlreadyStaked,
    #[msg("Staking not initialized")]
    StakingNotInitialized,
    #[msg("Not staked")]
    NotStaked,

    #[msg("Attributes plugin not found")]
    AttributesPluginNotFound,
    #[msg("Invalid update authority")]
    InvalidUpdateAuthority,

    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Underflow")]
    Underflow,
    #[msg("Overflow")]
    Overflow,
    #[msg("No rewards to claim")]
    NoRewardsToClaim,
    #[msg("Reward rate must be a whole number of tokens per second (multiple of 1_000_000)")]
    InvalidRewardRate,
    #[msg("Freeze period has not elapsed since stake")]
    FreezePeriodNotElapsed,
}