# Backend (to port)

Express webhooks + order lifecycle + escrow release (the **only** signer). Port
from the EVM ramp:

- Sentoo webhook: unauthenticated ping → re-fetch authoritative status → act.
- Compare-and-set order transitions; append-only status audit.
- Escrow release via the **Stellar SDK**, signed by the operator account
  (`require_auth`). Testnet raw key; mainnet KMS/MPC/multisig.
- Liquidity pre-check (balance − reserved − buffer ≥ amount; fail closed) +
  recipient trustline/account check before taking payment.

Not yet implemented — see [`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md).
