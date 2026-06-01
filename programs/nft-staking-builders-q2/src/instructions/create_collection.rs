use anchor_lang::prelude::*;
use mpl_core::types::{Attribute, Attributes, Plugin, PluginAuthority, PluginAuthorityPair};

use crate::errors::StakingError;

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub collection: Signer<'info>,
    /// CHECK: This is the update authority for the collection, acc doesnt exist
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: The metaplex Core program
    #[account(
        address = mpl_core::ID @ StakingError::IncorrectMplProgram
    )]
    pub mpl_program: UncheckedAccount<'info>,
}

impl<'info> CreateCollection<'info> {
    pub fn create_collection(
        &mut self,
        collection_name: String,
        collection_uri: String,
        bumps: &CreateCollectionBumps,
    ) -> Result<()> {
        let mut attributes = Vec::new();
        attributes.push(Attribute {
            key: "staked_nfts".to_string(),
            value: "0".to_string(),
        });

        let collection_plugin = Plugin::Attributes(Attributes {
            attribute_list: attributes,
        });

        let signers_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            self.update_authority.key.as_ref(),
            &[bumps.update_authority],
        ]];

        mpl_core::instructions::CreateCollectionV2CpiBuilder::new(&self.mpl_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.payer.to_account_info())
            .system_program(&self.system_program.to_account_info())
            .update_authority(Some(&self.update_authority.to_account_info()))
            .name(collection_name)
            .uri(collection_uri)
            .plugins(vec![PluginAuthorityPair {
                plugin: collection_plugin,
                authority: Some(PluginAuthority::UpdateAuthority),
            }])
            .invoke_signed(signers_seeds)?;

        Ok(())
    }
}
