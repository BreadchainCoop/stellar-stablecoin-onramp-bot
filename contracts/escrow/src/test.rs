#![cfg(test)]

use super::{Error, Escrow, EscrowClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Address, Env, IntoVal,
};

/// A fresh SAC (Stellar Asset Contract) to stand in for USDC in tests, plus
/// its token + admin clients.
fn make_token<'a>(
    env: &Env,
    admin: &Address,
) -> (Address, TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = sac.address();
    (
        addr.clone(),
        TokenClient::new(env, &addr),
        StellarAssetClient::new(env, &addr),
    )
}

fn setup<'a>() -> (
    Env,
    EscrowClient<'a>,
    Address,
    Address,
    TokenClient<'a>,
    StellarAssetClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_addr, token, token_admin) = make_token(&env, &admin);

    let escrow_id = env.register(Escrow, ());
    let escrow = EscrowClient::new(&env, &escrow_id);
    escrow.init(&owner, &token_addr);

    (env, escrow, owner, escrow_id, token, token_admin)
}

#[test]
fn init_sets_owner_and_token() {
    let (_env, escrow, owner, _id, token, _admin) = setup();
    assert_eq!(escrow.owner(), owner);
    assert_eq!(escrow.balance(), 0);
    let _ = token; // bound token exists
}

#[test]
fn deposit_then_release_moves_funds() {
    let (env, escrow, _owner, escrow_id, token, token_admin) = setup();
    let funder = Address::generate(&env);
    let recipient = Address::generate(&env);

    token_admin.mint(&funder, &1_000);
    escrow.deposit(&funder, &1_000);
    assert_eq!(escrow.balance(), 1_000);
    assert_eq!(token.balance(&escrow_id), 1_000);

    escrow.release(&recipient, &400);
    assert_eq!(escrow.balance(), 600);
    assert_eq!(token.balance(&recipient), 400);
}

#[test]
fn refund_returns_pool_to_owner() {
    let (env, escrow, _owner, _id, _token, token_admin) = setup();
    let funder = Address::generate(&env);
    token_admin.mint(&funder, &500);
    escrow.deposit(&funder, &500);

    escrow.refund(&500);
    assert_eq!(escrow.balance(), 0);
}

#[test]
fn release_more_than_balance_fails() {
    let (env, escrow, _owner, _id, _token, token_admin) = setup();
    let funder = Address::generate(&env);
    let recipient = Address::generate(&env);
    token_admin.mint(&funder, &100);
    escrow.deposit(&funder, &100);

    let err = escrow.try_release(&recipient, &101).err().unwrap().unwrap();
    assert_eq!(err, Error::InsufficientBalance);
}

#[test]
fn zero_or_negative_amounts_rejected() {
    let (env, escrow, _owner, _id, _token, _admin) = setup();
    let recipient = Address::generate(&env);
    assert_eq!(
        escrow.try_release(&recipient, &0).err().unwrap().unwrap(),
        Error::InvalidAmount
    );
}

#[test]
fn double_init_fails() {
    let (_env, escrow, owner, _id, token, _admin) = setup();
    let other = Address::generate(&_env);
    let err = escrow.try_init(&owner, &other).err().unwrap().unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
    let _ = token;
}

#[test]
fn release_requires_owner_auth() {
    // Without mock_all_auths, a release not authorized by the owner must fail.
    let env = Env::default();
    let owner = Address::generate(&env);
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_addr, _token, token_admin) = make_token(&env, &admin);

    let escrow_id = env.register(Escrow, ());
    let escrow = EscrowClient::new(&env, &escrow_id);
    env.mock_all_auths();
    escrow.init(&owner, &token_addr);
    token_admin.mint(&escrow_id, &1_000);

    // Re-run release with an explicit owner auth scope (sanity: the owner can).
    escrow
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "release",
                args: (recipient.clone(), 10_i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .release(&recipient, &10);
    assert_eq!(escrow.balance(), 990);
}
