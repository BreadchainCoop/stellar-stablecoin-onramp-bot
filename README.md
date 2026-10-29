# Stellar: Stablecoin on/off ramp bot

A Telegram chat bot that lets people move money on and off chain on **Stellar**,
straight from a conversation. Pay with a local bank transfer (via Sentoo) and
receive **USDC on Stellar**, or cash out stablecoins back to the bank. No app
download: chat, pay, and receive USDC at your Stellar address. That address
needs a funded account with a USDC trustline.

This is the **Stellar / Soroban** rebuild of the EVM ramp
([`curacao-crypto-onramp-bot`](https://github.com/BreadchainCoop/curacao-crypto-onramp-bot)).
The product and off-chain flow carry over; the chain layer is rebuilt for Stellar:
Soroban (Rust) smart contracts instead of Solidity, and the Stellar SDK instead of
ethers.

> Status: **early scaffold.** The Soroban escrow contract and project structure
> are in place; the bot and backend are being ported. Posture is testnet +
> provider sandbox only. Do not treat audit, KMS custody, or regulatory review as
> done.

## How it works

1. A user chats the **bot** (Telegram) and starts a buy.
2. The **backend** creates a **Sentoo** payment link for a local bank transfer.
3. The user pays; Sentoo calls the backend webhook.
4. The backend verifies the payment and asks the **Soroban escrow** to `release`
   USDC to the user's Stellar address.

Off-ramp is the reverse: the user sends USDC to the escrow and receives a bank
payout.

## Architecture

| Layer | Tech | Notes |
|---|---|---|
| `contracts/` | **Soroban (Rust)** | Pooled-custody USDC escrow: constructor (owner + token) / `deposit` / `release` / `refund` / `balance`. Owner-only release. Uses the SAC token interface so it works with Stellar USDC. |
| `bot/` | Node + Grammy | Telegram chat UX, order flow, quotes. Signs nothing. |
| `backend/` | Node + Express | Sentoo webhooks, order state machine, escrow release signer. |
| `supabase/` | Postgres | Users, orders, append-only status audit. |

### Stellar specifics

- **USDC on Stellar** is Circle's asset. For Soroban it's accessed through its
  **Stellar Asset Contract (SAC)** — the escrow is bound to that contract address
  and holds/moves the token via the standard `token` interface. Amounts are `i128`.
- **Accounts/keys** are ed25519. The escrow owner is the backend **operator**
  account; the release path must be signed by it. Custody (raw keypair vs KMS/MPC)
  is a deliberate decision — see the EVM project's custody notes; Privy supports
  ed25519, and Stellar-native options (e.g. Turnkey/DFNS) are on the table.
- **Networks:** Testnet, Futurenet, Mainnet (Pubnet). RPC via the Stellar CLI /
  Soroban RPC.

## Repo layout

```
contracts/   Soroban Rust workspace (escrow contract + tests + Makefile)
bot/         Telegram bot (to port)
backend/     Express webhooks + escrow release (to port)
supabase/    DB migrations (to port)
docs/        Architecture and plans
```

## Contracts quickstart

```bash
# prereqs
rustup target add wasm32v1-none
cargo install --locked stellar-cli --version 26.1.0

cd contracts
make test        # run the Soroban unit tests
make build       # compile the escrow to wasm
# deploy (testnet): fund a CLI identity first, then
make deploy SOURCE=<identity> NETWORK=testnet OWNER=<G...operator> TOKEN=<C...usdc_sac>
```

## What carries over from the EVM ramp

The non-chain design is proven and reused deliberately (not copied wholesale):
Sentoo payment-link + webhook handling (re-fetch authoritative status, never trust
the body), the compare-and-set order state machine, the `?ref=` Telegram return
link, the escrow **liquidity pre-check** before taking a payment, and the
pinned-payout-wallet rule. See `docs/ARCHITECTURE.md`.

## Roadmap

- [x] Repo + Soroban escrow contract + tests
- [ ] Deploy escrow to Stellar testnet, bind USDC SAC
- [ ] Port the backend release signer to the Stellar SDK (operator account / KMS)
- [ ] Port the bot (Stellar address validation, quotes, Sentoo flow)
- [ ] Port Supabase schema
- [ ] Testnet end-to-end, then a bounded mainnet canary

## License

MIT
