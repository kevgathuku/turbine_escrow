use anchor_lang::prelude::*;

// InitSpace: avoid having to manually calculate the rent
#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    // Random number used during seed derivation 
    // so one maker can open multiple escrows with the same token pair
    pub seed: u64,
    // The wallet that created the escrow
    pub maker: Pubkey,
    // The SPL mints address for the "make" side of the swap
    pub mint_a: Pubkey,
    // The SPL mints address for the "take"  side of the swap
    pub mint_b: Pubkey,
    // How much of token B the maker wants
    pub receive: u64,
    // Cached bump byte. filled by bumps.escrow
    pub bump: u8,
    // pub expiration: i64
}
