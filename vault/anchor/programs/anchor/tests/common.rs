use mollusk_svm::Mollusk;
use solana_sdk::{account::{AccountSharedData, WritableAccount}, program_option::COption, program_pack::Pack, pubkey::Pubkey};
use spl_token::{
    state::{Mint, AccountState as MintAccountState, Account as MintAccount},
};

// Uses pack to represent an intialized mint account
fn get_mint_account(mint_authority: &Pubkey, supply: u64) -> AccountSharedData {
    let mut account = AccountSharedData::new(0, Mint::LEN, &spl_token::id());
    Mint {
        mint_authority: COption::Some(*mint_authority),
        supply,
        decimals: 9,
        is_initialized: true,
        freeze_authority: COption::None,
    }
    .pack_into_slice(account.data_as_mut_slice());
    account
}

// Uses pack to represent associated token address for provided owner and mint with a given amount balance
pub fn get_token_account(owner: &Pubkey, mint: &Pubkey, amount: u64) -> AccountSharedData {
    let mut account = AccountSharedData::new(0, MintAccount::LEN, &mollusk_svm_programs_token::token::ID);
    MintAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: MintAccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    }
    .pack_into_slice(account.data_as_mut_slice());
    account
}


pub fn init_mollusk() -> (Mollusk, Pubkey, Pubkey, Pubkey, AccountSharedData, AccountSharedData) {
    // Copied from lib.rs
    let program_id = Pubkey::from_str_const("8mkgZQT7izpwtkxuy7ModN6NmeQCGJrQ2TvXqL8LpfjD");

    let mut mollusk = Mollusk::new(&program_id, "../../target/deploy/anchor");

    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

    let user = Pubkey::new_unique();

    let token_mint = Pubkey::new_unique();

    let token_mint_account = get_mint_account(&user, 5_000_000_000);

    let user_token_account = get_token_account(&user, &token_mint, 10_000_000);

    (mollusk, program_id, user, token_mint, token_mint_account, user_token_account)
}