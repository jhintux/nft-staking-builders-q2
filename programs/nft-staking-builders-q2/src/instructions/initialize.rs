use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
//use mpl_core::accounts::BaseCollectionV1;

use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [b"config", collection.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = payer,
        mint::authority = config.key(),
        mint::decimals = 6,
        seeds = [b"rewards_mint", collection.key().as_ref()],
        bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    /// CHECK : BaseCollectionV1
    pub collection: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        reward_rate_per_day: u64,
        freeze_period: u16,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        self.config.set_inner(Config {
            reward_rate_per_day,
            freeze_period,
            rewards_bump: bumps.rewards_mint,
            config_bump: bumps.config,
        });

        

        Ok(())
    }
}
