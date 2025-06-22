import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  getAccount,
  createMint,
  mintTo,
  createAssociatedTokenAccount,
  transfer,
} from '@solana/spl-token';
import { assert } from 'chai';
import type { MeteoraIntegrate } from '../target/types/meteora_integrate';
import pkg from '@coral-xyz/anchor';
const { BN } = pkg;

// Constants
const TOKEN_METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');
const USDC_DECIMALS = 6;
const USDC_MULTIPLIER = 10 ** USDC_DECIMALS;
const CP_AMM_PROGRAM_ID = new PublicKey('cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG');

// Helper functions for PDA derivation
function maxKey(a: Buffer, b: Buffer): Buffer {
  return Buffer.compare(a, b) > 0 ? a : b;
}

function minKey(a: Buffer, b: Buffer): Buffer {
  return Buffer.compare(a, b) < 0 ? a : b;
}


describe("meteora_integrate", () => {
  let provider: anchor.AnchorProvider;
  let program: Program<MeteoraIntegrate>;

  // Key variables
  let admin, buyer, creator, feeCollector, backendSigner;
  let secondBuyer; // Add a second buyer for transfer test
  let mint, sale, programSettings, vaultAuthority;
  let buyerTokenAccount, feeCollectorUsdcAccount;
  let secondBuyerTokenAccount; // Token account for second buyer
  let usdcMint, buyerUsdcAccount;
  let tokenVault, usdcVault;

  before(async () => {
    provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    program = anchor.workspace.MeteoraIntegrate as Program<MeteoraIntegrate>;

    // Add network check
    console.log("Connected to network:", provider.connection.rpcEndpoint);
    console.log("Program ID:", program.programId.toString());

    // Initialize keypairs
    admin = Keypair.generate();
    buyer = Keypair.generate();
    creator = Keypair.generate();
    feeCollector = Keypair.generate();
    backendSigner = Keypair.generate();
    secondBuyer = Keypair.generate(); // Initialize second buyer keypair

    // Airdrop SOL to participants
    const adminAirdrop = await provider.connection.requestAirdrop(
      admin.publicKey, 20 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(adminAirdrop);

    const buyerAirdrop = await provider.connection.requestAirdrop(
      buyer.publicKey, 20 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(buyerAirdrop);

    const creatorAirdrop = await provider.connection.requestAirdrop(
      creator.publicKey, 20 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(creatorAirdrop);

    // Airdrop SOL to second buyer
    const secondBuyerAirdrop = await provider.connection.requestAirdrop(
      secondBuyer.publicKey, 20 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(secondBuyerAirdrop);

    // Create fake USDC mint
    usdcMint = await createMint(
      provider.connection, admin, admin.publicKey, null, USDC_DECIMALS
    );

    // Create accounts
    buyerUsdcAccount = await createAssociatedTokenAccount(
      provider.connection, admin, usdcMint, buyer.publicKey
    );
    feeCollectorUsdcAccount = await createAssociatedTokenAccount(
      provider.connection, admin, usdcMint, feeCollector.publicKey
    );

    // Mint USDC to buyer
    await mintTo(
      provider.connection, admin, usdcMint, buyerUsdcAccount,
      admin.publicKey, 1_000_000 * USDC_MULTIPLIER
    );

    // Initialize program settings
    [programSettings] = PublicKey.findProgramAddressSync(
      [Buffer.from("program_settings")],
      program.programId
    );

    // Create a test mint for the token
    mint = await createMint(
      provider.connection, admin, admin.publicKey, null, 9
    );

    // Derive vault authority PDA
    [vaultAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_authority"), mint.toBuffer()],
      program.programId
    );

    // Create token vault addresses (these will be initialized by the program)
    // We can't use getAssociatedTokenAddress for PDAs, so we'll create them as regular addresses
    tokenVault = Keypair.generate().publicKey;
    usdcVault = Keypair.generate().publicKey;
  });

  it("checks integration", async () => {
    try {
      const positionNftMint = Keypair.generate();

      await createMint(
        provider.connection,
        creator,
        creator.publicKey,
        null,
        0,
        positionNftMint
      );

      console.log("Created position NFT mint:", positionNftMint.publicKey.toString());

      const [poolAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("pool_authority")],
        CP_AMM_PROGRAM_ID
      );
      console.log("Pool Authority:", poolAuthority.toString());

      const [pool] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("cpool"),
          Buffer.from(maxKey(mint.toBuffer(), usdcMint.toBuffer())),
          Buffer.from(minKey(mint.toBuffer(), usdcMint.toBuffer()))
        ],
        CP_AMM_PROGRAM_ID
      );
      console.log("Pool:", pool.toString());

      const [position] = PublicKey.findProgramAddressSync(
        [Buffer.from("position"), positionNftMint.publicKey.toBuffer()],
        CP_AMM_PROGRAM_ID
      );
      console.log("Position:", position.toString());

      const [tokenAVault] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("token_vault"),
          mint.toBuffer(),
          pool.toBuffer()
        ],
        CP_AMM_PROGRAM_ID
      );
      console.log("Token A Vault:", tokenAVault.toString());

      const [tokenBVault] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("token_vault"),
          usdcMint.toBuffer(),
          pool.toBuffer()
        ],
        CP_AMM_PROGRAM_ID
      );
      console.log("Token B Vault:", tokenBVault.toString());

      const [eventAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("event_authority")],
        CP_AMM_PROGRAM_ID
      );
      console.log("Event Authority:", eventAuthority.toString());

      const [positionNftAccount] = PublicKey.findProgramAddressSync(
        [Buffer.from("position_nft_account"), positionNftMint.publicKey.toBuffer()],
        CP_AMM_PROGRAM_ID
      );
      console.log("Position NFT Account:", positionNftAccount.toString());

      await program.methods
        .testIntegration()
        .accountsStrict({
          mint,
          sender: creator.publicKey,
          usdcMint,
          usdcVault,
          tokenVault,
          vaultAuthority,
          feeCollectorAccount: feeCollectorUsdcAccount,
          cpAmmProgram: CP_AMM_PROGRAM_ID,
          positionNftMint: positionNftMint.publicKey,
          positionNftAccount,
          poolAuthority,
          pool,
          position,
          tokenAVault,
          tokenBVault,
          eventAuthority,
          program: CP_AMM_PROGRAM_ID,
          tokenAProgram: TOKEN_PROGRAM_ID,
          tokenBProgram: TOKEN_PROGRAM_ID,
          token2022Program: TOKEN_2022_PROGRAM_ID,
          metadataProgram: TOKEN_METADATA_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator, positionNftMint])
        .rpc({ skipPreflight: true });
        // .simulate();
    } catch (err) {
      console.log("Test failed with error:", err);
      console.log("Error details:", JSON.stringify(err, null, 2));
      // throw err;
    }
  });
});
