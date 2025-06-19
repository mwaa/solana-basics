use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::events::InitializeEvent;
use crate::state::VaultState;
use crate::error::ErrorCode;
use crate::{VAULT_ACCOUNT_SEED, VAULT_SEED};

#[derive(Accounts)]
#[instruction()]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        seeds = [VAULT_SEED, user.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = user,
        space = 8 + std::mem::size_of::<VaultState>(),
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    #[account(
        init,
        seeds = [VAULT_ACCOUNT_SEED, vault_state.key().as_ref()],
        bump,
        payer = user,
        token::mint = mint,
        token::authority = vault_state,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    let vault_state = &mut ctx.accounts.vault_state.load_init()?;
    let mint = &ctx.accounts.mint;

    // Ensure the mint is initialized and has a non-zero supply
    require!(mint.supply > 0, ErrorCode::InvalidMint);
    
    vault_state.user = ctx.accounts.user.key();
    vault_state.mint = mint.key();
    vault_state.bump = ctx.bumps.vault_state;
    vault_state.bump_token_account = ctx.bumps.vault_account;
    vault_state.deposited = 0;

    emit!(InitializeEvent {
        owner: ctx.accounts.user.key(),
        mint: mint.key(),
    });
    Ok(())
}
