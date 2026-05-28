use anchor_lang::prelude::*;

mod instructions;
mod state;
mod errors;

use instructions::*;

declare_id!("EZ3oBGh31197iEowQrs7KabmnR9XogR8ZoHZMUK7Vq8J");

#[program]
pub mod nft_staking_builders_q2 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, reward_rate_per_day: u64, freeze_period: u16) -> Result<()> {
        ctx.accounts.initialize(reward_rate_per_day, freeze_period, &ctx.bumps)
    }

    pub fn create_collection(ctx: Context<CreateCollection>, collection_name: String, collection_uri: String) -> Result<()> {
        ctx.accounts.create_collection(collection_name, collection_uri, &ctx.bumps)
    }

    pub fn mint_asset(ctx: Context<MintAsset>, asset_name: String, asset_uri: String) -> Result<()> {
        ctx.accounts.mint_asset(asset_name, asset_uri)
    }
}

