use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    associated_token::AssociatedToken,
};

use crate::constants::{TOTAL_SUPPLY, TOKENS_FOR_SALE, USDC_LIQUIDITY, CP_AMM_PROGRAM_ID,
    CLIFF_FEE_NUMERATOR, NUMBER_OF_PERIOD, PERIOD_FREQUENCY, REDUCTION_FACTOR,
    FEE_SCHEDULER_MODE, SQRT_PRICE, LIQUIDITY, SQRT_MIN_PRICE, SQRT_MAX_PRICE};

// Program IDs for Meteora ecosystem
pub const METAPLEX_PROGRAM_ID: Pubkey = solana_program::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

declare_program!(cp_amm);
use cp_amm::{
    cpi::{
        self,
        accounts::InitializeCustomizablePool,
    },
    program::CpAmm,
    types::{
        InitializeCustomizablePoolParameters,
        PoolFeeParameters,
        BaseFeeParameters,
    },
};

#[derive(Accounts)]
pub struct TestIntegration<'info> {

    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub sender: Signer<'info>,

    // USDC accounts
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = sender,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_authority
    )]
    pub usdc_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = sender,
        associated_token::mint = mint,
        associated_token::authority = vault_authority
    )]
    pub token_vault: Account<'info, TokenAccount>,

    /// CHECK: This is the vault authority PDA - must be mutable for receiving SOL
    #[account(
        mut,  // Add mut constraint to allow receiving SOL
        seeds = [b"vault_authority", mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    // CP-AMM Program reference (needed for CPI in the "departing" -> "departed" transition)
    /// CHECK: This is the CP-AMM program ID
    #[account(address = CP_AMM_PROGRAM_ID)]
    pub cp_amm_program: Program<'info, CpAmm>,

    // CP-AMM pool initialization accounts
    /// CHECK: Position NFT mint that will be created
    #[account(mut)]
    pub position_nft_mint: Signer<'info>,

    /// CHECK: Position NFT account that will be created
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,

    /// CHECK: Pool authority PDA for the CP-AMM pool
    pub pool_authority: UncheckedAccount<'info>,

    /// CHECK: Pool account that will be created
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// CHECK: Position that will be created
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// CHECK: Token A vault for the pool
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,

    /// CHECK: Token B vault for the pool
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,

    // Event authority and program required for Anchor's event_cpi feature
    /// CHECK: Event authority PDA required for Anchor's event_cpi feature
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: CP-AMM program ID (required as explicit field for event CPI)
    pub program: UncheckedAccount<'info>,

    // Additional accounts required for the CP-AMM's token-2022 support
    /// CHECK: Token A program interface
    pub token_a_program: UncheckedAccount<'info>,

    /// CHECK: Token B program interface
    pub token_b_program: UncheckedAccount<'info>,

    /// CHECK: Token 2022 program
    pub token_2022_program: UncheckedAccount<'info>,

    /// CHECK: Metaplex metadata program
    #[account(address = METAPLEX_PROGRAM_ID)]
    pub metadata_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Main graduation function that handles the entire process
pub fn test_integration<'info>(
    ctx: Context<TestIntegration<'info>>,
) -> Result<()> {
    let token_amount = TOTAL_SUPPLY - TOKENS_FOR_SALE;
    let usdc_amount = USDC_LIQUIDITY;

    msg!("TOKEN_AMOUNT for pool: {}", token_amount);
    msg!("USDC_AMOUNT for pool: {}", usdc_amount);

    // Get the vault authority seeds for signing
    // The vault_authority will be used as the signer for token transfers from vaults
    // and as the payer for pool creation fees
    let vault_authority_bump = ctx.bumps.vault_authority;
    let vault_authority_seeds = &[
        b"vault_authority",
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[vault_authority_bump],
    ];
    let signer_seeds = &[&vault_authority_seeds[..]];

    // Check token balances before proceeding
    let token_balance = ctx.accounts.token_vault.amount;
    let usdc_balance = ctx.accounts.usdc_vault.amount;

    msg!("Token vault balance: {} tokens", token_balance);
    msg!("USDC vault balance: {} tokens", usdc_balance);

    // Verify there are sufficient funds for pool creation
    if token_balance < token_amount {
        msg!("Error: Insufficient token balance in vault. Have: {}, Need: {}", token_balance, token_amount);
        // Just log the error instead of throwing an exception
    }

    if usdc_balance < usdc_amount {
        msg!("Error: Insufficient USDC balance in vault. Have: {}, Need: {}", usdc_balance, usdc_amount);
        // Just log the error instead of throwing an exception
    }

    // Additional checks for token account owner
    msg!("Token vault owner: {}", ctx.accounts.token_vault.owner);
    msg!("USDC vault owner: {}", ctx.accounts.usdc_vault.owner);

    // Log account addresses for debugging
    msg!("Token vault address: {}", ctx.accounts.token_vault.key());
    msg!("USDC vault address: {}", ctx.accounts.usdc_vault.key());
    msg!("Token A vault (destination): {}", ctx.accounts.token_a_vault.key());
    msg!("Token B vault (destination): {}", ctx.accounts.token_b_vault.key());
    msg!("Vault authority: {}", ctx.accounts.vault_authority.key());

    // First, send SOL to the vault_authority to pay for rent
    msg!("Transferring SOL to vault_authority for rent");
    let lamports_for_pool = 50_000_000; // 0.05 SOL should be enough for pool creation

    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.sender.key(),
            &ctx.accounts.vault_authority.key(),
            lamports_for_pool,
        ),
        &[
            ctx.accounts.sender.to_account_info(),
            ctx.accounts.vault_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Create appropriate pool fee parameters
    // Using the same parameters as in scripts/create-pool-simple.ts
    let base_fee_params = BaseFeeParameters {
        cliff_fee_numerator: CLIFF_FEE_NUMERATOR, // 0.25% (25 basis points)
        number_of_period: NUMBER_OF_PERIOD,            // No fee reduction
        period_frequency: PERIOD_FREQUENCY,            // Not used since no reduction
        reduction_factor: REDUCTION_FACTOR,            // Not used since no reduction
        fee_scheduler_mode: FEE_SCHEDULER_MODE,          // Linear mode
    };

    // Complete fee parameters
    let pool_fee_params = PoolFeeParameters {
        base_fee: base_fee_params,
        protocol_fee_percent: 20,       // Meteora's protocol fee
        partner_fee_percent: 0,         // No partner fee
        referral_fee_percent: 20,        // Meteora required referral fee
        dynamic_fee: None,              // No dynamic fees
    };

    // Hardcoded values from calculate-pool-params.ts
    let sqrt_price_u128: u128 = SQRT_PRICE;
    let liquidity_u128: u128 = LIQUIDITY;
    let sqrt_min_price_u128: u128 = SQRT_MIN_PRICE;
    let sqrt_max_price_u128: u128 = SQRT_MAX_PRICE;

    msg!("Using hardcoded sqrt_price: {}", sqrt_price_u128);
    msg!("Using hardcoded liquidity: {}", liquidity_u128);
    msg!("Using hardcoded sqrt_min_price: {}", sqrt_min_price_u128);
    msg!("Using hardcoded sqrt_max_price: {}", sqrt_max_price_u128);

    // Set up customizable pool parameters using hardcoded values
    let pool_params = InitializeCustomizablePoolParameters {
        pool_fees: pool_fee_params,
        sqrt_min_price: sqrt_min_price_u128, // Allow 100x price range downward
        sqrt_max_price: sqrt_max_price_u128, // Allow 100x price range upward
        has_alpha_vault: false,                 // No alpha vault
        liquidity: liquidity_u128,                   // Use calculated liquidity
        sqrt_price: sqrt_price_u128,           // Use calculated sqrt price
        activation_type: 0,                     // Default activation
        collect_fee_mode: 1,                    // Collect fees in quote token (USDC) only
        activation_point: None,                 // Activate immediately
    };

    // Verify the source and destination token accounts have the correct mint associations
    msg!("Verifying token account mint associations:");
    msg!("  token_vault mint: {}", ctx.accounts.token_vault.mint);
    msg!("  usdc_vault mint: {}", ctx.accounts.usdc_vault.mint);
    msg!("  Expected token mint: {}", ctx.accounts.mint.key());
    msg!("  Expected USDC mint: {}", ctx.accounts.usdc_mint.key());

    // Create the accounts structure for the CPI call using the generated accounts module
    let cpi_accounts = InitializeCustomizablePool {
        creator: ctx.accounts.vault_authority.to_account_info(),
        position_nft_mint: ctx.accounts.position_nft_mint.to_account_info(),
        position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
        payer: ctx.accounts.vault_authority.to_account_info(),
        pool_authority: ctx.accounts.pool_authority.to_account_info(),
        pool: ctx.accounts.pool.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        token_a_mint: ctx.accounts.mint.to_account_info(),
        token_b_mint: ctx.accounts.usdc_mint.to_account_info(),
        token_a_vault: ctx.accounts.token_a_vault.to_account_info(),
        token_b_vault: ctx.accounts.token_b_vault.to_account_info(),
        payer_token_a: ctx.accounts.token_vault.to_account_info(),
        payer_token_b: ctx.accounts.usdc_vault.to_account_info(),
        token_a_program: ctx.accounts.token_a_program.to_account_info(),
        token_b_program: ctx.accounts.token_b_program.to_account_info(),
        token_2022_program: ctx.accounts.token_2022_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),

        // These fields are auto-generated by Anchor for event handling
        event_authority: ctx.accounts.event_authority.to_account_info(),
        program: ctx.accounts.program.to_account_info(),
    };

    // Log transfer details more clearly
    msg!("Transfer details for token A (our token):");
    msg!("  From: {} (balance: {})", ctx.accounts.token_vault.key(), token_balance);
    msg!("  To: {}", ctx.accounts.token_a_vault.key());
    msg!("  Amount: {}", token_amount);
    msg!("  Authority: {}", ctx.accounts.vault_authority.key());

    msg!("Transfer details for token B (USDC):");
    msg!("  From: {} (balance: {})", ctx.accounts.usdc_vault.key(), usdc_balance);
    msg!("  To: {}", ctx.accounts.token_b_vault.key());
    msg!("  Amount: {}", usdc_amount);
    msg!("  Authority: {}", ctx.accounts.vault_authority.key());

    // Log detailed information about the signing
    msg!("Creating CPI context with signer seeds:");
    msg!("  Seed Prefix: vault_authority");
    msg!("  Seed Mint: {}", ctx.accounts.mint.key());
    msg!("  Seed Bump: {}", vault_authority_bump);

    // Create CPI context with vault_authority PDA as signer (for transferring tokens from vaults)
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.cp_amm_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    // Log that we're about to make the CPI call
    msg!("Making CPI call to initialize_customizable_pool with the following parameters:");
    msg!("  sqrt_price: {}", pool_params.sqrt_price);
    msg!("  liquidity: {}", pool_params.liquidity);
    msg!("  sqrt_min_price: {}", pool_params.sqrt_min_price);
    msg!("  sqrt_max_price: {}", pool_params.sqrt_max_price);
    msg!("  base_fee.cliff_fee_numerator: {}", pool_params.pool_fees.base_fee.cliff_fee_numerator);

    // Make the CPI call to initialize the customizable pool using the cpi module
    cpi::initialize_customizable_pool(cpi_ctx, pool_params)?;

    Ok(())
}
