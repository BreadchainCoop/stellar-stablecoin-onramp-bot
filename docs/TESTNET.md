# Launching & testing the escrow on Stellar testnet

End-to-end: deploy the Soroban escrow to **testnet**, bind it to **USDC**, fund it,
and prove a release. Everything here is testnet — no real funds.

## Live deployment (testnet)

Deployed and verified end-to-end on **2026-10-21** (deposit → owner-only release),
bound to real Circle testnet USDC.

| Item | Value |
| --- | --- |
| Escrow contract | `CBGXBSIUEJYWDUA5UJNTP37NOEZN2FYNIOTME5S6N5LJWCPRTVRP5BL3` |
| Wasm hash (sha256) | `6f3cbfccbd774d61b29c7ea89fc637497fcd905dce85250054556cf31d1c3ad3` |
| Owner / operator | `GBOMEGI2WUGLCYDAWMWTGKDQ3OKSKT2PMPFKEO4AE5LCG4RZT42J3QRW` |
| USDC issuer (Circle testnet) | `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` |
| USDC token (SAC) | `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` |
| CLI used | `stellar-cli 28.1.0` (testnet runs Protocol 23; a P22 CLI can't parse the response) |

Build: `cargo build --target wasm32v1-none --release` (matches CI; pinned
`Cargo.lock`, soroban-sdk `22.0.7`). The wasm was deployed via `--wasm-hash`.

Proof transactions ([stellar.expert/explorer/testnet/contract/CBGXBSIU…](https://stellar.expert/explorer/testnet/contract/CBGXBSIUEJYWDUA5UJNTP37NOEZN2FYNIOTME5S6N5LJWCPRTVRP5BL3)):

- Deploy: `580a3ab5e46543025411983eb1385574ca1e4c3ecc8a154c26d44a21193e7341`
- Init (owner+token): `cf11981c8f98cbd6e6f92e3adb98e951639bf2292489b67a2992f26b8f7536a8`
- Deposit 15 USDC: `6e8d7a1f7dde4e8c715a186ab62631f0c6183d409b74c158b429301fedc10f0a`
- Release 10 USDC → `GBB4YVM5…`: `3fb2b27540a1ce9a5cab138c43e840a1f6d6c6d679142c2da22c106a18e34176`

After: escrow balance 5 USDC, recipient 10 USDC. Operator got its 20 testnet USDC
by buying on the testnet SDEX with XLM (path payment) — the Circle web faucet is
reCAPTCHA-gated.

## 0. Prerequisites

```bash
# Rust + the Soroban wasm target
rustup target add wasm32v1-none
# Stellar CLI (provides `stellar contract …`)
cargo install --locked stellar-cli
stellar --version
```

## 1. A funded testnet identity (pays fees)

```bash
# Create a CLI identity and fund it from Friendbot (testnet faucet).
stellar keys generate operator --network testnet --fund
stellar keys address operator        # G... (this is the escrow OWNER/operator)
```

`--fund` asks Friendbot for test XLM. Re-run `stellar keys fund operator --network testnet` if the balance runs low.

## 2. Build & deploy the escrow

```bash
cd contracts
make build          # -> target/wasm32v1-none/release/escrow.wasm
stellar contract deploy \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source operator \
  --network testnet \
  --alias escrow
# prints the contract id (C...). The --alias stores it for later `--id escrow`.
```

## 3. Resolve the testnet USDC token (SAC) address

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

## 4. Initialize the escrow (owner + token)

```bash
stellar contract invoke --id escrow --source operator --network testnet \
  -- init \
  --owner  $(stellar keys address operator) \
  --token  <USDC_SAC_ID>
# confirm:
stellar contract invoke --id escrow --source operator --network testnet -- owner
stellar contract invoke --id escrow --source operator --network testnet -- balance
```

## 5. Fund the escrow with test USDC

⚠️ **Decimals:** Stellar USDC has **7 decimals**. 1 USDC = `10_000_000` in the
`i128` amount. Keep this consistent with the backend quote math.

Option B (your own token) — mint straight to the escrow:

```bash
stellar contract invoke --id <USDC_SAC_ID> --source issuer --network testnet \
  -- mint --to <ESCROW_CONTRACT_ID> --amount 100000000   # 10 USDC
stellar contract invoke --id escrow --source operator --network testnet -- balance
```

## 6. Prove a release

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

## 7. Verify the source on StellarExpert

- The contract embeds source metadata (`contractmeta!` → `repo`, `name`), visible
  on the contract's StellarExpert page under testnet
  (`https://stellar.expert/explorer/testnet/contract/<id>`).
- Full source verification is a **reproducible build**: StellarExpert (or the
  community verification service) rebuilds the wasm from this repo at the deployed
  commit and matches its hash to the on-chain wasm. Keep the toolchain pinned
  (soroban-sdk `22.0.7`, `wasm32v1-none`, the workspace release profile) so the
  build is reproducible, and tag the commit you deploy.

## Notes for the backend port

- **Release signer** = the operator account (`require_auth` on `release`/`refund`).
  Testnet can use a raw secret key; mainnet needs KMS/MPC or multisig.
- **Trustline + account existence** are release preconditions for user wallets —
  check before taking payment (ties into the liquidity pre-check).
- **7-decimal** amounts end to end.

## Sources

- Stellar docs — [Deploy to Testnet](https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-to-testnet)
- Stellar docs — [Deploy the Stellar Asset Contract for a Stellar asset](https://developers.stellar.org/docs/build/guides/cli/deploy-stellar-asset-contract)
- Circle — [USDC on Stellar (precision & addressing)](https://developers.circle.com/cctp/quickstarts/transfer-usdc-stellar-arc)
