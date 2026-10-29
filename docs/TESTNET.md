# Launching & testing the escrow on Stellar testnet

End-to-end: deploy the Soroban escrow to **testnet**, bind it to **USDC**, fund it,
and prove a release. Everything here is testnet — no real funds.

## 0. Prerequisites

```bash
# Rust + the Soroban wasm target
rustup target add wasm32v1-none
# Stellar CLI (provides `stellar contract …`)
cargo install --locked stellar-cli --version 26.1.0
stellar --version
```

## 1. A funded testnet identity (pays fees)

```bash
# Create a CLI identity and fund it from Friendbot (testnet faucet).
stellar keys generate operator --network testnet --fund
stellar keys address operator        # G... (this is the escrow OWNER/operator)
```

`--fund` asks Friendbot for test XLM. Re-run `stellar keys fund operator --network testnet` if the balance runs low.

## 2. Resolve the testnet USDC token (SAC) address

Soroban moves tokens through a **Stellar Asset Contract (SAC)**, not the issuer
account. Two options:

**A. Circle testnet USDC (realistic).** The classic asset is
`USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` (verify against
Circle's current testnet issuer). Get its SAC id:

```bash
stellar contract id asset \
  --asset USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 \
  --network testnet
# -> C... (the USDC SAC address). If it isn't deployed yet on testnet:
#   stellar contract asset deploy --asset USDC:<issuer> --source operator --network testnet
```

To actually hold Circle testnet USDC you need a **trustline** to the issuer and a
faucet/anchor to send you some.

**B. Your own test token (fastest for dev).** Issue a throwaway asset you control
and mint freely — same `token` interface, no faucet needed:

```bash
stellar keys generate issuer --network testnet --fund
stellar contract asset deploy --asset TUSDC:$(stellar keys address issuer) \
  --source issuer --network testnet                 # -> C... (test USDC SAC)
```

Use this SAC id as `TOKEN` below for a self-contained test.

## 3. Build & deploy the escrow (owner + token)

The constructor binds the owner and token inside the deploy transaction, so there
is no separate init call for anyone to front-run.

```bash
cd contracts
make build          # -> target/wasm32v1-none/release/escrow.wasm
stellar contract deploy \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source operator \
  --network testnet \
  --alias escrow \
  -- \
  --owner $(stellar keys address operator) \
  --token <USDC_SAC_ID>
# prints the contract id (C...). The --alias stores it for later `--id escrow`.
# confirm:
stellar contract invoke --id escrow --source operator --network testnet -- owner
stellar contract invoke --id escrow --source operator --network testnet -- balance
```

## 4. Fund the escrow with test USDC

⚠️ **Decimals:** Stellar USDC has **7 decimals**. 1 USDC = `10_000_000` in the
`i128` amount. Keep this consistent with the backend quote math.

Option B (your own token) — mint straight to the escrow:

```bash
stellar contract invoke --id <USDC_SAC_ID> --source issuer --network testnet \
  -- mint --to <ESCROW_CONTRACT_ID> --amount 100000000   # 10 USDC
stellar contract invoke --id escrow --source operator --network testnet -- balance
```

## 5. Prove a release

⚠️ **Trustline:** a classic-asset recipient G-address must have a **USDC
trustline** or the transfer fails. (Contract addresses and SAC-native balances
don't need one; a user's wallet does.) The bot must enforce/surface this before
releasing to a user.

```bash
# recipient with a trustline + (for Circle USDC) created account
stellar contract invoke --id escrow --source operator --network testnet \
  -- release --recipient <G..._or_C...> --amount 10000000   # 1 USDC
```

A successful `release` is signed by the **operator** (owner `require_auth`) — this
is exactly the path the backend will drive with the operator key / KMS.

## 6. Verify the source on StellarExpert

- The contract embeds `name` and `repo` metadata (`contractmeta!`), visible on
  its StellarExpert page (`https://stellar.expert/explorer/testnet/contract/<id>`).
  That is a label, not a verification.
- StellarExpert validates source only for a wasm deployed from a GitHub release
  built by `stellar-expert/soroban-build-workflow` (tag-triggered, attested). That
  workflow is not set up yet, so a locally built wasm is **not** source-verified.
- Build with the pinned toolchain (`contracts/rust-toolchain.toml`, soroban-sdk
  `=29.0.1`, stellar-cli `26.1.0`) and tag the commit you deploy.

## Notes for the backend port

- **Release signer** = the operator account (`require_auth` on `release`/`withdraw`).
  Testnet can use a raw secret key; mainnet needs KMS/MPC or multisig.
- **Trustline + account existence** are release preconditions for user wallets —
  check before taking payment (ties into the liquidity pre-check).
- **7-decimal** amounts end to end.

## Sources

- Stellar docs — [Deploy to Testnet](https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-to-testnet)
- Stellar docs — [Deploy the Stellar Asset Contract for a Stellar asset](https://developers.stellar.org/docs/build/guides/cli/deploy-stellar-asset-contract)
- Circle — [USDC on Stellar (precision & addressing)](https://developers.circle.com/cctp/quickstarts/transfer-usdc-stellar-arc)
