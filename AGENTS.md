# Repository agent guide

Canonical instructions for coding agents in this repository. Host-specific files
may import it, but must not duplicate or override it.

## Project posture

This repo implements a fiat↔USDC on/off ramp on **Stellar**. Changes can affect
payments, KYC state, custody, payouts, and signing keys. Current posture is
**testnet + provider sandbox only**.

Use a plan-first workflow and require human review before merging changes to:
payment webhooks or order transitions; KYC handling; the Soroban escrow, release,
or signing; database migrations or RLS; deploy config, secrets, or any mainnet
path. Do not use unsupervised autonomy for mainnet work. An independent contract
audit, KMS/MPC-backed custody, and regulatory review remain **outstanding** — do
not describe them as done.

## Repository map

- `contracts/` — Soroban (Rust) workspace. `escrow` is the pooled-custody USDC
  escrow. Build/test/deploy via `contracts/Makefile`.
- `bot/` — Telegram bot (Grammy). Creates orders; holds NO signing key.
- `backend/` — Express webhooks, order lifecycle, escrow release (the only signer).
- `supabase/` — ordered Postgres migrations.
- `docs/` — architecture and accepted plans.

## Soroban / contracts rules

- Use **soroban-sdk 22.0.7**, edition 2021, the workspace release profile, and the
  `wasm32v1-none` target (matches the CoopStable contracts).
- `release` and `refund` are **owner-only** (operator `require_auth`). `init` is
  one-time. The escrow is pooled custody with off-chain accounting — do not imply
  per-order balances on-chain.
- Amounts are `i128`; reject non-positive amounts; check balance before transfers.
- Every state-changing fn emits an event. Keep NatSpec-equivalent doc comments.
- Add/keep unit tests (`src/test.rs`) for deposit/release/refund, insufficient
  balance, double-init, and owner-auth. Run `cargo test` and `stellar contract
  build` before handoff.
- Never hardcode or log secret keys, provider secrets, or RPC credentials.

## Money-path invariants (ported from the EVM ramp)

- **Sentoo webhook is an unauthenticated ping** carrying only a transaction id.
  Never derive payment status from the body — re-fetch authoritative status from
  the Sentoo API, then act.
- Change order status with **compare-and-set** transitions; never an unconditional
  update. Preserve the append-only status audit.
- **Release to the payout address pinned at order-create**, not the user's mutable
  current address.
- **Pre-check escrow liquidity before taking a payment** (balance − reserved −
  buffer ≥ amount; fail closed) so a buyer is never charged for USDC that can't be
  delivered.
- The Telegram return link uses a non-`start` query param (e.g. `?ref=`) so Sentoo's
  appended status segment stays in the query and the chat opens without re-running
  `/start`.

## Custody & keys

- The escrow owner is the backend **operator** account (ed25519). Keep the signing
  key off the bot. A raw secret key is **testnet-only**; mainnet needs KMS/MPC or a
  multisig. Do not recommend a raw key for mainnet.

## Commands

```bash
# contracts
cd contracts && make test && make build

# bot / backend (once ported)
cd bot && npm ci && npm test
cd backend && npm ci && npm test
```

Use Conventional Commit subjects (`feat:`, `fix:`, `docs:`, `chore:`). Never
commit, push, deploy, or run a mainnet script unless the user explicitly asks.
