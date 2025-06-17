## Vault — Instruction-Level Specification

*(Solana Anchor program, one vault per **{user × underlying-token}**; no share-mint)*

---

### Common Design Elements

| Element                                   | Description                                                                                                                                                                                                                                                        |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Seeds / PDAs**                          | `vault_state   = ["vault", owner, mint]` (stores meta & balances)  <br>`vault_token   = ["vault_token", vault_state]` (Actual SPL-Token account that holds the underlying)   |
| **Authority Model**                       | Only `user` (the wallet that created the vault) may deposit, withdraw, or close. Program signs via `vault_state` for CPI calls into the SPL-Token program.                                                                                                         |
| **State Layout (`VaultState` zero-copy)** | `{ owner: Pubkey, mint: Pubkey, bump: u8, bump_token_account: u8, deposited: u64, _paddidng: [u8; 6] }`                                                                                                                                                                   |
| **Events**                                | `InitializeEvent { owner, mint }`, `DepositEvent { owner, mint, amount }`, `WithdrawEvent { owner, mint, amount }`, `CloseEvent { owner, mint }`                                                                                                                   |
| **Error Codes**                           | `VaultAlreadyExists`, `Unauthorized`, `InvalidMint`, `InsufficientBalance`, `NonZeroBalance`, `MathOverflow`                                                                                                                                                       |
| **Constraints**                           | Program is upgrade-able via multisig; all token transfers use checked CPI (`transfer_checked`).                                                                                                                                                                    |
| **Dependencies**                          | Token 2022 not required; standard SPL-Token program v3.5+                                                                                                                                                                                                          |

---

## 1. `initialize_vault`

|                   |                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Purpose**       | Create a dedicated vault for *one* SPL mint and bind it to the user.                                                                                                                                                                                                                                                                                                                                                                                   |
| **Accounts**      | 1. `user` — Signer, funds rent. <br>2. `vault_state` (PDA, init, space = 8 + size\_of\<VaultState>)  <br>3. `vault_account` (PDA, associated token account for `mint`, owned by `vault_state`, init if needed) <br>4. `mint` — SPL Mint to be vaulted. <br>5. `system_program`, `token_program`, `rent` |
| **Args**          | *none* — mint is provided as account.                                                                                                                                                                                                                                                                                                                                                                                                                  |
| **Checks**        | • Fail if another `VaultState` with same seeds exists.<br>• Ensure `mint.supply > 0`.<br>• Verify PDAs bumps.                                                                                                                                                                                                                                                                                                                                          |
| **State Effects** | • Allocate & populate `vault_state`.<br>• Initial `deposited = 0`.                                                                                                                                                                                                                                                                                                                                                                                     |
| **Events**        | Emit `InitializeEvent`.                                                                                                                                                                                                                                                                                                                                                                                                                                |

---

## 2. `deposit`

|                   |                                                                                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Purpose**       | Move `amount` tokens from the user’s wallet into their vault; increment internal balance.                                                                                                   |
| **Accounts**      | 1. `user` — Signer.<br>2. `user_account` — User’s associated token account for `mint`.<br>3. `vault_account` (mut).<br>4. `mint` (mut).<br>5. `token_program` |
| **Args**          | `amount: u64` (in `mint.decimals` units)                                                                                                                                                    |
| **Checks**        | • `vault_state.owner == owner`.<br>• `user_account.mint == vault_state.mint`.<br>• `amount > 0`.<br>• No overflow when adding to `deposited`.                                           |
| **Process**       | CPI → `transfer_checked` from `user_account` → `vault_account`, signer = `owner`.                                                                                                         |
| **State Effects** | `vault_state.deposited += amount`.                                                                                                                                                          |
| **Events**        | `DepositEvent`.                                                                                                                                                                             |

---

## 3. `withdraw`

|                   |                                                                                                                                                                                    |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Purpose**       | Return up to `amount` of the underlying to the owner; reduce internal balance.                                                                                                     |
| **Accounts**      | 1. `user` — Signer.<br>2. `user_account` — Owner’s ATA for `mint`.<br>3. `vault_state` (mut).<br>4. `vault_account` (mut).<br>6. `token_program` |
| **Args**          | `amount: u64`                                                                                                                                                                      |
| **Checks**        | • `vault_state.user == user`.<br>• `amount > 0`.<br>• `amount ≤ vault_state.deposited`.<br>• `user_account.mint == vault_state.mint`.                                          |
| **Process**       | CPI → `transfer_checked` from `vault_token` → `user_account`.                                                                                             |
| **State Effects** | `vault_state.deposited -= amount`.                                                                                                                                                 |
| **Events**        | `WithdrawEvent`.                                                                                                                                                                   |

---

## 4. `close_vault`

|                   |                                                                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Purpose**       | Close both the token account and state account when balance is zero, reclaiming rent for the user.                                                                             |
| **Accounts**      | 1. `user` — Signer.<br>2. `vault_state` (mut, close → `user`).<br>3. `vault_account` (mut, close → `user`).<br>4. `token_program`, `system_program` |
| **Args**          | *none*                                                                                                                                                                         |
| **Checks**        | • `vault_state.user == user`.<br>• `vault_state.deposited == 0`.<br>• `vault_token.amount == 0`.                                                                             |
| **Process**       | CPI → `close_account` on `vault_token`.                                                                                                                 |
| **State Effects** | Deallocate `vault_state`; rent returned to `owner`.                                                                                                                            |
| **Events**        | `CloseEvent`.                                                                                                                                                                  |

---

### Sequence Diagram (high-level)

```
User Wallet        Program (Vault)           SPL Token
    |                    |                       |
init| create PDAs        |                       |
    |------------------->|                       |
    |                    |                       |
dep | transfer_checked ------------------------->|
    |                    |                       |
    |<-------event-------|                       |
wd  |                    | transfer_checked ---->|
    |<-------event-------|                       |
cls | close_account ---------------------------->|
    |                    |                       |
```

---

### Edge-Cases & Safeguards

* **Re-entrancy:** Anchor’s single-TX model prevents it, but all CPI calls occur after state mutations to be explicit.
* **Forced Token Upgrade:** If an underlying mint upgrades its program id, admin can migrate by closing vaults (requires zero balance).
* **Rent Hikes:** Periodic script can iterate vaults to top-up rent if Solana rent mechanics change.

---

### Suggested Unit-Test Matrix

| Scenario                                       | Expected Result              |
| ---------------------------------------------- | ---------------------------- |
| Double initialize with same seeds              | Fails `VaultAlreadyExists`   |
| Deposit 0                                      | Fails `InvalidArgument`      |
| Withdraw over balance                          | Fails `InsufficientBalance`  |
| Close with non-zero balance                    | Fails `NonZeroBalance`       |
| Full-cycle (init → deposit → withdraw → close) | Passes and all rent returned |

---

*Please note some decisions have been made to demonstrate Anchor functionality e.g we don't necessary need to explicityly track deposited amount in vault*
