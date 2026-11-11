pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("EedMZJZX9UJTqRRfEe1QowaJTGz81ZFA4zCVFbsy4dWy");

#[program]
pub mod turbine_escrow {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        ctx.accounts.init_escrow(seed, receive, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.transfer_to_maker()?;
        ctx.accounts.withdraw_and_close_vault()
    }

    //  #[instruction(discriminator = 2)]
    // pub fn refund(ctx: Context<Refund>) -> Result<()> {
    //     Ok(())
    // }
}
