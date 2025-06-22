use anchor_lang::prelude::*;

pub mod constants;
pub mod instructions;

use instructions::*;

declare_id!("wHwT65qyLdWvFwQvkfsgsJwwmf1v4d3JQcsyXDCrg5L");

#[program]
pub mod meteora_integrate {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn test_integration(ctx: Context<TestIntegration>) -> Result<()> {
        instructions::test_integration(ctx)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
