use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::events::CloseEvent;
use crate::state::VaultState;
use crate::{VAULT_ACCOUNT_SEED, VAULT_SEED};

#[derive(Accounts)]
pub struct CloseVault<'info> {
    pub user: Signer<'info>,

    #[account(
        mut,
        token::mint = vault_state.load()?.mint,
        token::authority = user,
    )]
    pub user_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref(), mint.key().as_ref()],
        bump = vault_state.load()?.bump,
        has_one = user,
        close = user,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_state.key().as_ref()],
        bump = vault_state.load()?.bump_token_account,
        close = user,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Anchor does not require explicit instruction for close,
pub fn handler(ctx: Context<CloseVault>) -> Result<()> {
    
    let vault_state_key = ctx.accounts.vault_state.key();
    let vault_account_bump = ctx.accounts.vault_state.load()?.bump_token_account;
    let amount = ctx.accounts.vault_account.amount;

    let seeds = &[VAULT_ACCOUNT_SEED, vault_state_key.as_ref(), &[vault_account_bump]];
    let signer = &[&seeds[..]];
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.vault_account.to_account_info(),
        to: ctx.accounts.user_account.to_account_info(),
        authority: ctx.accounts.vault_state.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    emit!(CloseEvent {
        owner: ctx.accounts.user.key(),
        mint: ctx.accounts.vault_account.mint,
    });
    Ok(())
}
