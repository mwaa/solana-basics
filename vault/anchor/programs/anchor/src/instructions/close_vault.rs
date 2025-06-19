use anchor_lang::prelude::*;
use anchor_spl::token::{close_account, transfer_checked, CloseAccount, Mint, Token, TokenAccount, TransferChecked};
use crate::events::CloseEvent;
use crate::state::VaultState;
use crate::{VAULT_ACCOUNT_SEED, VAULT_SEED};

#[derive(Accounts)]
pub struct CloseVault<'info> {
    #[account(mut)]
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
    )]
    pub vault_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Anchor does not require explicit instruction for close,
pub fn handle_close_vault(ctx: Context<CloseVault>) -> Result<()> {
    
    let user_key = ctx.accounts.user.key();
    let mint_key = ctx.accounts.mint.key();
    let vault_state_bump = ctx.accounts.vault_state.load()?.bump;
    let amount = ctx.accounts.vault_account.amount;

    let seeds = &[VAULT_SEED, user_key.as_ref(), mint_key.as_ref(), &[vault_state_bump]];
    let signer = &[&seeds[..]];
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.vault_account.to_account_info(),
        to: ctx.accounts.user_account.to_account_info(),
        authority: ctx.accounts.vault_state.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;


    // Close the token account using the Token Program
    let close_accounts = CloseAccount {
        account: ctx.accounts.vault_account.to_account_info(),
        destination: ctx.accounts.user.to_account_info(),
        authority: ctx.accounts.vault_state.to_account_info(),
    };
    let close_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), close_accounts, signer);
    close_account(close_ctx)?;

    emit!(CloseEvent {
        owner: ctx.accounts.user.key(),
        mint: ctx.accounts.vault_account.mint,
    });
    Ok(())
}
