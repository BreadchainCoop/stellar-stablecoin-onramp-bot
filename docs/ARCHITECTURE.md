# Architecture

Stellar/Soroban rebuild of the EVM fiat↔USDC ramp. Same product and off-chain
flow; the chain layer is Stellar.

## Flow (on-ramp)
1. Bot (Telegram) takes the order and the user's Stellar payout address.
2. Backend creates a Sentoo payment link (local bank transfer).
3. User pays → Sentoo webhook → backend re-fetches authoritative status.
4. Backend calls the Soroban escrow `release` (signed by the operator) → USDC to
   the user's address.

Off-ramp is the reverse (user sends USDC to the escrow, receives a bank payout).

## Components
- **Soroban escrow** (`contracts/escrow`): pooled custody, owner-only release.
- **Backend**: webhooks, order state machine, release signer.
- **Bot**: chat UX, quotes, order creation. No keys.
- **Supabase**: users/orders/audit.

## Ported invariants (from the EVM ramp)
- Sentoo webhook is untrusted; re-fetch status before acting.
- Compare-and-set order transitions + append-only audit.
- Release to the payout address pinned at order-create.
- Liquidity pre-check before taking payment (fail closed).
- `?ref=` Telegram return link (survives Sentoo's appended `/status`).

## Stellar-specific
- USDC via its **SAC**; **7 decimals**; recipients need a **trustline** and a
  created account.
- Operator account signs `release`/`refund` (`require_auth`). Mainnet custody =
  KMS/MPC/multisig, not a raw key.
- Networks: Testnet → (Futurenet) → Mainnet, bounded canary first.
