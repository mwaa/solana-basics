pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("8mkgZQT7izpwtkxuy7ModN6NmeQCGJrQ2TvXqL8LpfjD");

#[program]
pub mod anchor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::handle_initialize(ctx)
    }
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::handle_deposit(ctx, amount)
    }
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::handle_withdraw(ctx, amount)
    }
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::handle_close_vault(ctx)
    }
}
