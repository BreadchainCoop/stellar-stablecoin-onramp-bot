# Soroban escrow — build / test / deploy
#
# Prereqs: Rust + the wasm target, and the Stellar CLI.
#   rustup target add wasm32v1-none
#   cargo install --locked stellar-cli --version 26.1.0
#
# Deploy vars (override on the command line or via env). Deploys go to testnet
# only; mainnet needs a KMS/multisig owner, never this target.
#   SOURCE    a `stellar keys` identity name that pays fees (never a raw secret)
#   OWNER     the operator (backend signer) address that will own the escrow
#   TOKEN     the USDC token contract (SAC) address on testnet

WASM_DIR = target/wasm32v1-none/release
WASM     = $(WASM_DIR)/escrow.wasm

.PHONY: build test fmt clean deploy

build:
	stellar contract build

test:
	cargo test

fmt:
	cargo fmt --all

clean:
	cargo clean

# Deploy the compiled escrow; its constructor binds OWNER (operator) + TOKEN
# (USDC SAC) in the same transaction. Prints the new contract id.
#   make deploy SOURCE=<id> OWNER=<G...> TOKEN=<C...>
deploy: build
	$(if $(and $(SOURCE),$(OWNER),$(TOKEN)),,$(error deploy needs SOURCE, OWNER and TOKEN))
	stellar contract deploy \
		--wasm $(WASM) \
		--source $(SOURCE) \
		--network testnet \
		-- --owner $(OWNER) --token $(TOKEN)
