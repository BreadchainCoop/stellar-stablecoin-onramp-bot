# Soroban escrow — build / test / deploy
#
# Prereqs: Rust + the wasm target, and the Stellar CLI.
#   rustup target add wasm32v1-none
#   cargo install --locked stellar-cli
#
# Deploy vars (override on the command line or via env):
#   NETWORK   testnet | mainnet            (default testnet)
#   SOURCE    the funded CLI identity/secret used to pay deploy fees
#   OWNER     the operator (backend signer) address that will own the escrow
#   TOKEN     the USDC token contract (SAC) address on the target network

NETWORK ?= testnet
WASM_DIR = target/wasm32v1-none/release
WASM     = $(WASM_DIR)/escrow.wasm

.PHONY: build test fmt clean deploy init

build:
	stellar contract build

test:
	cargo test

fmt:
	cargo fmt --all

clean:
	cargo clean

# Deploy the compiled escrow. Prints the new contract id.
deploy: build
	stellar contract deploy \
		--wasm $(WASM) \
		--source $(SOURCE) \
		--network $(NETWORK)

# Initialize a deployed escrow: bind its owner (operator) + USDC token.
#   make init CONTRACT=<id> OWNER=<G...> TOKEN=<C...> SOURCE=<id> NETWORK=testnet
init:
	stellar contract invoke \
		--id $(CONTRACT) \
		--source $(SOURCE) \
		--network $(NETWORK) \
		-- init --owner $(OWNER) --token $(TOKEN)
