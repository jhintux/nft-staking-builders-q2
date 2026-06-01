use crate::errors::StakingError;
use crate::rewards;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, UpdateAuthority},
};

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        has_one = owner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ StakingError::InvalidUpdateAuthority
    )]
    pub asset: Account<'info, BaseAssetV1>,
    #[account(
        mut,
        has_one = update_authority
    )]
    pub collection: Account<'info, BaseCollectionV1>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [b"rewards_mint", collection.key().as_ref()],
        bump = config.rewards_bump,
        mint::authority = config,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub user_rewards_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: PDA update authority for the collection
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,
    /// CHECK: The metaplex Core program
    #[account(
        address = mpl_core::ID @ StakingError::IncorrectMplProgram
    )]
    pub mpl_program: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimRewards<'info> {
    pub fn claim_rewards(&mut self, bumps: &ClaimRewardsBumps) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        let (_, fetched_att_list, _) = fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            mpl_core::types::PluginType::Attributes,
        )
        .map_err(|_| StakingError::AttributesPluginNotFound)?;

        let mut staked_at: Option<i64> = None;
        let mut last_claimed_at: Option<i64> = None;
        let mut attribute_list = Vec::new();

        for attribute in fetched_att_list.attribute_list {
            match attribute.key.as_str() {
                "staked" => {
                    let ts = attribute
                        .value
                        .parse::<i64>()
                        .map_err(|_| StakingError::InvalidTimestamp)?;
                    require!(ts != 0, StakingError::NotStaked);
                    staked_at = Some(ts);
                    attribute_list.push(attribute);
                }
                "last_claimed_at" => {
                    let ts = attribute
                        .value
                        .parse::<i64>()
                        .map_err(|_| StakingError::InvalidTimestamp)?;
                    last_claimed_at = Some(ts);
                }
                _ => attribute_list.push(attribute),
            }
        }

        let staked_at = staked_at.ok_or(StakingError::StakingNotInitialized)?;
        rewards::require_freeze_period_elapsed(staked_at, self.config.freeze_period, now)?;

        let claim_from = rewards::resolve_claim_from(staked_at, last_claimed_at)?;
        let amount = rewards::compute_reward_amount(now, claim_from, self.config.reward_rate_per_sec)?;

        mint_rewards(
            &self.config,
            &self.collection,
            &self.rewards_mint,
            &self.user_rewards_token_account,
            &self.token_program,
            amount,
        )?;

        attribute_list.push(Attribute {
            key: "last_claimed_at".to_string(),
            value: now.to_string(),
        });

        let signers_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            self.update_authority.key.as_ref(),
            &[bumps.update_authority],
        ]];

        UpdatePluginV1CpiBuilder::new(&self.mpl_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes { attribute_list }))
            .invoke_signed(signers_seeds)?;

        Ok(())
    }
}

pub(crate) fn mint_rewards<'info>(
    config: &Account<'info, Config>,
    collection: &Account<'info, BaseCollectionV1>,
    rewards_mint: &InterfaceAccount<'info, Mint>,
    user_rewards_token_account: &InterfaceAccount<'info, TokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    amount: u64,
) -> Result<()> {
    let collection_key = collection.key();
    let config_seeds: &[&[u8]] = &[
        b"config",
        collection_key.as_ref(),
        &[config.config_bump],
    ];
    token_interface::mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: rewards_mint.to_account_info(),
                to: user_rewards_token_account.to_account_info(),
                authority: config.to_account_info(),
            },
            &[config_seeds],
        ),
        amount,
    )
}
