use crate::errors::StakingError;
use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, UpdateAuthority},
};

#[derive(Accounts)]
pub struct Stake<'info> {
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
    /// CHECK: This is the update authority for the collection, acc doesnt exist
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
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn stake(&mut self, bumps: &StakeBumps) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let now_str = now.to_string();

        let signers_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            self.update_authority.key.as_ref(),
            &[bumps.update_authority],
        ]];
        
        match fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            mpl_core::types::PluginType::Attributes,
        ) {
            Ok((_, fetched_att_list, _)) => {
                let mut attribute_list = Vec::new();
                let mut is_initialized = false;

                for attribute in fetched_att_list.attribute_list {
                    if attribute.key == "staked" {
                        require!(attribute.value == "0", StakingError::AlreadyStaked);
                        attribute_list.push(Attribute {
                            key: "staked".to_string(),
                            value: now_str.clone(),
                        });
                        is_initialized = true;
                    } else if attribute.key != "last_claimed_at" {
                        attribute_list.push(attribute);
                    }
                }

                if !is_initialized {
                    attribute_list.push(Attribute {
                        key: "staked".to_string(),
                        value: now_str.clone(),
                    });
                    attribute_list.push(Attribute {
                        key: "staked_time".to_string(),
                        value: 0.to_string(),
                    });
                }

                attribute_list.push(Attribute {
                    key: "last_claimed_at".to_string(),
                    value: now_str.clone(),
                });

                UpdatePluginV1CpiBuilder::new(&self.mpl_program.to_account_info())
                    .asset(&self.asset.to_account_info())
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.owner.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes { attribute_list }))
                    .invoke_signed(signers_seeds)?;
            }
            Err(_) => {
                AddPluginV1CpiBuilder::new(&self.mpl_program.to_account_info())
                    .asset(&self.asset.to_account_info())
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.owner.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes {
                        attribute_list: vec![
                            Attribute {
                                key: "staked".to_string(),
                                value: now_str.clone(),
                            },
                            Attribute {
                                key: "staked_time".to_string(),
                                value: 0.to_string(),
                            },
                            Attribute {
                                key: "last_claimed_at".to_string(),
                                value: now_str.clone(),
                            },
                        ],
                    }))
                    .init_authority(PluginAuthority::UpdateAuthority)
                    .invoke_signed(signers_seeds)?;
            }
        }

        AddPluginV1CpiBuilder::new(&self.mpl_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke_signed(signers_seeds)?;

        Ok(())
    }
}
