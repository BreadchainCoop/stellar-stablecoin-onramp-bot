#![cfg(test)]
extern crate std;

// Behaviour tree (BTT); test names follow it.
//
// __constructor
// - It: stores owner and token; the pool starts empty
// - It: emits Initialized { owner, token }
// deposit(from, amount)
// - When: amount <= 0 -> InvalidAmount, before any auth
// - Given: `from` signed -> moves amount into the pool; auth: from -> deposit -> token.transfer; emits Deposit
// - Given: another address signed -> Error(Auth, InvalidAction)
// release(recipient, amount)
// - When: amount <= 0 -> InvalidAmount, before any auth
// - When: recipient is the escrow -> InvalidRecipient, before any auth
// - Given: a non-owner signed -> Error(Auth, InvalidAction)
// - Given: the owner signed
//   - When: amount > pool -> InsufficientBalance
//   - When: amount == pool -> recipient receives it all; auth: owner -> release, no subs; emits Release
// withdraw(amount)
// - When: amount <= 0 -> InvalidAmount, before any auth
// - Given: a non-owner signed -> Error(Auth, InvalidAction)
// - Given: the owner signed
//   - When: amount > pool -> InsufficientBalance
//   - When: amount == pool -> owner receives it all; auth: owner -> withdraw, no subs; emits Withdraw
// deposit / release / withdraw
// - It: extends the instance TTL to INSTANCE_EXTEND_TO

use super::{
    storage::{DAY_IN_LEDGERS, INSTANCE_EXTEND_TO},
    Deposit, Error, Escrow, EscrowClient, Initialized, Release, Withdraw,
};
use soroban_sdk::{
    symbol_short,
    testutils::{
        storage::Instance as _, Address as _, AuthorizedFunction, AuthorizedInvocation,
        Events as _, Ledger as _, MockAuth, MockAuthInvoke,
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
            s.escrow.mock_auths(&[]).try_deposit(&funder, &amount),
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
            s.escrow.mock_auths(&[]).try_release(&recipient, &amount),
            Err(Ok(Error::InvalidAmount))
        );
    }
}

#[test]
fn release_when_recipient_is_the_escrow_fails_with_invalid_recipient() {
    let s = setup();
    s.token_admin.mint(&s.id, &1_000);

    assert_eq!(
        s.escrow.mock_auths(&[]).try_release(&s.id, &1_000),
        Err(Ok(Error::InvalidRecipient))
    );
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
fn withdraw_given_non_owner_signer_fails() {
    let s = setup();
    let attacker = Address::generate(&s.env);
    s.token_admin.mint(&s.id, &1_000);

    s.escrow
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &s.id,
                fn_name: "withdraw",
                args: (1_000_i128,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .withdraw(&1_000);
}

#[test]
fn withdraw_when_amount_is_not_positive_fails_with_invalid_amount() {
    let s = setup();
    for amount in [0, -1] {
        assert_eq!(
            s.escrow.mock_auths(&[]).try_withdraw(&amount),
            Err(Ok(Error::InvalidAmount))
        );
    }
}

#[test]
fn withdraw_when_amount_exceeds_pool_fails_with_insufficient_balance() {
    let s = setup();
    s.token_admin.mint(&s.id, &100);

    assert_eq!(
        s.escrow.try_withdraw(&101),
        Err(Ok(Error::InsufficientBalance))
    );
}

#[test]
fn withdraw_when_amount_equals_pool_pays_owner_in_full() {
    let s = setup();
    s.token_admin.mint(&s.id, &1_000);

    s.escrow.withdraw(&1_000);
    assert_eq!(
        s.env.auths(),
        std::vec![(
            s.owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s.id.clone(),
                    symbol_short!("withdraw"),
                    (1_000_i128,).into_val(&s.env),
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.id),
        [Withdraw { amount: 1_000 }.to_xdr(&s.env, &s.id)]
    );
    assert_eq!(s.token.balance(&s.owner), 1_000);
    assert_eq!(s.escrow.balance(), 0);
}

#[test]
fn state_changing_calls_extend_instance_ttl() {
    let s = setup();
    let funder = Address::generate(&s.env);
    let recipient = Address::generate(&s.env);
    s.token_admin.mint(&funder, &1_000);
    let ttl = || {
        s.env
            .as_contract(&s.id, || s.env.storage().instance().get_ttl())
    };
    // Two days on, the TTL sits below the threshold (EXTEND_TO - 1 day).
    let age = || {
        s.env
            .ledger()
            .with_mut(|l| l.sequence_number += 2 * DAY_IN_LEDGERS)
    };

    s.escrow.deposit(&funder, &1_000);
    assert_eq!(ttl(), INSTANCE_EXTEND_TO);

    age();
    s.escrow.release(&recipient, &400);
    assert_eq!(ttl(), INSTANCE_EXTEND_TO);

    age();
    s.escrow.withdraw(&600);
    assert_eq!(ttl(), INSTANCE_EXTEND_TO);
}
