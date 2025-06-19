use anchor::VaultState;
use anchor_lang::InstructionData;
use anchor_spl::associated_token::spl_associated_token_account;
#[cfg(test)]
use mollusk_svm::{Mollusk, program, result::Check};
use mollusk_svm_programs_token::token::{
    keyed_account as keyed_account_for_token_program,
};
use solana_sdk::{
    account::{Account, AccountSharedData, WritableAccount, ReadableAccount},
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    program_option::COption, 
    program_pack::Pack,
    pubkey::Pubkey,
};
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

//maker a test for each instructions (explain why in Anchor you use steruct and vanila rust you use new_with_bytes())

#[test]
fn test_initialize_vault() {

    let (system_program, system_account) = program::keyed_account_for_system_program();

    let (token_program, token_program_account) = keyed_account_for_token_program();

    // let rent_sysvar = Pubkey::from_str_const("SysvarRent111111111111111111111111111111111");
    let rent_sysvar = solana_sdk::sysvar::rent::id();

    let (mollusk, program_id, user, token_mint, token_mint_account, _) = init_mollusk();

        //Derive vault state PDA
    let (vault_state_pda, _) =
        Pubkey::find_program_address(&["vault".as_ref(), user.as_ref(), token_mint.as_ref()], &program_id);

    //Derive Vault PDA
    let (vault_account_pda, _) =
        Pubkey::find_program_address(&["vault_account".as_ref(), vault_state_pda.as_ref()], &program_id);

    //Initialize Acounts
    let user_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);
    //Before being initialized the owne of the account is the system program
    let state_account = Account::new(0, 0, &system_program);
    let vault_account = Account::new(0, 0, &system_program);

    // Create the actual rent sysvar account with proper data
    let rent_account = solana_sdk::account::create_account_shared_data_for_test(
        &solana_sdk::sysvar::rent::Rent::default()
    );

    let ix_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new(token_mint, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(rent_sysvar, false),
    ];

    let data = (anchor::instruction::Initialize {}).data();

    //Create the initialize instruction
    let instruction = Instruction::new_with_bytes(program_id, &data, ix_accounts);

    let tx_accounts = &vec![
        (user, user_account.clone()),
        (vault_state_pda, state_account.clone()),
        (vault_account_pda, vault_account.clone()),
        (token_mint, token_mint_account.clone().into()),
        (system_program, system_account.clone()),
        (token_program, token_program_account.clone()),
        (rent_sysvar, rent_account.clone().into()),
    ];

    let checks = &vec![Check::success()];

    let _deposit_result =
        mollusk.process_and_validate_instruction(&instruction, tx_accounts, checks);

}


#[test]
fn test_deposit_vault() {
    let (mollusk, program_id, user, token_mint, token_mint_account, user_token_account) = init_mollusk();

    let (system_program, _) = program::keyed_account_for_system_program();
    let (token_program, token_program_account) = keyed_account_for_token_program();

    // Derive vault state PDA
    let (vault_state_pda, state_bump) =
        Pubkey::find_program_address(&["vault".as_ref(), user.as_ref(), token_mint.as_ref()], &program_id);

    // Derive Vault PDA
    let (vault_account_pda, vault_account_bump) =
        Pubkey::find_program_address(&["vault_account".as_ref(), vault_state_pda.as_ref()], &program_id);

    let mut vault_state_account = Account::new(
        mollusk
            .sysvars
            .rent
            .minimum_balance(VaultState::SPACE), 
            VaultState::SPACE, 
            &program_id
        );

    // Set the vault state fields
    let initial_vault_state = anchor::state::VaultState {
        user,
        mint: token_mint,
        bump: state_bump as u8,
        bump_token_account: vault_account_bump as u8,
        deposited: 0,
        _padding: [0; 6], // Padding to ensure the size is correct
    };
    
    // Get data allocated in state_account
    let state_data = vault_state_account.data_as_mut_slice();
    
    // Write the discriminator (8 bytes)
    // For Anchor accounts, the discriminator is the first 8 bytes of the SHA256 hash of the account name
    let discriminator: [u8; 8] = anchor_lang::solana_program::hash::hash(b"account:VaultState").to_bytes()[..8].try_into().unwrap();
    state_data[..8].copy_from_slice(&discriminator);
    
    // Write the vault state data directly (zero-copy accounts use direct memory copy)
    unsafe {
        let vault_state_ptr = state_data[8..].as_mut_ptr() as *mut anchor::state::VaultState;
        *vault_state_ptr = initial_vault_state;
    }

    // Create vault token account (owned by the PDA)
    let vault_token_account = get_token_account(&vault_state_pda, &token_mint, 0);

    // Amount to deposit
    let deposit_amount = 5_000_000;

    // Store initial balances for verification
    let initial_user_balance = 10_000_000; // From common.rs
    let initial_vault_balance = 0;

    let user_ata = spl_associated_token_account::get_associated_token_address(&user, &token_mint);

    let ix_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_ata, false), // user_account (token account)
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
    ];

    let data = (anchor::instruction::Deposit { amount: deposit_amount }).data();

    // Create the deposit instruction
    let instruction = Instruction::new_with_bytes(program_id, &data, ix_accounts);

    let tx_accounts = &vec![
        (user, Account::new(LAMPORTS_PER_SOL, 0, &system_program)),
        (user_ata, user_token_account.clone().into()), // user's token account
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, vault_token_account.clone().into()),
        (token_mint, token_mint_account.clone().into()),
        (token_program, token_program_account.clone()),
    ];

    // Process the instruction
    let result = mollusk.process_instruction(&instruction, tx_accounts);

    // Verify success
    assert!(!result.program_result.is_err(), "Deposit instruction failed");

    // Check account balances changed correctly
    // User token account should have decreased by deposit_amount
    let user_token_account_after = result.get_account(&user_ata).unwrap();
    let user_balance_after = u64::from_le_bytes(
        user_token_account_after.data[64..72].try_into().unwrap() // Offset for amount field
    );
    assert_eq!(
        user_balance_after, 
        initial_user_balance - deposit_amount,
        "User token balance should decrease by deposit amount"
    );

    // Vault token account should have increased by deposit_amount
    let vault_token_account_after = result.get_account(&vault_account_pda).unwrap();
    let vault_balance_after = u64::from_le_bytes(
        vault_token_account_after.data[64..72].try_into().unwrap() // Offset for amount field
    );
    println!("Vault balance after deposit: {}", vault_balance_after);
    assert_eq!(
        vault_balance_after,
        initial_vault_balance + deposit_amount,
        "Vault token balance should increase by deposit amount"
    );

    // Check vault state deposited amount was updated
    let vault_state_after = result.get_account(&vault_state_pda).unwrap();
    let deposited_amount = u64::from_le_bytes(
        vault_state_after.data[72..80].try_into().unwrap() // Offset for deposited field
    );
    assert_eq!(
        deposited_amount,
        deposit_amount,
        "Vault state deposited amount should be updated"
    );

    println!("Deposit successful!");
    println!("User balance: {} -> {}", initial_user_balance, user_balance_after);
    println!("Vault balance: {} -> {}", initial_vault_balance, vault_balance_after);
}

#[test]
fn test_withdraw_vault() {
    let (mollusk, program_id, user, token_mint, token_mint_account, _) = init_mollusk();

    let (system_program, _) = program::keyed_account_for_system_program();
    let (token_program, token_program_account) = keyed_account_for_token_program();

    // Derive vault state PDA
    let (vault_state_pda, state_bump) =
        Pubkey::find_program_address(&["vault".as_ref(), user.as_ref(), token_mint.as_ref()], &program_id);

    // Derive Vault PDA
    let (vault_account_pda, vault_account_bump) =
        Pubkey::find_program_address(&["vault_account".as_ref(), vault_state_pda.as_ref()], &program_id);

    let mut vault_state_account = Account::new(
        mollusk
            .sysvars
            .rent
            .minimum_balance(VaultState::SPACE), 
            VaultState::SPACE, 
            &program_id
        );

    // Set the vault state fields with existing deposit
    let existing_deposit = 8_000_000;
    let initial_vault_state = anchor::state::VaultState {
        user,
        mint: token_mint,
        bump: state_bump as u8,
        bump_token_account: vault_account_bump as u8,
        deposited: existing_deposit,
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
        *vault_state_ptr = initial_vault_state;
    }

    // Create vault token account with existing balance
    let vault_token_account = get_token_account(&vault_state_pda, &token_mint, existing_deposit);

    // Update user token account balance (user has less tokens because they deposited)
    let initial_user_balance = 2_000_000; // User has 2M tokens after depositing 8M
    let user_token_account = get_token_account(&user, &token_mint, initial_user_balance);

    // Amount to withdraw
    let withdraw_amount = 1_000_000;

    let user_ata = spl_associated_token_account::get_associated_token_address(&user, &token_mint);

    let ix_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_ata, false),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
    ];

    let data = (anchor::instruction::Withdraw { amount: withdraw_amount }).data();

    // Create the withdraw instruction
    let instruction = Instruction::new_with_bytes(program_id, &data, ix_accounts);

    let tx_accounts = &vec![
        (user, Account::new(LAMPORTS_PER_SOL, 0, &system_program)),
        (user_ata, user_token_account.clone().into()),
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, vault_token_account.clone().into()),
        (token_mint, token_mint_account.clone().into()),
        (token_program, token_program_account.clone()),
    ];

    // Process the instruction
    let result = mollusk.process_instruction(&instruction, tx_accounts);

    // Verify success
    assert!(!result.program_result.is_err(), "Withdraw instruction failed");

    // Check account balances changed correctly
    // User token account should have increased by withdraw_amount
    let user_token_account_after = result.get_account(&user_ata).unwrap();
    let user_balance_after = u64::from_le_bytes(
        user_token_account_after.data[64..72].try_into().unwrap()
    );
    assert_eq!(
        user_balance_after, 
        initial_user_balance + withdraw_amount,
        "User token balance should increase by withdraw amount"
    );

    // Vault token account should have decreased by withdraw_amount
    let vault_token_account_after = result.get_account(&vault_account_pda).unwrap();
    let vault_balance_after = u64::from_le_bytes(
        vault_token_account_after.data[64..72].try_into().unwrap()
    );
    assert_eq!(
        vault_balance_after,
        existing_deposit - withdraw_amount,
        "Vault token balance should decrease by withdraw amount"
    );

    // Check vault state deposited amount was updated
    let vault_state_after = result.get_account(&vault_state_pda).unwrap();
    let deposited_amount = u64::from_le_bytes(
        vault_state_after.data[72..80].try_into().unwrap()
    );
    assert_eq!(
        deposited_amount,
        existing_deposit - withdraw_amount,
        "Vault state deposited amount should be updated"
    );

    println!("Withdraw successful!");
    println!("User balance: {} -> {}", initial_user_balance, user_balance_after);
    println!("Vault balance: {} -> {}", existing_deposit, vault_balance_after);
    println!("Deposited amount: {} -> {}", existing_deposit, deposited_amount);
}

#[test]
fn test_close_vault() {
    let (mollusk, program_id, user, token_mint, token_mint_account, _) = init_mollusk();

    let (system_program, system_account) = program::keyed_account_for_system_program();
    let (token_program, token_program_account) = keyed_account_for_token_program();

    // Derive vault state PDA
    let (vault_state_pda, state_bump) =
        Pubkey::find_program_address(&["vault".as_ref(), user.as_ref(), token_mint.as_ref()], &program_id);

    // Derive Vault PDA
    let (vault_account_pda, vault_account_bump) =
        Pubkey::find_program_address(&["vault_account".as_ref(), vault_state_pda.as_ref()], &program_id);

    let mut vault_state_account = Account::new(
        mollusk
            .sysvars
            .rent
            .minimum_balance(VaultState::SPACE), 
            VaultState::SPACE, 
            &program_id
        );

    // Set the vault state fields with remaining deposit
    let remaining_deposit = 1_500_000;
    let initial_vault_state = anchor::state::VaultState {
        user,
        mint: token_mint,
        bump: state_bump as u8,
        bump_token_account: vault_account_bump as u8,
        deposited: remaining_deposit,
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
        *vault_state_ptr = initial_vault_state;
    }

    // Create vault token account with remaining balance
    let vault_token_account = get_token_account(&vault_state_pda, &token_mint, remaining_deposit);

    // Update user token account balance
    let initial_user_balance = 8_500_000;
    let user_token_account = get_token_account(&user, &token_mint, initial_user_balance);

    let user_ata = spl_associated_token_account::get_associated_token_address(&user, &token_mint);

    let ix_accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(user_ata, false),
        AccountMeta::new(vault_state_pda, false),
        AccountMeta::new(vault_account_pda, false),
        AccountMeta::new_readonly(token_mint, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = (anchor::instruction::CloseVault {}).data();

    // Create the close instruction
    let instruction = Instruction::new_with_bytes(program_id, &data, ix_accounts);

    // Store initial lamports for rent recovery check
    let initial_user_lamports = LAMPORTS_PER_SOL;
    let vault_state_lamports = vault_state_account.lamports();
    let vault_account_lamports = vault_token_account.lamports();

    let tx_accounts = &vec![
        (user, Account::new(initial_user_lamports, 0, &system_program)),
        (user_ata, user_token_account.clone().into()),
        (vault_state_pda, vault_state_account.clone()),
        (vault_account_pda, vault_token_account.clone().into()),
        (token_mint, token_mint_account.clone().into()),
        (token_program, token_program_account.clone()),
        (system_program, system_account.clone()),
    ];

    // Process the instruction
    let result = mollusk.process_instruction(&instruction, tx_accounts);

    // Verify success
    assert!(!result.program_result.is_err(), "Close vault instruction failed");

    // Check that all tokens were transferred to user
    let user_token_account_after = result.get_account(&user_ata).unwrap();
    let user_balance_after = u64::from_le_bytes(
        user_token_account_after.data[64..72].try_into().unwrap()
    );
    assert_eq!(
        user_balance_after, 
        initial_user_balance + remaining_deposit,
        "User should receive all remaining tokens from vault"
    );

    // Check that vault accounts were closed (should not exist)
    assert!(
        result.get_account(&vault_state_pda).is_none() || 
        result.get_account(&vault_state_pda).unwrap().lamports() == 0,
        "Vault state account should be closed"
    );
    assert!(
        result.get_account(&vault_account_pda).is_none() || 
        result.get_account(&vault_account_pda).unwrap().lamports() == 0,
        "Vault token account should be closed"
    );

    // Check that user received rent from closed accounts
    let user_account_after = result.get_account(&user).unwrap();
    let expected_lamports = initial_user_lamports + vault_state_lamports + vault_account_lamports;
    assert_eq!(
        user_account_after.lamports(),
        expected_lamports,
        "User should receive rent from closed accounts"
    );

    println!("Close vault successful!");
    println!("User token balance: {} -> {}", initial_user_balance, user_balance_after);
    println!("Tokens recovered from vault: {}", remaining_deposit);
    println!("Rent recovered: {} lamports", vault_state_lamports + vault_account_lamports);
}
