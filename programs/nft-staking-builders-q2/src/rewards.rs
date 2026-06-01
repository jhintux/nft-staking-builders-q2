use crate::errors::StakingError;
use anchor_lang::prelude::*;

/// Smallest mint unit for the rewards token (6 decimals). A rate of `1_000_000` pays 1 whole token per second.
pub const REWARD_TOKEN_UNIT: u64 = 1_000_000;

pub fn require_whole_token_rate(reward_rate_per_sec: u64) -> Result<()> {
    require!(
        reward_rate_per_sec.is_multiple_of(REWARD_TOKEN_UNIT),
        StakingError::InvalidRewardRate
    );
    Ok(())
}

pub fn resolve_claim_from(staked_at: i64, last_claimed_at: Option<i64>) -> Result<i64> {
    let mut claim_from = last_claimed_at.unwrap_or(staked_at);
    if claim_from == 0 {
        claim_from = staked_at;
    }
    require!(claim_from >= staked_at, StakingError::InvalidTimestamp);
    Ok(claim_from)
}

pub fn require_freeze_period_elapsed(staked_at: i64, freeze_period: u16, now: i64) -> Result<()> {
    let unlock_at = staked_at
        .checked_add(freeze_period as i64)
        .ok_or(StakingError::Overflow)?;
    require!(now >= unlock_at, StakingError::FreezePeriodNotElapsed);
    Ok(())
}

pub fn compute_reward_amount(
    now: i64,
    claim_from: i64,
    reward_rate_per_sec: u64,
) -> Result<u64> {
    require!(claim_from <= now, StakingError::InvalidTimestamp);
    let elapsed = now
        .checked_sub(claim_from)
        .ok_or(StakingError::Underflow)?;
    require!(elapsed > 0, StakingError::NoRewardsToClaim);
    amount_from_elapsed(elapsed, reward_rate_per_sec)
}

/// Same as [`compute_reward_amount`] but returns `0` when nothing has accrued (used on unstake).
pub fn compute_reward_amount_if_any(
    now: i64,
    claim_from: i64,
    reward_rate_per_sec: u64,
) -> Result<u64> {
    require!(claim_from <= now, StakingError::InvalidTimestamp);
    let elapsed = now
        .checked_sub(claim_from)
        .ok_or(StakingError::Underflow)?;
    if elapsed <= 0 {
        return Ok(0);
    }
    amount_from_elapsed(elapsed, reward_rate_per_sec)
}

fn amount_from_elapsed(elapsed: i64, reward_rate_per_sec: u64) -> Result<u64> {
    let amount = (elapsed as u128)
        .checked_mul(reward_rate_per_sec as u128)
        .ok_or(StakingError::Overflow)?;
    amount.try_into().map_err(|_| StakingError::Overflow.into())
}
