use anchor::VaultState;
use anchor_spl::associated_token::spl_associated_token_account;
use mollusk_svm::{program, result::Check, Mollusk};
use mollusk_svm_programs_token::token::keyed_account as keyed_account_for_token_program;
use solana_sdk::{
    account::{Account, AccountSharedData, WritableAccount},
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent,
};
use spl_token::state::{Account as TokenAccount, AccountState, Mint};
use anchor_lang::InstructionData;

const PROGRAM_ID: Pubkey = Pubkey::from_str_const("8mkgZQT7izpwtkxuy7ModN6NmeQCGJrQ2TvXqL8LpfjD");
const VAULT_SEED: &[u8] = b"vault";
const VAULT_ACCOUNT_SEED: &[u8] = b"vault_account";

fn get_mint_account(mint_authority: &Pubkey, supply: u64) -> AccountSharedData {
    let mut account = AccountSharedData::new(0, Mint::LEN, &spl_token::id());
    Mint {
        mint_authority: Some(*mint_authority).into(),
        supply,
        decimals: 9,
        is_initialized: true,
        freeze_authority: None.into(),
    }
    .pack_into_slice(account.data_as_mut_slice());
    account
}

fn get_token_account(owner: &Pubkey, mint: &Pubkey, amount: u64) -> AccountSharedData {
    let mut account = AccountSharedData::new(0, TokenAccount::LEN, &spl_token::id());
    TokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: None.into(),
        state: AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    }
    .pack_into_slice(account.data_as_mut_slice());
    account
}

fn main() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, "../../target/deploy/anchor");
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

    let (system_program, system_account) = program::keyed_account_for_system_program();
    let (token_program, token_program_account) = keyed_account_for_token_program();

    let user = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let user_token_account = spl_associated_token_account::get_associated_token_address(&user, &token_mint);

    // Derive PDAs
    let (vault_state_pda, state_bump) = Pubkey::find_program_address(
        &[VAULT_SEED, user.as_ref(), token_mint.as_ref()],
        &PROGRAM_ID,
    );
    let (vault_account_pda, vault_account_bump) = Pubkey::find_program_address(
        &[VAULT_ACCOUNT_SEED, vault_state_pda.as_ref()],
        &PROGRAM_ID,
    );

    // --- Benchmark 1: Initialize ---
    let user_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);
    let mut vault_state_account = Account::new(
        mollusk
            .sysvars
            .rent
            .minimum_balance(VaultState::SPACE), 
            VaultState::SPACE, 
            &PROGRAM_ID
        );
    let vault_token_account = Account::new(0, 0, &system_program);
    let mint_account = get_mint_account(&user, 1_000_000_000);
    let rent_account = solana_sdk::account::create_account_shared_data_for_test(&rent::Rent::default());

    let initialize_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(rent::id(), false),
    ];
    let initialize_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &(anchor::instruction::Initialize {}).data(),
        initialize_accounts,
    );
    let initialize_tx_accounts = vec![
        (user, user_account.clone()),
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, vault_token_account.clone()),
        (token_mint, mint_account.clone().into()),
        (system_program, system_account.clone()),
        (token_program, token_program_account.clone()),
        (rent::id(), rent_account.clone().into()),
    ];

    // --- Benchmark 2: Deposit ---
    let initialized_vault_state = anchor::state::VaultState {
        user,
        mint: token_mint,
        bump: state_bump as u8,
        bump_token_account: vault_account_bump as u8,
        deposited: 8_000_000,
        _padding: [0; 6],
    };
    
    // Get data allocated in state_account
    let state_data = vault_state_account.data_as_mut_slice();
    
    // Write the discriminator (8 bytes)
    let discriminator: [u8; 8] = anchor_lang::solana_program::hash::hash(b"account:VaultState").to_bytes()[..8].try_into().unwrap();
    state_data[..8].copy_from_slice(&discriminator);
    
    // Write the vault state data directly
    unsafe {
        let vault_state_ptr = state_data[8..].as_mut_ptr() as *mut anchor::state::VaultState;
        *vault_state_ptr = initialized_vault_state;
    }

    let initialized_vault_token = get_token_account(&vault_state_pda, &token_mint, 0);
    let user_token_account_with_balance = get_token_account(&user, &token_mint, 1_000_000);

    let deposit_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
    ];
    println!("Deposit accounts: {:?}", deposit_accounts);

    let deposit_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &(anchor::instruction::Deposit { amount: 500_000 }).data(),
        deposit_accounts,
    );
    let deposit_tx_accounts = vec![
        (user, user_account.clone()),
        (user_token_account, user_token_account_with_balance.clone().into()),
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, initialized_vault_token.clone().into()),
        (token_mint, mint_account.clone().into()),
        (token_program, token_program_account.clone().into()),
    ];

    // --- Benchmark 3: Withdraw ---
    let vault_with_funds = get_token_account(&vault_state_pda, &token_mint, 500_000);
    let user_token_account_after_deposit = get_token_account(&user, &token_mint, 500_000);
    let withdraw_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
    ];
    let withdraw_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &(anchor::instruction::Withdraw { amount: 250_000 }).data(),
        withdraw_accounts,
    );
    let withdraw_tx_accounts = vec![
        (user, user_account.clone()),
        (user_token_account, user_token_account_after_deposit.clone().into()),
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, vault_with_funds.clone().into()),
        (token_mint, mint_account.clone().into()),
        (token_program, token_program_account.clone().into()),
    ];

    // --- Benchmark 4: CloseVault ---
    let vault_with_remaining = get_token_account(&vault_state_pda, &token_mint, 250_000);
    let user_token_account_after_withdraw = get_token_account(&user, &token_mint, 750_000);
    let close_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];
    let close_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &anchor::instruction::CloseVault {}.data(),
        close_accounts,
    );
    let close_tx_accounts = vec![
        (user, user_account),
        (user_token_account, user_token_account_after_withdraw.clone().into()),
        (vault_state_pda, vault_state_account.clone().into()),
        (vault_account_pda, vault_with_remaining.clone().into()),
        (token_mint, mint_account.clone().into()),
        (token_program, token_program_account.clone().into()),
        (system_program, system_account.clone().into()),
    ];

    
    let checks = &vec![Check::success()];

    mollusk.process_and_validate_instruction(&initialize_instruction, &initialize_tx_accounts, checks);
    mollusk.process_and_validate_instruction(&deposit_instruction, &deposit_tx_accounts, checks);
    // mollusk.process_and_validate_instruction(&withdraw_instruction, &withdraw_tx_accounts, checks);
    // mollusk.process_and_validate_instruction(&close_instruction, &close_tx_accounts, checks);

}