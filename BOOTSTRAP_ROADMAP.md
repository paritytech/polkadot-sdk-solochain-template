# pLim Chain Bootstrap Roadmap

**Last updated:** 2026-04-14
**Owner:** pLim/lab) Protocol Team
**Scope:** Testnet-first rollout of the full pLim token economy + dApp integrations, culminating in a clean Mainnet v2 promotion.

---

## 1. Executive summary

The pLim Chain is a Substrate L1 that will host the entire pLim/lab) token economy: native **PLIM** (gas), **pEUR** (MiCA-compliant main fiat stablecoin), **pUSD** (secondary fiat stablecoin), **ePL** (staking derivative) and **gPLIM** (governance). Today a single-validator "mainnet v1" on port 9946 runs with Alice sudo — a known critical risk we are retiring — alongside a healthier 5-validator testnet on port 9945. This roadmap bootstraps the economy on testnet first, hardens pEUR for MiCA compliance, wires all internal dApps (plim-gateway, plimcontrol, Stripe billing, Referral, Natali-Virtual, AIFSACCT, Kickstarter, pLim Agent, bridges), and then promotes a fresh **Mainnet v2** with proper genesis, rotated validator keys, no Alice sudo, and 24/7 monitoring.

---

## 2. Token catalog (summary)

See `TOKEN_CATALOG.md` in the same directory for the authoritative reference.

| Symbol | Type                 | Decimals | Pallet             | Asset ID | Role                                  |
|--------|----------------------|----------|--------------------|----------|---------------------------------------|
| PLIM   | Native               | 12       | pallet_balances    | —        | Gas, fees, validator stake            |
| ePL    | Asset                | 12       | pallet_assets      | 1        | Staking derivative                    |
| gPLIM  | Asset                | 12       | pallet_assets      | 2        | Governance voting weight              |
| pEUR   | Asset (MAIN FIAT)    | 6        | pallet_assets      | 3        | MiCA-compliant EUR stablecoin         |
| pUSD   | Asset (SECONDARY)    | 6        | pallet_assets      | 4        | USD stablecoin                        |

---

## 3. Current state of the chain

### Deployed runtime pallets
Indices as they exist in `runtime/src/lib.rs`:

- `0` System
- `1` Timestamp
- `2` Aura
- `3` Grandpa
- `4` Balances
- `5` TransactionPayment
- `6` Sudo
- `7` pallet-template (being removed)
- `10` PlimIdentity
- `11` PlimPayments
- `12` PlimMandates
- `13` PlimChannels
- `14` PlimDelegation
- `15` PlimCompliance
- `16` PlimReputation
- `17` PlimTimestamps

All `Plim*` custom pallets are currently empty 33-line stubs — they declare a config trait and an empty Call enum, nothing else. They compile but do no work.

### Running nodes (on 91.99.60.74)

| Net     | Port | Validators | Head     | Genesis      | Sudo   | Status |
|---------|------|------------|----------|--------------|--------|--------|
| mainnet v1 | 9946 | 1 (Alice) | ~85100   | 0xd13f10b8… | Alice  | RISK — retire |
| testnet    | 9945 | 5 (Alice..Eve) | live | 0xe8399f14… | Alice  | Dev use only |

### What's broken / missing
- `chain-spec-mainnet.json` and `chain-spec-mainnet-raw.json` have been moved out of tree (`.MOVED` suffix) because they referenced Alice sudo. New genesis files will be produced by agent #1 (mainnet v2).
- Custom pallets are empty stubs — no dispatchables, no storage, no events beyond defaults.
- `PlimPayments::pay` does not exist yet (scheduled for Step 3 below).
- Gateway does not yet have a Substrate client (Step 4).
- `pallet-assets` is not in the runtime yet — to be added in spec_version 101 (Step 1).
- Keys evacuated to `/root/SECURE/` on 2026-04-14; no validator keys remain in the repo tree.

---

## 4. Target architecture — Mainnet v2

Mainnet v2 is a **fresh chain** (new genesis hash, new chain_id `plim-mainnet-2`). It is not a runtime upgrade of mainnet v1 — mainnet v1 will be drained and archived.

**Design pillars:**

1. **5 validator genesis set** (Aura + Grandpa), keys generated offline, stored in Borg-encrypted backup plus hardware token. No Alice, Bob, Charlie, Dave, or Eve keys in genesis.
2. **Sudo disarmament path:** sudo starts under a 3-of-5 multisig, migrates to `pallet-democracy` + `pallet-collective` (governance council) within 90 days of mainnet v2 launch, then sudo is removed entirely.
3. **pallet-assets** manages ePL, gPLIM, pEUR, pUSD. Asset IDs reserved per TOKEN_CATALOG.md.
4. **Genesis allocations:**
   - Treasury: TBD PLIM (recommendation: 40% of max supply)
   - Validators: TBD PLIM each for stake bonding
   - Team/ecosystem: TBD (vested via `pallet-vesting`)
   - Community airdrop reserve: TBD
5. **Chain parameters:**
   - `ss58Format = 42` (pLim custom)
   - `tokenDecimals = 12` for native PLIM
   - `tokenSymbol = "PLIM"`
   - Block time: 6s (Aura)
   - Epoch: TBD
6. **Upgrade path:** all future runtime upgrades go through `set_code` gated by multisig, then governance once sudo is removed.

---

## 5. The 7-step bootstrap plan

### Step 1 — Add pallet-assets + bump spec_version to 101
**Status:** DONE (agent #1)
**What:** Add `pallet-assets` to `runtime/Cargo.toml`, `runtime/src/lib.rs`, construct_runtime, and bump `spec_version` from 100 to 101.
**Where:** `/opt/plimlab/plim-protocol/plim-chain/runtime/src/lib.rs`, `/opt/plimlab/plim-protocol/plim-chain/runtime/Cargo.toml`
**Verification:**
```bash
cd /opt/plimlab/plim-protocol/plim-chain
cargo build --release -p plim-runtime 2>&1 | tail -20
grep -n 'spec_version: 101' runtime/src/lib.rs
grep -n 'pallet-assets' runtime/Cargo.toml
```
**Risk:** runtime size balloons; benchmarks need refresh. Mitigation: use default pallet-assets weights for testnet; rebenchmark before mainnet v2.
**Depends on:** nothing.

### Step 2 — New `mainnet_genesis()` preset, no Alice sudo
**Status:** DONE (agent #1)
**What:** Add a `mainnet_genesis()` preset function that produces a chain spec with a 3-of-5 multisig sudo account, 5 placeholder validator keys (to be replaced by real offline-generated keys before launch), and asset registrations for ePL/gPLIM/pEUR/pUSD.
**Where:** `/opt/plimlab/plim-protocol/plim-chain/runtime/src/genesis_config_presets.rs`
**Verification:**
```bash
cd /opt/plimlab/plim-protocol/plim-chain
./target/release/plim-node build-spec --chain mainnet-v2 > /tmp/mainnet-v2.json
jq '.genesis.runtime.sudo.key' /tmp/mainnet-v2.json    # must NOT be Alice
jq '.genesis.runtime.assets.assets' /tmp/mainnet-v2.json  # must list 4 assets
```
**Risk:** placeholder validator keys must be replaced before genesis finalization — track in the risk register.
**Depends on:** Step 1.

### Step 3 — Implement `PlimPayments::pay`
**Status:** DONE (agent #1)
**What:** Real dispatchable `pay(origin, to, asset_id, amount, memo)` that routes a transfer via either `pallet_balances` (asset_id = 0, native PLIM) or `pallet_assets::transfer` (asset_id > 0). Emits a `PaymentMade` event with memo hash for off-chain reconciliation.
**Where:** `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-payments/src/lib.rs`
**Verification:**
```bash
cd /opt/plimlab/plim-protocol/plim-chain
cargo test -p pallet-plim-payments 2>&1 | tail -20
# On a running testnet node:
./target/release/plim-node --dev --tmp &
# From polkadot.js apps, submit plimPayments.pay(ALICE, 3, 1000000, 0x0)
```
**Risk:** asset_id=0 collision with native transfer — documented in pallet doc; gateway must branch on asset_id.
**Depends on:** Steps 1–2.

### Step 4 — Gateway Substrate client
**Status:** DONE (agent #3)
**What:** `plim-gateway` gains a `SubstrateClient` wrapper around `subxt` (or equivalent) that exposes `submit_pay`, `get_balance`, `get_asset_balance`, and a block subscription for event indexing. The gateway reads node URL from env var `PLIM_NODE_WS_URL`.
**Where:** Gateway repo — paths owned by agent #3.
**Verification:**
```bash
# From gateway host:
curl -s http://localhost:8080/api/chain/health
# Expected: {"node":"connected","head":<number>,"finalized":<number>}
```
**Risk:** subxt metadata must match runtime spec_version; CI step will re-fetch metadata on every runtime bump.
**Depends on:** Steps 1–3.

### Step 5 — Wire each app (testnet first)

All wiring happens against testnet (port 9945) before any mainnet attempt. Each sub-step is independently verifiable.

#### 5a. Stripe → pEUR mint flow
- **Trigger:** `invoice.paid` webhook from Stripe account `acct_1TFrNZDHf0ymfkgV`.
- **Flow:** webhook handler → gateway → multisig mint call `pallet_assets::mint(3, customer_account, amount_in_6dp)`.
- **Testnet behaviour:** sudo can mint directly (shortcut), flag `PLIM_MINT_MODE=sudo`.
- **Mainnet behaviour:** multisig approval required, flag `PLIM_MINT_MODE=multisig`.
- **Verification:** issue a €1 test invoice in Stripe test mode, confirm 1000000 units of asset 3 appear in the target account.

#### 5b. plimcontrol UI
- **Add page:** `/admin/chain/assets` showing live balances for PLIM, ePL, gPLIM, pEUR, pUSD per user.
- **Add page:** `/admin/chain/mint-requests` for multisig mint approvals.
- **Verification:** log in as admin, confirm balances match polkadot.js apps read of the same accounts.

#### 5c. Referral rewards
- **Flow:** referral-system → gateway → `plimPayments.pay` (asset_id=2, gPLIM) to the referrer on conversion event.
- **Verification:** run the referral Fase 4 e2e test against testnet; assert on-chain gPLIM balance delta.

#### 5d. Natali quests anchoring
- **Flow:** quest completion → hash of quest payload → `plimTimestamps.anchor(hash)` call (to be implemented after Step 5c, separate ticket).
- **Verification:** TBD — requires PlimTimestamps pallet body (currently stub).

#### 5e. AIFSACCT anchoring
- **Flow:** nightly close → Merkle root of journal → anchor tx on chain.
- **Verification:** TBD — requires PlimTimestamps pallet body.

#### 5f. Bridges (ap2, mcp, mpp, x402)
- **Scope:** each bridge registers a service account on chain, consumes pEUR/pUSD for metered payments via `plimPayments.pay`.
- **Verification:** per-bridge integration test against testnet.

**Risk for Step 5 overall:** scope creep. Mitigation: each sub-step is a separate PR and can land independently.
**Depends on:** Steps 1–4.

### Step 6 — pEUR MiCA compliance gate

**This step MUST be complete before a single pEUR unit exists on mainnet v2.**

Checklist:

- [ ] Legal entity designated (recommended jurisdiction: Liechtenstein, DLT Act; fallback: Malta, France, Germany)
- [ ] MiCA e-money token whitepaper drafted, reviewed by counsel, submitted to competent authority (60-day review window)
- [ ] Authorisation as EMT issuer obtained (or partnership with an authorised EMI)
- [ ] 1:1 EUR reserve account opened at an EU credit institution, segregated from operating funds
- [ ] Monthly reserve attestation contract signed with audit firm
- [ ] Redemption flow documented and operational (holder → KYC → fiat payout, max T+5 business days)
- [ ] Multisig mint authority: 3-of-5 (issuer officer, CFO, auditor, custodian, legal)
- [ ] On-chain compliance pallet: freeze/blacklist dispatchables gated by multisig
- [ ] Public dashboard showing reserve balance vs circulating supply

**Verification:** none of the above can be self-verified — each is a legal/organizational gate.
**Depends on:** Step 5a (technical integration) but is independent of Step 5 completion.

### Step 7 — Mainnet v2 promotion

Only after Steps 1–6 are green.

Checklist:

- [ ] Validator keys generated offline on airgapped hardware, stored in Borg-encrypted backup + hardware token. Public keys exported for genesis.
- [ ] Session keys rotated; Aura + Grandpa keys inserted into each validator via `author_insertKey` over localhost only.
- [ ] Final `chain-spec-mainnet-v2-raw.json` produced from `mainnet_genesis()` preset with real validator keys, reviewed by ≥2 engineers, sha256 recorded in the release notes.
- [ ] Monitoring: Prometheus scrape of `/metrics` on each validator, alerts for: block production stall (>60s), finality lag (>30 blocks), peer count (<3 warning / <1 page).
- [ ] Alert routing: PagerDuty → primary on-call → secondary → whole team.
- [ ] WireGuard tunnel healthy (reuse existing 10.8.0.2 setup + handshake alert).
- [ ] Backup: daily Borg snapshot of each validator's `chains/plim-mainnet-v2/db` directory, retention 30 daily + 12 weekly + 12 monthly.
- [ ] Sudo custody: 3-of-5 multisig keys distributed, never co-located.
- [ ] Mainnet v1 drained: snapshot of state, export of balances to CSV, communicated deprecation date ≥14 days in advance.
- [ ] Mainnet v2 genesis block signed off in writing (email, archived) by the responsible technical lead.
- [ ] Launch window: low-traffic UTC hour, full team on-call for first 4 hours post-genesis.

---

## 6. 24/7 operations playbook

### Backup
- Borg repo: see `reference_backup_borg.md` in memory index for passphrase location.
- Validators: `systemctl stop plim-node-mainnet-v2 && borg create ... && systemctl start plim-node-mainnet-v2` nightly via cron at 04:15 UTC (staggered across validators so only one stops at a time).
- Chain spec + runtime WASM: versioned in git and pinned to release tag.

### Monitoring
- Prometheus: existing pLim stack on 91.99.60.74. Add jobs `plim-validator-01..05` scraping `:9615/metrics` (or equivalent).
- Alertmanager rules (minimum set):
  - `PlimBlockProductionStalled` — `increase(substrate_block_height[2m]) == 0` for 2m → page
  - `PlimFinalityLag` — `substrate_block_height - substrate_block_finalized > 30` for 5m → warn
  - `PlimLowPeers` — `substrate_sub_libp2p_peers_count < 3` for 5m → warn; `< 1` for 1m → page
  - `PlimWireGuardDown` — reuse existing WG handshake rule
- Socat watchdog (existing) remains.
- 4-tier LLM cascade (gemma3:12b primary, via agent.plimlab.ch → 10.8.0.2) supervises incident triage.

### Alert response SOP
1. On page, acknowledge within 5 minutes in PagerDuty.
2. SSH to `91.99.60.74` (SSH 22), `sudo systemctl status plim-node-mainnet-v2`, `journalctl -u plim-node-mainnet-v2 -n 200`.
3. Check peer count: `curl -s http://localhost:9933 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}'`.
4. If stuck: try `systemctl restart`; if not resolved in 10 min, escalate.
5. All incidents get a postmortem in `/opt/plimlab/plim-protocol/plim-chain/docs/postmortems/`.

### Validator key custody
- Each validator has its own session keyfile under `/var/lib/plim-node-v2/keystore/` owned by user `plim-node`, mode 0600.
- Offline master keys never touch the server; only session keys generated via `key generate-node-key` are inserted.
- Rotation: every 90 days or immediately on suspected compromise.

### Mainnet v1 → v2 cutover
- **T-14 days:** announce deprecation.
- **T-7 days:** freeze any new Stripe mint requests routed to v1 (none should exist by this point).
- **T-0:** stop `plim-node-mainnet` (v1), export final state snapshot, tar + borg it, mark repo `chain-spec-mainnet.json.MOVED` permanent.
- **T+0:** start mainnet v2 with the sha256-verified raw spec.
- **T+4h:** if green, open mainnet to external participants.

---

## 7. Risk register

| ID | Risk | Current status | Mitigation | Owner |
|----|------|----------------|-----------|-------|
| R1 | Alice sudo on mainnet v1 | ACTIVE CRITICAL | Retire v1 entirely in Step 7; never reuse v1 genesis | Tech lead |
| R2 | Single validator on mainnet v1 (peers=0) | ACTIVE HIGH | v2 launches with 5 validators | Tech lead |
| R3 | Missing mainnet chain-spec files (.MOVED) | Tracked | New files produced by `mainnet_genesis()` preset | Agent #1 |
| R4 | Custom pallets are empty stubs | ACTIVE HIGH | Implement PlimPayments first (Step 3); others follow in ordered tickets | Protocol team |
| R5 | UI promises features pallets don't deliver | Tracked | Step 5b gated on Step 3 | Frontend lead |
| R6 | pEUR minted without MiCA authorization | POTENTIAL CATASTROPHIC | Step 6 is a hard gate; technical mint path disabled on mainnet until gate passes | Legal + tech lead |
| R7 | pUSD legal framework unclear | Tracked | Keep pUSD testnet-only until US/EU framework chosen | Legal |
| R8 | Validator key compromise | Mitigated | Offline generation, Borg backup, 90-day rotation | SecOps |
| R9 | Stripe webhook replay → double mint | Mitigated | Existing idempotency store in `plim_subscriptions` | Billing team |
| R10 | Runtime upgrade bricks chain | Mitigated | Testnet canary runs upgrade 7 days before mainnet | Protocol team |
| R11 | Borg passphrase loss | Mitigated | Passphrase in password manager + `/root/borg-passphrase.txt` + physical printout in safe | SecOps |

---

## 8. Glossary

- **Aura** — Substrate's slot-based block production (authority round-robin).
- **Grandpa** — Substrate's finality gadget (GHOST-based recursive ancestor voting).
- **Sudo** — Pallet granting a single account (or multisig) unchecked dispatch rights; must be removed before true decentralization.
- **pallet-assets** — Substrate pallet that lets you mint arbitrary fungible assets alongside native balances.
- **Asset ID** — The u32 key under which a pallet-assets token is registered.
- **EMT (Electronic Money Token)** — Under MiCA, a crypto-asset pegged to a single fiat currency; subject to the strictest authorization regime.
- **MiCA** — EU Regulation 2023/1114 on Markets in Crypto-Assets, in force 2024; applies to any issuer offering to EU residents.
- **Multisig** — Aggregated signature scheme requiring M-of-N co-signers to dispatch a call; on Substrate via `pallet-multisig`.
- **Genesis** — The block-0 state of a chain; immutable once the chain is running.
- **spec_version** — Monotonically increasing integer that identifies runtime versions; must bump for every runtime behaviour change.
- **Session keys** — Per-validator online keys used to sign blocks (Aura) and finality votes (Grandpa); rotatable.
- **mainnet v1 / v2** — We use v1/v2 informally to distinguish the retired single-validator chain from the new hardened chain; they have different genesis hashes and are not related by upgrade.
- **pEUR / pUSD / ePL / gPLIM** — see `TOKEN_CATALOG.md`.

---

**End of roadmap.** Questions or objections → open a ticket and tag protocol-team; do not hotfix on mainnet v1.
