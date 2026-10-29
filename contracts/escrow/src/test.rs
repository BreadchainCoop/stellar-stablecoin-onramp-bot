#![cfg(test)]
extern crate std;

use super::{Error, Escrow, EscrowClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, MockAuth, MockAuthInvoke},
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

    let escrow_id = env.register(Escrow, (&owner, &token_addr));
    let escrow = EscrowClient::new(&env, &escrow_id);

    (env, escrow, owner, escrow_id, token, token_admin)
}

#[test]
fn constructor_sets_owner_and_token() {
    let (_env, escrow, owner, _id, token, _admin) = setup();
    assert_eq!(escrow.owner(), owner);
    assert_eq!(escrow.token(), token.address);
    assert_eq!(escrow.balance(), 0);
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
fn deposit_is_authorized_by_funder_including_token_transfer() {
    let (env, escrow, _owner, escrow_id, token, token_admin) = setup();
    let funder = Address::generate(&env);
    token_admin.mint(&funder, &1_000);

    escrow.deposit(&funder, &1_000);
    assert_eq!(
        env.auths(),
        std::vec![(
            funder.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    escrow_id.clone(),
                    symbol_short!("deposit"),
                    (&funder, 1_000_i128).into_val(&env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        token.address.clone(),
                        symbol_short!("transfer"),
                        (&funder, &escrow_id, 1_000_i128).into_val(&env),
                    )),
                    sub_invocations: std::vec![],
                }],
            }
        )]
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_without_funder_signature_fails() {
    let (env, escrow, _owner, escrow_id, _token, token_admin) = setup();
    let victim = Address::generate(&env);
    let attacker = Address::generate(&env);
    token_admin.mint(&victim, &1_000);

    escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "deposit",
                args: (&victim, 1_000_i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deposit(&victim, &1_000);
}

#[test]
fn release_is_authorized_by_owner_only() {
    let (env, escrow, owner, escrow_id, _token, token_admin) = setup();
    let recipient = Address::generate(&env);
    token_admin.mint(&escrow_id, &1_000);

    escrow.release(&recipient, &400);
    assert_eq!(
        env.auths(),
        std::vec![(
            owner,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    escrow_id.clone(),
                    symbol_short!("release"),
                    (&recipient, 400_i128).into_val(&env),
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn release_signed_by_non_owner_fails() {
    let (env, escrow, _owner, escrow_id, _token, token_admin) = setup();
    let attacker = Address::generate(&env);
    token_admin.mint(&escrow_id, &1_000);

    escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "release",
                args: (&attacker, 1_000_i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .release(&attacker, &1_000);
}

#[test]
fn refund_is_authorized_by_owner_only() {
    let (env, escrow, owner, escrow_id, _token, token_admin) = setup();
    token_admin.mint(&escrow_id, &1_000);

    escrow.refund(&500);
    assert_eq!(
        env.auths(),
        std::vec![(
            owner,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    escrow_id.clone(),
                    symbol_short!("refund"),
                    (500_i128,).into_val(&env),
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn refund_signed_by_non_owner_fails() {
    let (env, escrow, _owner, escrow_id, _token, token_admin) = setup();
    let attacker = Address::generate(&env);
    token_admin.mint(&escrow_id, &1_000);

    escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "refund",
                args: (1_000_i128,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .refund(&1_000);
}
