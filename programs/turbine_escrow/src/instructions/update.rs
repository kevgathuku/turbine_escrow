use anchor_lang::prelude::*;

use crate::{error::EscrowError, state::Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(receive: u64)]
pub struct Update<'info> {
    pub maker: Signer<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        constraint = receive > 0 @ EscrowError::InvalidReceive,
        has_one = maker @ EscrowError::InvalidMaker,
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update(&mut self, receive: u64) -> Result<()> {
        self.escrow.receive = receive;
        Ok(())
    }
}
