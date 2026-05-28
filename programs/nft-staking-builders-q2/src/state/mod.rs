use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Config {
    pub reward_rate_per_day: u64,
    pub freeze_period: u16,
    pub rewards_bump: u8,
    pub config_bump: u8,
}