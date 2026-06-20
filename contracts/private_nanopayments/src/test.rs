#![cfg(test)]

use super::PrivateNanopayments;
use soroban_sdk::{
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    Address, Bytes, BytesN, Env,
};

fn create_token<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(env, &sac.address()),
        StellarAssetClient::new(env, &sac.address()),
    )
}

struct Setup<'a> {
    env: Env,
    contract: Address,
    token: TokenClient<'a>,
    sender: Address,
}

fn setup() -> Setup<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token, token_admin) = create_token(&env, &admin);
    let sender = Address::generate(&env);
    token_admin.mint(&sender, &1_000_000);

    let contract = env.register(PrivateNanopayments, ());
    let client = PrivateNanopaymentsClient::new(&env, &contract);
    client.initialize(&token.address);

    Setup { env, contract, token, sender }
}

fn valid_proof(env: &Env) -> Bytes {
    Bytes::from_array(env, &[1u8])
}

fn invalid_proof(env: &Env) -> Bytes {
    Bytes::from_array(env, &[0u8])
}

#[test]
fn test_happy_path_deposit_then_claim() {
    let s = setup();
    let client = PrivateNanopaymentsClient::new(&s.env, &s.contract);

    let commitment = BytesN::from_array(&s.env, &[7u8; 32]);
    let nullifier = BytesN::from_array(&s.env, &[9u8; 32]);
    let recipient = Address::generate(&s.env);

    client.deposit(&s.sender, &500, &commitment);
    client.claim(&valid_proof(&s.env), &commitment, &nullifier, &recipient);

    assert_eq!(s.token.balance(&recipient), 500);
    assert_eq!(s.token.balance(&s.contract), 0);
}

#[test]
#[should_panic(expected = "nullifier already used")]
fn test_replayed_nullifier_rejected() {
    let s = setup();
    let client = PrivateNanopaymentsClient::new(&s.env, &s.contract);

    let commitment_a = BytesN::from_array(&s.env, &[1u8; 32]);
    let commitment_b = BytesN::from_array(&s.env, &[2u8; 32]);
    let nullifier = BytesN::from_array(&s.env, &[3u8; 32]);
    let recipient = Address::generate(&s.env);

    client.deposit(&s.sender, &100, &commitment_a);
    client.deposit(&s.sender, &100, &commitment_b);

    client.claim(&valid_proof(&s.env), &commitment_a, &nullifier, &recipient);
    // Reusing the same nullifier against a different commitment must still be rejected.
    client.claim(&valid_proof(&s.env), &commitment_b, &nullifier, &recipient);
}

#[test]
fn test_storage_updated_post_claim() {
    let s = setup();
    let client = PrivateNanopaymentsClient::new(&s.env, &s.contract);

    let commitment = BytesN::from_array(&s.env, &[4u8; 32]);
    let nullifier = BytesN::from_array(&s.env, &[5u8; 32]);
    let recipient = Address::generate(&s.env);

    client.deposit(&s.sender, &250, &commitment);
    client.claim(&valid_proof(&s.env), &commitment, &nullifier, &recipient);

    // Re-claiming the now-cleared commitment with a fresh nullifier must fail to find escrow.
    let other_nullifier = BytesN::from_array(&s.env, &[6u8; 32]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim(&valid_proof(&s.env), &commitment, &other_nullifier, &recipient);
    }));
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "invalid proof")]
fn test_invalid_proof_rejected() {
    let s = setup();
    let client = PrivateNanopaymentsClient::new(&s.env, &s.contract);

    let commitment = BytesN::from_array(&s.env, &[8u8; 32]);
    let nullifier = BytesN::from_array(&s.env, &[10u8; 32]);
    let recipient = Address::generate(&s.env);

    client.deposit(&s.sender, &100, &commitment);
    client.claim(&invalid_proof(&s.env), &commitment, &nullifier, &recipient);
}

#[test]
#[should_panic]
fn test_unauthorized_deposit_rejected() {
    let env = Env::default();
    // Auths are NOT mocked here, so `from.require_auth()` must panic.
    let admin = Address::generate(&env);
    let (token, _token_admin) = create_token(&env, &admin);
    let sender = Address::generate(&env);

    let contract = env.register(PrivateNanopayments, ());
    let client = PrivateNanopaymentsClient::new(&env, &contract);
    client.initialize(&token.address);

    let commitment = BytesN::from_array(&env, &[11u8; 32]);
    client.deposit(&sender, &100, &commitment);
}
