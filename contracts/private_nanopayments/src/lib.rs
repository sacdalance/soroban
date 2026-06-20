#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, BytesN, Env};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Token,                    // address of the USDC SAC used for escrow
    Commitment(BytesN<32>),   // commitment -> escrowed amount
    Nullifier(BytesN<32>),    // spent-nullifier marker
}

#[contract]
pub struct PrivateNanopayments;

#[contractimpl]
impl PrivateNanopayments {
    // Why: token address must never be hardcoded (§6.6) so it is supplied once at deployment.
    pub fn initialize(env: Env, token: Address) {
        if env.storage().instance().has(&DataKey::Token) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Token, &token);
    }

    // Why: escrows funds under an opaque commitment so the deposit reveals no recipient identity.
    pub fn deposit(env: Env, from: Address, amount: u128, commitment: BytesN<32>) {
        from.require_auth();

        if amount == 0 {
            panic!("amount must be > 0");
        }
        if env.storage().persistent().has(&DataKey::Commitment(commitment.clone())) {
            panic!("commitment already used");
        }

        let token_client = token::Client::new(&env, &Self::token_address(&env));
        token_client.transfer(&from, &env.current_contract_address(), &(amount as i128));

        env.storage()
            .persistent()
            .set(&DataKey::Commitment(commitment.clone()), &amount);

        env.events().publish((symbol_short!("deposit"), commitment), amount);
    }

    // Why: anyone may relay a valid proof on behalf of the recipient (§6.4) — only proof validity
    // and nullifier freshness gate the payout, never the identity of the caller.
    //
    // `commitment` is required even though it is a public ZK input: the proof only proves a
    // relation between commitment and nullifier, it does not let the contract recover which
    // escrow entry to pay out from without being told explicitly.
    pub fn claim(env: Env, proof: Bytes, commitment: BytesN<32>, nullifier: BytesN<32>, recipient: Address) {
        // 1. check — nullifier must be unused before any other effect (§6.1, §6.5).
        if env.storage().persistent().has(&DataKey::Nullifier(nullifier.clone())) {
            panic!("nullifier already used");
        }

        // 2. verify — proof must bind this nullifier to this commitment (§6.2, §6.3).
        if !Self::verify_proof(&env, &proof, &commitment, &nullifier) {
            panic!("invalid proof");
        }

        let amount: u128 = env
            .storage()
            .persistent()
            .get(&DataKey::Commitment(commitment.clone()))
            .expect("commitment not found");

        // 3. effects before interactions — mark spent and clear escrow before the transfer (§6.5).
        env.storage().persistent().set(&DataKey::Nullifier(nullifier.clone()), &true);
        env.storage().persistent().remove(&DataKey::Commitment(commitment.clone()));

        // 4. interaction — release funds last.
        let token_client = token::Client::new(&env, &Self::token_address(&env));
        token_client.transfer(&env.current_contract_address(), &recipient, &(amount as i128));

        env.events().publish((symbol_short!("claim"), nullifier), recipient);
    }

    fn token_address(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("token not initialized")
    }

    // TODO: replace with real BN254 Groth16 verify (Protocol 25 host fn). This stub checks a
    // single sentinel byte so tests can exercise both the accept and reject paths deterministically.
    fn verify_proof(_env: &Env, proof: &Bytes, _commitment: &BytesN<32>, _nullifier: &BytesN<32>) -> bool {
        proof.len() == 1 && proof.get(0) == Some(1u8)
    }
}

mod test;
