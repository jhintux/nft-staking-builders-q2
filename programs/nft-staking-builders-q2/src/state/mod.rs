use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Config {
    /// Whole tokens per second in base units (6 decimals). Must be a multiple of 1_000_000 (e.g. 1_000_000 = 1 token/sec).
    pub reward_rate_per_sec: u64,
    pub freeze_period: u16,
    pub rewards_bump: u8,
    pub config_bump: u8,
}