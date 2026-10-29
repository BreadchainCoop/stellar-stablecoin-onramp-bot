# AGENTS.md

stellar-stablecoin-onramp-bot — fiat↔USDC on/off ramp on Stellar, rebuilt from the EVM ramp: a Telegram bot takes the order, Sentoo collects a local bank transfer, and a Soroban escrow releases USDC to the buyer. `contracts/escrow` is the pooled-custody escrow (Rust, Soroban); `backend/` (Express webhooks, order lifecycle, the only signer), `bot/` (Grammy, holds no key) and `supabase/` (ordered Postgres migrations) are READMEs until ported. Posture is **testnet + provider sandbox only**. Docs in `docs/` (start at `docs/ARCHITECTURE.md`, then `docs/TESTNET.md`). This file is the canonical agent guide; host-specific files import it and never override it.

## Commands

```bash
cd contracts
make test                        # cargo test
make build                       # stellar contract build -> target/wasm32v1-none/release/escrow.wasm
make fmt                         # cargo fmt --all
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings   # CI lint gate
cargo audit && cargo llvm-cov --locked --fail-under-lines 90   # CI audit + coverage gate
make deploy SOURCE=<identity> OWNER=<G…> TOKEN=<C…>   # testnet only, only when asked (docs/TESTNET.md)
```

`bot/` and `backend/` get `npm ci && npm test` once ported.

## Hard Rules

**Commits.** One line, `<type>(<scope>): <imperative summary>`, under 72 chars, no body unless asked. Never add `Co-Authored-By` or any AI attribution to commits or PRs. Never commit, push, deploy or run a mainnet script unless asked.

**Plan first, human review before merge.** Applies to payment webhooks and order transitions, KYC, the escrow (release, withdraw, signing), migrations and RLS, deploy config, secrets, and any mainnet path. No unsupervised autonomy on mainnet work.

**Don't claim what isn't done.** The contract audit, KMS/MPC custody and regulatory review are outstanding. Never describe them as done.

**Soroban toolchain.** `soroban-sdk = "=29.0.1"`, once, in `[workspace.dependencies]`; edition 2021; `wasm32v1-none`; the workspace release profile as is (`overflow-checks = true`). Build with `stellar contract build`, never plain `cargo build`. Rust is pinned in `contracts/rust-toolchain.toml` and stellar-cli `26.1.0` in CI; bump them together.

**Escrow shape.** Pooled custody with off-chain order accounting: never imply per-order balances on-chain. One-time setup goes in `__constructor`, never a public `init`. `release` and `withdraw` are owner-only: load the owner from storage and `require_auth` it. No upgrade entry point; a new version is a new deploy, drained with `withdraw`.

**Amounts.** `i128` in USDC's 7 decimals (1 USDC = `10_000_000`). Reject non-positive amounts and check the pool balance before any transfer, each with a typed `Error`, never `unwrap` or `panic!`.

**Events.** Every state-changing entry point emits a `#[contractevent]`, and a test asserts it.

**Contract tests.** Cover deposit, release, withdraw, insufficient balance and owner auth. Every `require_auth` gets two tests: the exact `env.auths()` tree, and a wrong signer failing with `Error(Auth, InvalidAction)`. Assert errors as `Err(Ok(Error::X))`. Run `make test` and `make build` before handoff.

**Doc comments.** `///` on every entry point, two lines at most. The module doc is one line of what; design narrative goes in `docs/ARCHITECTURE.md`.

**Sentoo webhook is an unauthenticated ping.** It carries only a transaction id. Never derive payment status from the body: re-fetch it from the Sentoo API, then act.

**Order status.** Compare-and-set transitions only, never an unconditional update. The status audit is append-only.

**Release target.** Release to the payout address pinned at order-create, never the user's current address.

**Liquidity pre-check.** Before taking a payment: escrow balance − reserved − buffer ≥ amount, and the recipient account exists with a USDC trustline. Fail closed, so no buyer pays for USDC that can't be delivered.

**Telegram return link.** Use a non-`start` query param (`?ref=`), so Sentoo's appended status segment stays in the query and the chat opens without re-running `/start`.

**Keys.** The escrow owner is the backend operator account (ed25519); the bot never holds a signing key. A raw secret key is testnet-only; mainnet needs KMS/MPC or a multisig, and never recommend a raw key for it. Never hardcode, log or commit a secret key, provider secret or RPC credential. Pass `stellar` CLI identities by name, never a raw secret.

When corrected, add a rule here.
