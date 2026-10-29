#![no_std]
//! Pooled-custody USDC escrow: deposit, owner-only release and withdraw.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contractmeta, token, Address, Env,
};

mod storage;

// Labels shown by explorers; not a source verification. The release workflow
// injects `source_repo` (github:<owner>/<repo>), which StellarExpert uses to link
// the attested wasm to its source, so it isn't embedded here (docs/TESTNET.md §6).
contractmeta!(key = "name", val = "stellar-onramp-escrow");
contractmeta!(
    key = "repo",
    val = "https://github.com/BreadchainCoop/stellar-stablecoin-onramp-bot"
);

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // Codes 1–99 belong to the escrow; give any new contract its own range.
    // 1 was AlreadyInitialized (public init, removed); never reuse.
    NotInitialized = 2,
    InvalidAmount = 3,
    InsufficientBalance = 4,
    InvalidRecipient = 5,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    #[topic]
    pub owner: Address,
    pub token: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposit {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    #[topic]
    pub recipient: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdraw {
    pub amount: i128,
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Pins the owner (operator) and the USDC token. Runs once, atomically with
    /// deployment, so the setup cannot be front-run.
    pub fn __constructor(env: Env, owner: Address, token: Address) {
        storage::set_owner(&env, &owner);
        storage::set_token(&env, &token);
        Initialized { owner, token }.publish(&env);
    }

    /// Anyone may fund the pool by moving `amount` of the token into the escrow.
    /// `from` must authorize the transfer.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), Error> {
        storage::extend_instance(&env);
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        from.require_auth();
        let token = storage::get_token(&env)?;
        token::Client::new(&env, &token).transfer(&from, env.current_contract_address(), &amount);
        Deposit { from, amount }.publish(&env);
        Ok(())
    }

    /// Owner-only: release `amount` from the pool to `recipient` (the buyer's
    /// pinned payout address). The owner must authorize the call.
    pub fn release(env: Env, recipient: Address, amount: i128) -> Result<(), Error> {
        storage::extend_instance(&env);
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if recipient == env.current_contract_address() {
            return Err(Error::InvalidRecipient);
        }
        Self::require_owner(&env)?;
        let token = storage::get_token(&env)?;
        let client = token::Client::new(&env, &token);
        if client.balance(&env.current_contract_address()) < amount {
            return Err(Error::InsufficientBalance);
        }
        client.transfer(&env.current_contract_address(), &recipient, &amount);
        Release { recipient, amount }.publish(&env);
        Ok(())
    }

    /// Owner-only: move `amount` of pooled funds to the owner (treasury
    /// withdrawal). The owner must authorize.
    pub fn withdraw(env: Env, amount: i128) -> Result<(), Error> {
        storage::extend_instance(&env);
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let owner = Self::require_owner(&env)?;
        let token = storage::get_token(&env)?;
        let client = token::Client::new(&env, &token);
        if client.balance(&env.current_contract_address()) < amount {
            return Err(Error::InsufficientBalance);
        }
        client.transfer(&env.current_contract_address(), &owner, &amount);
        Withdraw { amount }.publish(&env);
        Ok(())
    }

    /// Current pooled token balance held by the escrow.
    pub fn balance(env: Env) -> Result<i128, Error> {
        let token = storage::get_token(&env)?;
        Ok(token::Client::new(&env, &token).balance(&env.current_contract_address()))
    }

    /// The operator/owner address.
    pub fn owner(env: Env) -> Result<Address, Error> {
        storage::get_owner(&env)
    }

    /// The token (USDC) address this escrow is bound to.
    pub fn token(env: Env) -> Result<Address, Error> {
        storage::get_token(&env)
    }

    fn require_owner(env: &Env) -> Result<Address, Error> {
        let owner = storage::get_owner(env)?;
        owner.require_auth();
        Ok(owner)
    }
}

mod test;
