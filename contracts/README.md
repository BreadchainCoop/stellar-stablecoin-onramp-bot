# Soroban contracts

Rust/Soroban smart contracts for the Stellar on/off ramp.

- **`escrow/`** — pooled-custody USDC escrow. Owner + token set by the
  constructor at deploy; `deposit`, owner-only `release` / `refund`, `balance`,
  `owner`, `token`.
  Mirrors the EVM `Escrow.sol`. Binds to the USDC Stellar Asset Contract (SAC).

Toolchain: soroban-sdk `=29.0.1`, edition 2021, target `wasm32v1-none`, Rust
pinned in `rust-toolchain.toml`, stellar-cli `26.1.0`.

```bash
make test     # cargo test
make build    # stellar contract build -> target/wasm32v1-none/release/escrow.wasm
make fmt
```

Deploy + testnet USDC: see [`../docs/TESTNET.md`](../docs/TESTNET.md).
