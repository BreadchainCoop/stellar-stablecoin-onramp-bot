#![cfg(test)]
extern crate std;

// Behaviour tree (BTT); test names follow it.
//
// __constructor
// - It: stores owner and token; the pool starts empty
// - It: emits Initialized { owner, token }
// deposit(from, amount)
// - When: amount <= 0 -> InvalidAmount
// - Given: `from` signed -> moves amount into the pool; auth: from -> deposit -> token.transfer; emits Deposit
// - Given: another address signed -> Error(Auth, InvalidAction)
// release(recipient, amount)
// - Given: a non-owner signed -> Error(Auth, InvalidAction)
// - Given: the owner signed
//   - When: amount <= 0 -> InvalidAmount
//   - When: amount > pool -> InsufficientBalance
//   - When: amount == pool -> recipient receives it all; auth: owner -> release, no subs; emits Release
// refund(amount)
// - Given: a non-owner signed -> Error(Auth, InvalidAction)
// - Given: the owner signed
//   - When: amount <= 0 -> InvalidAmount
//   - When: amount > pool -> InsufficientBalance
//   - When: amount == pool -> owner receives it all; auth: owner -> refund, no subs; emits Refund

use super::{Deposit, Error, Escrow, EscrowClient, Initialized, Refund, Release};
use soroban_sdk::{
    symbol_short,
    testutils::{
        Address as _, AuthorizedFunction, AuthorizedInvocation, Events as _, MockAuth,
        MockAuthInvoke,
    },
    token::{StellarAssetClient, TokenClient},
    Address, Env, Event as _, IntoVal,
};

struct Setup<'a> {
    env: Env,
    escrow: EscrowClient<'a>,
    id: Address,
    owner: Address,
    /// A real SAC standing in for USDC.
    token: TokenClient<'a>,
    token_admin: StellarAssetClient<'a>,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let id = env.register(Escrow, (&owner, sac.address()));

    Setup {
        escrow: EscrowClient::new(&env, &id),
        token: TokenClient::new(&env, &sac.address()),
        token_admin: StellarAssetClient::new(&env, &sac.address()),
        env,
        id,
        owner,
    }
}

#[test]
fn constructor_stores_owner_and_token_with_an_empty_pool() {
    let s = setup();
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.id),
        [Initialized {
            owner: s.owner.clone(),
            token: s.token.address.clone(),
        }
        .to_xdr(&s.env, &s.id)]
    );
    assert_eq!(s.escrow.owner(), s.owner);
    assert_eq!(s.escrow.token(), s.token.address);
    assert_eq!(s.escrow.balance(), 0);
}

#[test]
fn deposit_when_amount_is_not_positive_fails_with_invalid_amount() {
    let s = setup();
    let funder = Address::generate(&s.env);
    for amount in [0, -1] {
        assert_eq!(
            s.escrow.try_deposit(&funder, &amount),
            Err(Ok(Error::InvalidAmount))
        );
    }
}

#[test]
fn deposit_given_funder_signed_moves_amount_into_pool() {
    let s = setup();
    let funder = Address::generate(&s.env);
    s.token_admin.mint(&funder, &1_000);

    s.escrow.deposit(&funder, &1_000);
    assert_eq!(
        s.env.auths(),
        std::vec![(
            funder.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s.id.clone(),
                    symbol_short!("deposit"),
                    (&funder, 1_000_i128).into_val(&s.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        s.token.address.clone(),
                        symbol_short!("transfer"),
                        (&funder, &s.id, 1_000_i128).into_val(&s.env),
                    )),
                    sub_invocations: std::vec![],
                }],
            }
        )]
    );
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.id),
        [Deposit {
            from: funder.clone(),
            amount: 1_000,
        }
        .to_xdr(&s.env, &s.id)]
    );
    assert_eq!(s.escrow.balance(), 1_000);
    assert_eq!(s.token.balance(&funder), 0);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_given_another_signer_fails() {
    let s = setup();
    let victim = Address::generate(&s.env);
    let attacker = Address::generate(&s.env);
    s.token_admin.mint(&victim, &1_000);

    s.escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &s.id,
                fn_name: "deposit",
                args: (&victim, 1_000_i128).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .deposit(&victim, &1_000);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn release_given_non_owner_signer_fails() {
    let s = setup();
    let attacker = Address::generate(&s.env);
    s.token_admin.mint(&s.id, &1_000);

    s.escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &s.id,
                fn_name: "release",
                args: (&attacker, 1_000_i128).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .release(&attacker, &1_000);
}

#[test]
fn release_when_amount_is_not_positive_fails_with_invalid_amount() {
    let s = setup();
    let recipient = Address::generate(&s.env);
    for amount in [0, -1] {
        assert_eq!(
            s.escrow.try_release(&recipient, &amount),
            Err(Ok(Error::InvalidAmount))
        );
    }
}

#[test]
fn release_when_amount_exceeds_pool_fails_with_insufficient_balance() {
    let s = setup();
    let recipient = Address::generate(&s.env);
    s.token_admin.mint(&s.id, &100);

    assert_eq!(
        s.escrow.try_release(&recipient, &101),
        Err(Ok(Error::InsufficientBalance))
    );
}

#[test]
fn release_when_amount_equals_pool_pays_recipient_in_full() {
    let s = setup();
    let recipient = Address::generate(&s.env);
    s.token_admin.mint(&s.id, &1_000);

    s.escrow.release(&recipient, &1_000);
    assert_eq!(
        s.env.auths(),
        std::vec![(
            s.owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s.id.clone(),
                    symbol_short!("release"),
                    (&recipient, 1_000_i128).into_val(&s.env),
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.id),
        [Release {
            recipient: recipient.clone(),
            amount: 1_000,
        }
        .to_xdr(&s.env, &s.id)]
    );
    assert_eq!(s.token.balance(&recipient), 1_000);
    assert_eq!(s.escrow.balance(), 0);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn refund_given_non_owner_signer_fails() {
    let s = setup();
    let attacker = Address::generate(&s.env);
    s.token_admin.mint(&s.id, &1_000);

    s.escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &s.id,
                fn_name: "refund",
                args: (1_000_i128,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .refund(&1_000);
}

#[test]
fn refund_when_amount_is_not_positive_fails_with_invalid_amount() {
    let s = setup();
    for amount in [0, -1] {
        assert_eq!(s.escrow.try_refund(&amount), Err(Ok(Error::InvalidAmount)));
    }
}

#[test]
fn refund_when_amount_exceeds_pool_fails_with_insufficient_balance() {
    let s = setup();
    s.token_admin.mint(&s.id, &100);

    assert_eq!(
        s.escrow.try_refund(&101),
        Err(Ok(Error::InsufficientBalance))
    );
}

#[test]
fn refund_when_amount_equals_pool_pays_owner_in_full() {
    let s = setup();
    s.token_admin.mint(&s.id, &1_000);

    s.escrow.refund(&1_000);
    assert_eq!(
        s.env.auths(),
        std::vec![(
            s.owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s.id.clone(),
                    symbol_short!("refund"),
                    (1_000_i128,).into_val(&s.env),
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.id),
        [Refund { amount: 1_000 }.to_xdr(&s.env, &s.id)]
    );
    assert_eq!(s.token.balance(&s.owner), 1_000);
    assert_eq!(s.escrow.balance(), 0);
}
