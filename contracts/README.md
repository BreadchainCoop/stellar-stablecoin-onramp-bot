# Soroban contracts

Rust/Soroban smart contracts for the Stellar on/off ramp.

- **`escrow/`** — pooled-custody USDC escrow. `init` (owner + token, once),
  `deposit`, owner-only `release` / `refund`, `balance`, `owner`, `token`.
  Mirrors the EVM `Escrow.sol`. Binds to the USDC Stellar Asset Contract (SAC).

Toolchain: soroban-sdk `22.0.7`, edition 2021, target `wasm32v1-none` (matches
the CoopStable contracts).

```bash
make test     # cargo test
make build    # stellar contract build -> target/wasm32v1-none/release/escrow.wasm
make fmt
```

Deploy + testnet USDC: see [`../docs/TESTNET.md`](../docs/TESTNET.md).
