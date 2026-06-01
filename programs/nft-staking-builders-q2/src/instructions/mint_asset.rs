use anchor_lang::prelude::*;
use mpl_core::accounts::BaseCollectionV1;

use crate::errors::StakingError;

#[derive(Accounts)]
pub struct MintAsset<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub asset: Signer<'info>,
    #[account(mut)]
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

impl<'info> MintAsset<'info> {
    pub fn mint_asset(
        &mut self,
        asset_name: String,
        asset_uri: String,
        bumps: &MintAssetBumps,
    ) -> Result<()> {
        let signers_seeds: &[&[&[u8]]] = &[&[
            b"update_authority",
            self.update_authority.key.as_ref(),
            &[bumps.update_authority],
        ]];

        mpl_core::instructions::CreateV2CpiBuilder::new(&self.mpl_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.payer.to_account_info()))
            .payer(&self.payer.to_account_info())
            .owner(Some(&self.payer.to_account_info()))
            .update_authority(None)
            .system_program(&self.system_program.to_account_info())
            .name(asset_name)
            .uri(asset_uri)
            .invoke_signed(signers_seeds)?;

        Ok(())
    }
}