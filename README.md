# turbine_escrow

[![CI](https://github.com/kevgathuku/turbine_escrow/actions/workflows/ci.yml/badge.svg)](https://github.com/kevgathuku/turbine_escrow/actions/workflows/ci.yml)

1. Opens an escrow offering token A for token B (`make`)
2. Accepts the escrow, swapping the tokens and closing both PDAs (`take`)
3. Cancels the escrow, refunding token A and closing both PDAs (`refund`)
4. Reprices an open escrow, changing only the receive amount (`update`)

Each escrow is a PDA holding the swap terms (`seed`, `maker`, both mints, `receive`), with the offered tokens locked in a vault ATA owned by that PDA.

## Setup

Requirements: Rust 1.91, Solana CLI, Anchor CLI, a funded localnet wallet.

Wallet for the provider (see `Anchor.toml`):

```txt
~/.config/solana/id.json
```

## Commands

```bash
# build the program
anchor build
# run all LiteSVM tests
anchor test
```

## Tests

### make

Make an escrow: creates the `escrow` PDA (`seeds = ["escrow", maker, seed]`) and the `vault` ATA for `mint_a`, then moves the deposit of token A from the maker into the vault.

Verifies:

- stored terms (seed, maker, both mints, receive amount).
- vault funded with exactly the deposit, maker debited.
- Rejects zero deposit (`InvalidDeposit`).
- Rejects zero receive (`InvalidReceive`).
- Rejects same mint both sides (`DuplicateMint`).
- Rejects maker balance below deposit (`InsufficientBalance`).

### take

Take the escrow: moves the receive amount of token B from taker to maker, drains the full vault of token A to the taker, then closes both PDAs.

Verifies:

- both token movements executed (taker got A, maker got B).
- both PDAs gone, rents refunded.
- stranger can take (permissionless by design).
- Rejects taker balance below receive (`InsufficientBalance`).
- Rejects wrong mint on either side (`InvalidMintA` / `InvalidMintB`).
- Rejects second take after close (`AccountNotInitialized`).

### refund

Refund the escrow: maker-only cancel that moves the full vault of token A back to the maker, then closes both PDAs.

Verifies:

- full deposit returned to the maker.
- both PDAs gone, rents refunded.
- failed attempts move nothing.
- Rejects stranger attempting to cancel (`ConstraintSeeds`).

### update

Update the escrow: maker-only reprice that changes the receive amount without touching the vault.

Verifies:

- receive amount updated to the new value.
- vault balance and maker balance untouched.
- Rejects zero receive (`InvalidReceive`).

## Notes

- Tests run against the freshly built `.so` — rebuild after any `src/` change
- Tests build instructions through shared helpers in `programs/turbine_escrow/tests/common/mod.rs`
- Token setup (mints, ATAs, funding) goes through real Token/ATA program transactions via `litesvm-token` builders, not faked accounts
