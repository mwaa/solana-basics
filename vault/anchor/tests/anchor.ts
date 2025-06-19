import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Anchor } from "../target/types/anchor";
import { assert } from "chai";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo, getAccount, TOKEN_PROGRAM_ID, TokenAccountNotFoundError, } from "@solana/spl-token";


describe("anchor", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.anchor as Program<Anchor>;
  const provider = anchor.getProvider();
  const connection = provider.connection;
  const payer = (provider.wallet as any).payer;
  let user: anchor.web3.Keypair;
  let mint: anchor.web3.PublicKey;
  let userTokenAccount: anchor.web3.PublicKey;
  let vaultState: anchor.web3.PublicKey;
  let vaultAccount: anchor.web3.PublicKey;

  before(async () => {
    user = anchor.web3.Keypair.generate();
    // Airdrop SOL to user
    const tx = await provider.connection.requestAirdrop(user.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    });

    // Create mint
    mint = await createMint(
      connection,
      payer,
      user.publicKey,
      null,
      6 // decimals
    );
    // Create user's associated token account
    userTokenAccount = (await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      user.publicKey
    )).address;
    // Mint tokens to user
    await mintTo(
      connection,
      payer,
      mint,
      userTokenAccount,
      user,
      1_000_000
    );

    // Derive PDAs
    [vaultState] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    );
    [vaultAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_account"), vaultState.toBuffer()],
      program.programId
    );
  });

  it("Initializes the vault", async () => {
    await program.methods.initialize().accountsStrict({
      user: user.publicKey,
      vaultState,
      vaultAccount,
      mint,
      systemProgram: anchor.web3.SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    }).signers([user]).rpc();
    // Check vault state account exists
    const vaultStateAcc = await program.account.vaultState.fetch(vaultState);
    assert.ok(vaultStateAcc.user.equals(user.publicKey));
    assert.ok(vaultStateAcc.mint.equals(mint));
  });

  it("Deposits tokens", async () => {
    await program.methods.deposit(new anchor.BN(100_000)).accountsStrict({
      user: user.publicKey,
      userAccount: userTokenAccount,
      vaultState,
      vaultAccount,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([user]).rpc();
    // Check vault account balance
    const vaultAcc = await getAccount(connection, vaultAccount);
    assert.equal(Number(vaultAcc.amount), 100_000);
    // Check vault state deposited
    const vaultStateAcc = await program.account.vaultState.fetch(vaultState);
    assert.equal(Number(vaultStateAcc.deposited), 100_000);
  });

  it("Withdraws tokens", async () => {
    await program.methods.withdraw(new anchor.BN(50_000)).accountsStrict({
      user: user.publicKey,
      userAccount: userTokenAccount,
      vaultState,
      vaultAccount,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([user]).rpc();
    // Check vault account balance
    const vaultAcc = await getAccount(connection, vaultAccount);
    assert.equal(Number(vaultAcc.amount), 50_000);
    // Check vault state deposited
    const vaultStateAcc = await program.account.vaultState.fetch(vaultState);
    assert.equal(Number(vaultStateAcc.deposited), 50_000);
  });

  it("Closes the vault", async () => {
    await program.methods.closeVault().accountsStrict({
      user: user.publicKey,
      userAccount: userTokenAccount,
      vaultState,
      vaultAccount,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([user]).rpc();
    // Vault account should be closed (throws if not found)
    try {
      await getAccount(connection, vaultAccount);
      assert.fail("Vault account not closed");
    } catch (e) {
      assert.instanceOf(e, TokenAccountNotFoundError);
    }

    const isAccount = await connection.getAccountInfo(vaultState);
    assert.isNull(isAccount, "Vault state account should be closed");

  });
});
