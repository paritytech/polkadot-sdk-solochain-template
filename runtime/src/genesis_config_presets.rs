// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	AccountId, AssetsConfig, BalancesConfig, RuntimeGenesisConfig, SudoConfig, UNIT,
};
use alloc::{vec, vec::Vec};
use frame_support::build_struct_json_patch;
use hex_literal::hex;
use serde_json::Value;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{ed25519, sr25519};
use sp_genesis_builder::{self, PresetId};
use sp_keyring::Sr25519Keyring;

/// Preset id for the new mainnet-v2 genesis.
pub const MAINNET_RUNTIME_PRESET: &str = "mainnet_v2";

// ─── Plim Chain Mainnet v2 — hard-coded genesis keys ────────────────────────
// Generated 2026-04-14 via `plim-node key generate`.
// Mnemonics for these keys live encrypted at
//   /root/SECURE/mainnet_v2_keys_2026-04-14.txt (chmod 600, root-only).
// Only public keys are embedded in the runtime.

/// Sudo account (genesis bootstrap only — to be removed once governance is live).
const MAINNET_SUDO: [u8; 32] =
	hex!("9e6bc7c31fbe6548d1e81521b7c04336e1a21625617d4438484f8860d9045f4e");

/// Treasury account (100M PLIM initial allocation).
const MAINNET_TREASURY: [u8; 32] =
	hex!("d0171a4227eb294218bc0d14358036805e3dccd38a1c7b5318a8ab6388390860");

/// Team vesting account (50M PLIM initial allocation, subject to vesting).
const MAINNET_TEAM_VESTING: [u8; 32] =
	hex!("4a236832e159a220d47c3d364f83d6ef9e74162326c8686d894343c31dcd8d53");

/// Genesis Aura authority (sr25519 public key).
const MAINNET_AURA: [u8; 32] =
	hex!("445ccf1067803f9ffb98e6d36e82d47e31732ed63699de649404d8d739ffe82a");

/// Genesis GRANDPA authority (ed25519 public key).
const MAINNET_GRANDPA: [u8; 32] =
	hex!("1b140e84cc31a49f57076ae46b8338cfcaec66f762e31275e49fc325c349c6eb");

/// Endowment for test accounts: 1,000,000 PLIM each.
const ENDOWMENT: u128 = 1_000_000 * UNIT;

// Returns the genesis config presets populated with given parameters.
fn testnet_genesis(
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
) -> Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect::<Vec<_>>(),
		},
		aura: pallet_aura::GenesisConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>(),
		},
		grandpa: pallet_grandpa::GenesisConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect::<Vec<_>>(),
		},
		sudo: SudoConfig { key: Some(root) },
	})
}

/// Return the development genesis config.
pub fn development_config_genesis() -> Value {
	testnet_genesis(
		vec![(
			sp_keyring::Sr25519Keyring::Alice.public().into(),
			sp_keyring::Ed25519Keyring::Alice.public().into(),
		)],
		vec![
			Sr25519Keyring::Alice.to_account_id(),
			Sr25519Keyring::Bob.to_account_id(),
			Sr25519Keyring::AliceStash.to_account_id(),
			Sr25519Keyring::BobStash.to_account_id(),
		],
		sp_keyring::Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Return the local genesis config preset.
pub fn local_config_genesis() -> Value {
	testnet_genesis(
		vec![
			(
				sp_keyring::Sr25519Keyring::Alice.public().into(),
				sp_keyring::Ed25519Keyring::Alice.public().into(),
			),
			(
				sp_keyring::Sr25519Keyring::Bob.public().into(),
				sp_keyring::Ed25519Keyring::Bob.public().into(),
			),
		],
		Sr25519Keyring::iter()
			.filter(|v| v != &Sr25519Keyring::One && v != &Sr25519Keyring::Two)
			.map(|v| v.to_account_id())
			.collect::<Vec<_>>(),
		Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Build the production mainnet-v2 genesis.
///
/// This is a *clean* genesis with NO Alice / dev sudo. The sudo, treasury,
/// team vesting and validator session keys are all freshly generated keys
/// whose mnemonics are stored encrypted off-chain.
///
/// Token allocation — total supply 1,000,000,000 PLIM at genesis:
///   - Treasury     : 600,000,000 PLIM (60%, governance-managed)
///   - Team vesting : 200,000,000 PLIM (20%, vesting wired in v3)
///   - Sudo         :       1,000 PLIM (gas-only — not a treasury)
///   - Reserve      : 200,000,000 PLIM (20%, NOT minted at genesis — issued
///                                       post-launch via treasury proposals
///                                       for airdrops, validator rewards
///                                       and ecosystem grants)
///
/// Asset catalog (created in `pallet-assets`):
///   id=1 ePL    "ePLIM Staking"     12 dec
///   id=2 gPLIM  "Plim Governance"   12 dec
///   id=3 pEUR   "Plim Euro"          6 dec
///   id=4 pUSD   "Plim US Dollar"     6 dec
///
/// Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations
pub fn mainnet_genesis() -> Value {
	// SAFETY: these public keys come from `plim-node key generate` and are
	// the canonical 32-byte sr25519 / ed25519 representations.
	// `AccountId` is `AccountId32`, which implements `From<[u8; 32]>`.
	let sudo: AccountId = AccountId::from(MAINNET_SUDO);
	let treasury: AccountId = AccountId::from(MAINNET_TREASURY);
	let team: AccountId = AccountId::from(MAINNET_TEAM_VESTING);

	let aura_id: AuraId = sr25519::Public::from_raw(MAINNET_AURA).into();
	let grandpa_id: GrandpaId = ed25519::Public::from_raw(MAINNET_GRANDPA).into();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// 60% — protocol treasury (governance-managed).
				(treasury.clone(), 600_000_000 * UNIT),
				// 20% — team & contributors (vesting via pallet_vesting in v3).
				(team.clone(), 200_000_000 * UNIT),
				// gas-only for sudo to send extrinsics during bootstrap.
				(sudo.clone(), 1_000 * UNIT),
				// Remaining 20% (200M) is reserved for: airdrops, validator
				// rewards and ecosystem grants — added post-genesis via
				// treasury proposals, NOT minted here.
			],
		},
		aura: pallet_aura::GenesisConfig {
			authorities: vec![aura_id],
		},
		grandpa: pallet_grandpa::GenesisConfig {
			authorities: vec![(grandpa_id, 1)],
		},
		sudo: SudoConfig { key: Some(sudo.clone()) },
		assets: AssetsConfig {
			// (id, owner, is_sufficient, min_balance)
			assets: vec![
				(1, sudo.clone(), true, 1),
				(2, sudo.clone(), true, 1),
				(3, sudo.clone(), true, 1),
				(4, sudo.clone(), true, 1),
			],
			// (id, name, symbol, decimals)
			metadata: vec![
				(1, b"ePLIM Staking".to_vec(), b"ePL".to_vec(), 12),
				(2, b"Plim Governance".to_vec(), b"gPLIM".to_vec(), 12),
				(3, b"Plim Euro".to_vec(), b"pEUR".to_vec(), 6),
				(4, b"Plim US Dollar".to_vec(), b"pUSD".to_vec(), 6),
			],
			accounts: vec![],
			next_asset_id: None,
		},
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_config_genesis(),
		MAINNET_RUNTIME_PRESET => mainnet_genesis(),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
		PresetId::from(MAINNET_RUNTIME_PRESET),
	]
}
