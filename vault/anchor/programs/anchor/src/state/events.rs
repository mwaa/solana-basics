use anchor_lang::prelude::*;

#[event]
pub struct InitializeEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
}

#[event]
pub struct DepositEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct WithdrawEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CloseEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
}
