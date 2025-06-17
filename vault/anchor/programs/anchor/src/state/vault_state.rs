use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(Default)]
pub struct VaultState {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub deposited: u64,
    pub bump: u8,
    pub bump_token_account: u8,
    pub _padding: [u8; 6], // Padding to ensure the size is 64 bytes
}
