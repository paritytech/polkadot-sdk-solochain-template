// chain_spec.rs
// Correct for polkadot-sdk-solochain-template with sc-chain-spec 42.x
// Uses ChainSpecBuilder API (from_genesis was removed)

use sc_consensus_grandpa::AuthorityId as GrandpaId;
use sc_service::ChainType;
use sc_telemetry::serde_json;
use solochain_template_runtime::{
    AccountId,

    Signature,WASM_BINARY,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

// NEW API: ChainSpec uses () as extension type
pub type ChainSpec = sc_service::GenericChainSpec;

// -------------------------------------------------------
// Helpers
// -------------------------------------------------------
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

// Each POA validator needs:
//   AuraId    → block production (round-robin)
//   GrandpaId → block finalization (voting)
fn authority_keys_from_seed(seed: &str) -> (AuraId, GrandpaId) {
    (
        get_from_seed::<AuraId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

// -------------------------------------------------------
// DEVELOPMENT CHAIN — single node (Alice only)
// cargo run -- --chain dev
// -------------------------------------------------------
pub fn development_chain_spec() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("POA Voting Dev")
    .with_id("poa_voting_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
        ],
        true,
    ))
    .build())
}

// -------------------------------------------------------
// LOCAL POA TESTNET — 2 nodes on same machine
// Node 1 (Alice): terminal 1, port 30333, rpc 9944
// Node 2 (Bob):   terminal 2, port 30334, rpc 9945
// cargo run -- --chain local
// -------------------------------------------------------
pub fn local_chain_spec() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("POA Voting Local Testnet")
    .with_id("poa_voting_local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_patch(testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"), // Node 1 — your terminal 1
            authority_keys_from_seed("Bob"),   // Node 2 — your terminal 2
        ],
        // Alice = sudo (can start/end election, trigger tally)
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts (for paying tx fees)
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"), // Test voter 1
            get_account_id_from_seed::<sr25519::Public>("Dave"),    // Test voter 2
            get_account_id_from_seed::<sr25519::Public>("Eve"),     // Test voter 3
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),  // Test voter 4
        ],
        true,
    ))
    .build())
}

// -------------------------------------------------------
// GENESIS CONFIG BUILDER
// Uses serde_json patch format (new API requirement)
// -------------------------------------------------------
fn testnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> serde_json::Value {
    serde_json::json!({
        // Balances — each test account gets 1000 tokens
        "balances": {
            "balances": endowed_accounts
                .iter()
                .map(|k| (k.clone(), 1_000_000_000_000_000u128))
                .collect::<Vec<_>>(),
        },

        // Aura — POA block production
        // Round-robin: Alice → Bob → Alice → Bob...
        "aura": {
            "authorities": initial_authorities
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>(),
        },

        // Grandpa — POA block finalization
        // With 2 validators (weight=1 each): BOTH must agree to finalize
        // If one node goes offline → blocks produced but NOT finalized
        "grandpa": {
            "authorities": initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1u64))
                .collect::<Vec<_>>(),
        },

        // Sudo — Alice controls admin election functions
        "sudo": {
            "key": root_key,
        },
    })
}