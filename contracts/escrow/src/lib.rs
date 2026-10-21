#![no_std]
//! Pooled-custody USDC escrow for the Stellar stablecoin on/off ramp.
//!
//! Mirrors the EVM `Escrow.sol` from the original ramp: a single owner (the
//! backend operator) can `release` USDC to a buyer or `refund` pooled funds back
//! to itself. Funds are a shared pool with OFF-CHAIN order accounting — there are
//! no per-order balances enforced on-chain.
//!
//! Token moves go through the Stellar Asset Contract (SAC) interface via
//! `soroban_sdk::token`, so this works with USDC (the SAC-wrapped Circle asset on
//! Stellar) or any Soroban token. Amounts are `i128` stroops of the token's unit.

use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype, symbol_short, token,
    Address, Env,
};

// Source metadata — surfaced by explorers (StellarExpert) and used for source
// verification. Keep in sync with the deployed commit.
contractmeta!(key = "name", val = "stellar-onramp-escrow");
contractmeta!(
    key = "repo",
    val = "https://github.com/BreadchainCoop/stellar-stablecoin-onramp-bot"
);
// `source_repo` in the `github:<owner>/<repo>` form is the key block explorers
// (StellarExpert) read to link the on-chain wasm to its source for verification.
contractmeta!(
    key = "source_repo",
    val = "github:BreadchainCoop/stellar-stablecoin-onramp-bot"
);

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InsufficientBalance = 4,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Owner,
    Token,
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// One-time initialization: pin the owner (operator) and the USDC token
    /// contract address. Fails if already initialized.
    pub fn init(env: Env, owner: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Owner) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Token, &token);
        Ok(())
    }

    /// Anyone may fund the pool by moving `amount` of the token into the escrow.
    /// `from` must authorize the transfer.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        from.require_auth();
        let token = Self::token_address(&env)?;
        token::Client::new(&env, &token).transfer(&from, &env.current_contract_address(), &amount);
        env.events()
            .publish((symbol_short!("deposit"), from), amount);
        Ok(())
    }

    /// Owner-only: release `amount` from the pool to `recipient` (the buyer's
    /// pinned payout address). The owner must authorize the call.
    pub fn release(env: Env, recipient: Address, amount: i128) -> Result<(), Error> {
        let owner = Self::require_owner(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let token = Self::token_address(&env)?;
        let client = token::Client::new(&env, &token);
        if client.balance(&env.current_contract_address()) < amount {
            return Err(Error::InsufficientBalance);
        }
        client.transfer(&env.current_contract_address(), &recipient, &amount);
        env.events()
            .publish((symbol_short!("release"), owner, recipient), amount);
        Ok(())
    }

    /// Owner-only: withdraw `amount` of pooled funds back to the owner
    /// (treasury withdrawal, not a customer refund). The owner must authorize.
    pub fn refund(env: Env, amount: i128) -> Result<(), Error> {
        let owner = Self::require_owner(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let token = Self::token_address(&env)?;
        let client = token::Client::new(&env, &token);
        if client.balance(&env.current_contract_address()) < amount {
            return Err(Error::InsufficientBalance);
        }
        client.transfer(&env.current_contract_address(), &owner, &amount);
        env.events()
            .publish((symbol_short!("refund"), owner), amount);
        Ok(())
    }

    /// Current pooled token balance held by the escrow.
    pub fn balance(env: Env) -> i128 {
        match Self::token_address(&env) {
            Ok(token) => token::Client::new(&env, &token).balance(&env.current_contract_address()),
            Err(_) => 0,
        }
    }

    /// The operator/owner address.
    pub fn owner(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Owner)
            .ok_or(Error::NotInitialized)
    }

    /// The token (USDC) address this escrow is bound to.
    pub fn token(env: Env) -> Result<Address, Error> {
        Self::token_address(&env)
    }

    fn token_address(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .ok_or(Error::NotInitialized)
    }

    fn require_owner(env: &Env) -> Result<Address, Error> {
        let owner: Address = env
            .storage()
            .instance()
            .get(&DataKey::Owner)
            .ok_or(Error::NotInitialized)?;
        owner.require_auth();
        Ok(owner)
    }
}

mod test;
