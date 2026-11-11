use anchor_lang::prelude::*;

// InitSpace derives the max serialized size so space never goes stale
#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    // Random number used during seed derivation
    // so one maker can open multiple escrows with the same token pair
    pub seed: u64,
    // The wallet that created the escrow
    pub maker: Pubkey,
    // The SPL mint the maker deposits ("make" side)
    pub mint_a: Pubkey,
    // The SPL mint the maker wants in return ("take" side)
    pub mint_b: Pubkey,
    // Amount of mint_b the maker wants, in base units
    pub receive: u64,
    // Cached PDA bump, set from ctx.bumps.escrow
    pub bump: u8,
    // pub expiration: i64
}
