use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Incorrect MPL program")]
    IncorrectMplProgram
}