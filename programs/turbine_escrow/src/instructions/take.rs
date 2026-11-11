use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::{error::EscrowError, state::Escrow, ESCROW_SEED};

// Taker accepts the escrow: pays amount `receive` of mint_b to the maker
// Taker receives mint_a from vault, then both PDAs close.
#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    // maker receives reclaimed rent on close; never signs, so plain system account.
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    // `close = maker`: escrow is deleted after take, rent refunded to maker.
    // Box keeps this large context off the stack.
    #[account(
      mut,
      close = maker,
      seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
      bump = escrow.bump,
      has_one = maker @ EscrowError::InvalidMaker,
      has_one = mint_a @ EscrowError::InvalidMintA,
      has_one = mint_b @ EscrowError::InvalidMintB,
  )]
    pub escrow: Box<Account<'info, Escrow>>,

    /// Token Accounts
    // Read-only references; checked via escrow's has_one + the ATA constraints below
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,
    // Escrow's vault for mint_a
    #[account(
      mut,
      associated_token::mint = mint_a,
      associated_token::authority = escrow,
      associated_token::token_program = token_program
  )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // init_if_needed -> Taker may never have held A
    // created on demand, paid for by taker
    #[account(
      init_if_needed,
      payer = taker,
      associated_token::mint = mint_a,
      associated_token::authority = taker,
      associated_token::token_program = token_program
  )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    // Must already exist funded with >= receive: no init here, unlike taker_ata_a
    #[account(
      mut,
      constraint = taker_ata_b.amount >= escrow.receive @ EscrowError::InsufficientBalance,
      associated_token::mint = mint_b,
      associated_token::authority = taker,
      associated_token::token_program = token_program
  )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // init_if_needed -> Maker may never have held B
    // created on demand, paid for by taker
    #[account(
      init_if_needed,
      payer = taker,
      associated_token::mint = mint_b,
      associated_token::authority = maker,
      associated_token::token_program = token_program
  )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Programs
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    // Transfer the receive amount of token B from taker's ATA to maker's ATA
    // authority = taker signs directly; no PDA seeds needed (unlike the vault below)
    pub fn transfer_to_maker(&mut self) -> Result<()> {
        let cpi_program = self.token_program.key();
        let cpi_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)
    }

    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        let cpi_program = self.token_program.key();

        // Transfer token A to the taker
        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        // Vault authority is the escrow PDA, so it signs via seeds, not a wallet signature.
        let seeds = &[
            ESCROW_SEED,
            self.maker.key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        // Entire vault balance of token A sent to taker
        transfer_checked(cpi_context, self.vault.amount, self.mint_a.decimals)?;

        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let cpi_close_context =
            CpiContext::new_with_signer(cpi_program, close_accounts, signer_seeds);

        // Close the vault. Reclaimed rent goes to maker.
        close_account(cpi_close_context)
    }
}
