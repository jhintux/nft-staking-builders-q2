use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct MintAsset<'info> {
    pub system_program: Program<'info, System>,
}

impl<'info> MintAsset<'info> {
    pub fn mint_asset(
        &mut self,
        asset_name: String,
        asset_uri: String,
    ) -> Result<()> {
        Ok(())
    }
}