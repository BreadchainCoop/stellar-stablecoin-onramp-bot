# Launching & testing the escrow on Stellar testnet

End-to-end: deploy the Soroban escrow to **testnet**, bind it to **USDC**, fund it,
and prove a release. Everything here is testnet — no real funds.

## Live deployment (testnet)

Deployed and proven end-to-end on **2026-10-21** (deposit → owner-only release),
bound to real Circle testnet USDC.

> **Pre-review contract.** This deployment runs `v0.1.0` (commit `0a88113`):
> SDK 22, a public `init`, `refund`. The reviewed contract (constructor,
> `withdraw`, SDK 29) has a different interface and needs a new release and
> deploy (§6); these ids won't accept its calls.

**Source-verified escrow** (StellarExpert `status: verified`, from the attested
`v0.1.0` release build):

| Item | Value |
| --- | --- |
| Escrow contract | `CAKKY5WKDXZRQVGD2UR72AV4FMZOS7EW3UGRET4CMQSX3QMWQTNEALJT` |
| Wasm hash (sha256) | `4131294462173056ac60795fbdad1c6cc566de1484382b2d1ab816d6038108ba` |
| Verified at commit | `0a88113` (tag `v0.1.0`), path `contracts`, package `escrow` |
| Attested release | `v0.1.0_contracts_escrow_cli28.1.0` (stellar-expert/soroban-build-workflow) |
| Owner / operator | `GBOMEGI2WUGLCYDAWMWTGKDQ3OKSKT2PMPFKEO4AE5LCG4RZT42J3QRW` |
| USDC issuer (Circle testnet) | `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` |
| USDC token (SAC) | `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` |
| CLI | `stellar-cli 28.1.0` (testnet runs Protocol 23; a P22 CLI can't parse the response) |

The on-chain wasm hash == the attested release wasm hash == `4131…` (confirmed by
`stellar contract fetch`), and the GitHub build attestation verifies against
`--signer-repo stellar-expert/soroban-build-workflow`.

**First deploy** (unverified — predates the `source_repo`/attestation setup, kept
for history): escrow `CBGXBSIUEJYWDUA5UJNTP37NOEZN2FYNIOTME5S6N5LJWCPRTVRP5BL3`,
wasm `6f3cbfcc…`, built with `cargo build --target wasm32v1-none --release`.

Proof transactions on the **verified** escrow ([stellar.expert/explorer/testnet/contract/CAKKY5WK…](https://stellar.expert/explorer/testnet/contract/CAKKY5WKDXZRQVGD2UR72AV4FMZOS7EW3UGRET4CMQSX3QMWQTNEALJT)):

- Deploy (attested wasm): `7202b16a8408bf533254ca2d0c5ed19138bfba432548b114ebbd74d9a31e13ec`
- Init (owner+token): `08417b611627ff03adb80b1e7424617e71652a67116835b701e5ae2d50c751f2`
- Deposit 4 USDC: escrow → 4 USDC
- Release 2 USDC → `GBB4YVM5…`: `573027bd2543d58721a406bb3cc616fbc49f2d80f7d45e6500c753a5024ac6f7`

After: escrow balance 2 USDC, recipient total 12 USDC. (The first, unverified
escrow was proven the same way: deposit 15 → release 10, balance 5.) Operator got
its 20 testnet USDC by buying on the testnet SDEX with XLM (path payment) — the
Circle web faucet is reCAPTCHA-gated.

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
is no separate init call for anyone to front-run. For a source-verified deploy,
use the attested release wasm instead of a local build (§6).

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

## 6. Deploy an attested release (source-verified wasm)

StellarExpert marks a contract **source-verified** when it finds a GitHub build
attestation for the on-chain wasm hash, produced by
[`stellar-expert/soroban-build-workflow`](https://github.com/stellar-expert/soroban-build-workflow).
That works on testnet: the pre-review `CAKKY5WK…` above shows `status: verified`.

- The contract's `name` and `repo` metadata (`contractmeta!`) are labels, not a
  verification. The workflow injects `source_repo`, the key StellarExpert uses to
  link the wasm to this repo and commit.
- **One-time repo prerequisite (admin):** Settings → Actions → General → Workflow
  permissions must allow the `id-token`, `contents` and `attestations` writes that
  `release.yml` declares, or the attestation step fails.
- Pushing a tag `v<version>` (the workspace version in `contracts/Cargo.toml`)
  runs `.github/workflows/release.yml`. It runs the contracts CI, then the build
  workflow, which builds an optimized wasm, publishes it as a GitHub release and
  attests it.
- Check the downloaded wasm against the attestation, then deploy that file instead
  of a local build:

```bash
gh attestation verify escrow_v<version>.wasm --repo BreadchainCoop/stellar-stablecoin-onramp-bot
stellar contract deploy --wasm escrow_v<version>.wasm --source operator --network testnet \
  --alias escrow -- --owner $(stellar keys address operator) --token <USDC_SAC_ID>
```

- The release is built with `--optimize` and the workflow's stellar-cli (28.1.0),
  so its hash differs from a local `make build`.

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
