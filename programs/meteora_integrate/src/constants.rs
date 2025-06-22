use anchor_lang::prelude::*;

// Token decimals
pub const TOKEN_DECIMALS: u8 = 6;                 // 6 decimal places for our token
pub const USDC_DECIMALS: u8 = 6;                  // 6 decimal places for USDC (replacing SOL)


// Token configuration
pub const TOTAL_SUPPLY: u64 = 1000000 * 1_000_000; // 1M tokens with 6 decimals
pub const SALE_PERCENTAGE: u64 = 70; // 70% of total supply for sale

// 0.01 USDC per token (1¢) - starting price
pub const START_PRICE: u64 = 10_000; // 0.01 USDC with 6 decimals

// 0.02 USDC per token (2¢) - ending price after all tokens are sold
pub const END_PRICE: u64 = 20_000; // 0.02 USDC with 6 decimals

// Calculate tokens for sale
pub const TOKENS_FOR_SALE: u64 = TOTAL_SUPPLY * SALE_PERCENTAGE / 100;

pub const USDC_LIQUIDITY: u64 = 9500_000_000; // 9500 USDC

// Devnet USDC mint address - this will be replaced with the actual fake USDC mint address
// This is a placeholder and should be updated with the actual mint address after running the create-fake-usdc script
pub const USDC_MINT_ADDRESS: &str = "FHgn3Su3sV1LGqJzyvRGFEBsw4qNNXW8witNdgzu4FNo";

// Meteora CP-AMM Program ID
pub const CP_AMM_PROGRAM_ID: Pubkey = anchor_lang::solana_program::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

pub const CLIFF_FEE_NUMERATOR: u64 = 1_000_000; // 0.1% (meets MIN_FEE_NUMERATOR requirement)
pub const NUMBER_OF_PERIOD: u16 = 1; // No fee reduction
pub const PERIOD_FREQUENCY: u64 = 0; // Not used since no reduction
pub const REDUCTION_FACTOR: u64 = 0; // Not used since no reduction
pub const FEE_SCHEDULER_MODE: u8 = 0; // Linear mode

pub const SQRT_PRICE: u128 = 3193225655092720520;
pub const LIQUIDITY: u128 = 959628863123313138957945808262;
pub const SQRT_MIN_PRICE: u128 = 1844674407370955;
pub const SQRT_MAX_PRICE: u128 = 1844674407370955161600;
