# pLim Protocol — Live STATUS

**Last updated:** 2026-04-14T15:30:00Z
**Branch:** `wip/2026-04-14-server-fixes` (plim-chain submodule)
**Author of this update:** automated bootstrap orchestration (agent 6 of 7, parallel batch)
**Scope:** Single source of truth for the end-of-day pLim Protocol mainnet-v2 bootstrap effort.
**Reader:** This file is written so that a brand-new operator picking up the ecosystem today can understand
what happened, what's live, what's dangerous, and what to do next — without having to reconstruct
the context from git logs.

> NOTE: facts marked **[verified]** were confirmed by direct command during the generation of this
> document. Facts marked **[reported]** are claims from parallel orchestration agents that this agent
> did not independently re-verify. Facts marked **[TBD]** are open questions or unknowns.

---

## 1. At a glance

The pLim Protocol ecosystem is in the middle of a controlled rebuild.

- **Mainnet v1** is still running (PID 3867217, up 5 days) on port 9946 and still happens to
  accept RPC, but it is structurally **quarantined**: it was bootstrapped with `--alice` sudo,
  has **0 peers** [verified], and its on-disk chain spec files have been moved to
  `/root/SECURE/chain-specs-archived-2026-04-14/` so that no future reboot can accidentally
  re-ingest them. The running binary is alive only because the Linux kernel still holds the
  deleted file descriptors open (deleted-fd lifeline). A reboot of this box will kill v1
  permanently.
- **Mainnet v2** is the intended production chain. Runtime code is **compile-clean**
  (`cargo check -p plim-runtime` PASS, 29.68s) [reported]. A release binary exists at
  `/mnt/data/cargo-target/release/plim-node` (78 MB, built 2026-04-14T15:23) [verified],
  and a raw chain spec `chain-spec-mainnet-v2-raw.json` (984 KB) is present in the repo
  [verified]. The v2 chain is **not yet running** anywhere — no listening service on port
  9947, no systemd unit active, no Traefik route. Genesis allocations are still in-flight
  (task #40 is generating the final pallet wiring).
- **Testnet 5-validator** (Alice / Bob / Charlie / Dave / Eve) is **healthy**: port 9945,
  4 peers [verified], 5 systemd services active [verified], head at block 0x14b8c = **84876**
  [verified], genesis `0xe8399f148c4eb872d24dd5b2773ee3231723c5fd88a96f0295027f904cf5e2ae`
  [verified]. But its raw chain spec `chain-spec-testnet-5val-raw.json` is one of the
  deleted-fd bombs: if the Alice node restarts, restoration from the archived genesis
  **FAILED** earlier today due to a genesis mismatch [reported]. Treat the whole testnet
  as "do not reboot without a recovery plan."
- **8 custom pallets**: PlimPayments has been given a real `pay` extrinsic with mock + 3
  unit tests [reported]. The other 7 (PlimIdentity, PlimMandates, PlimChannels,
  PlimDelegation, PlimCompliance, PlimReputation, PlimTimestamps) are being moved from
  stub to functional in parallel right now (task #40). The plim-chain working tree shows
  modifications to 4 of those pallets [verified via `git status`].
- **Gateway Substrate client**: foundation committed (`1f6fc40`) [verified], `@polkadot/api`
  dependency added to `package.json`, `src/lib/chain.ts` singleton in place, POC route
  `GET /tokens/chain-info` wired up [verified via grep]. `npm install` + service restart is
  task #41 and not yet done.
- **plimcontrol UI** has pUSD live in production as a sibling to pEUR across the 3 token
  pages; 0 pBRL references remain [reported]; commit `cc83beb` on `wip/pusd-stablecoin`.
  The public site responds 200 [verified].

In short: mainnet v1 is gracefully dying, mainnet v2 is almost ready to be born, testnet is
a critical hostage we cannot reboot, and the gateway layer is half-assembled on top.

---

## 2. Current chain topology

### 2.1 Mainnet v1 (QUARANTINED)

| Property | Value | Source |
|---|---|---|
| Systemd unit | `plim-node-mainnet.service` (active, running) | [verified] |
| Main PID | 3867217 | [verified] |
| Uptime | since 2026-04-08 17:44:59 UTC (~5 days) | [verified] |
| Binary path | `/mnt/data/cargo-target/release/plim-node` | [verified] |
| Chain spec (in-memory) | `chain-spec-mainnet-raw.json` (DELETED on disk, fd held open) | [verified] |
| RPC port | 9946 (local + external, unsafe, Safe methods) | [verified] |
| libp2p port | 30335 | [verified] |
| Prometheus port | 9617 | [verified] |
| Base path | `/mnt/data/plim-chain-mainnet` | [verified] |
| Peers | **0** | [verified] |
| isSyncing | false | [verified] |
| Head block | 0x14e35 = **85557** | [verified] |
| Genesis hash | `0xd13f10b893e9feaa487c20459bd9da8f2e23f40f20a275cebc03c4d6041eeb1d` | [verified] |
| Validator mode | `--validator` + `--alice` (implicit Alice sudo, legacy) | [verified — matches cmdline args: `--name Plim-Mainnet-1 ... --validator`; alice sudo inferred from context] |
| Lifecycle | **do not reboot** — the deleted chain spec fd is the only thing keeping it alive | — |

Reboot risk: if the host or the service restarts, systemd will try to exec the same command
line which references the deleted `chain-spec-mainnet-raw.json` path — **startup will fail**.
This is intentional: the box has been deliberately rigged to refuse a v1 restart so that the
only forward path is v2.

### 2.2 Testnet 5-validator (HEALTHY but reboot-fragile)

| Validator | Systemd unit | RPC | libp2p | Base path | State |
|---|---|---|---|---|---|
| Alice | `plim-node-testnet.service` | 9945 | 30334 | `/mnt/data/plim-chain-testnet/alice` | active, running, peers=4 [verified] |
| Bob | `plim-node-testnet-bob.service` | (internal) | (internal) | (internal) | active, running [verified] |
| Charlie | `plim-node-testnet-charlie.service` | (internal) | (internal) | (internal) | active, running [verified] |
| Dave | `plim-node-testnet-dave.service` | (internal) | (internal) | (internal) | active, running [verified] |
| Eve | `plim-node-testnet-eve.service` | (internal) | (internal) | (internal) | active, running [verified] |

Chain state:

| Property | Value | Source |
|---|---|---|
| Head block | 0x14b8c = **84876** | [verified] |
| isSyncing | false | [verified] |
| Peers | 4 (Alice view) | [verified] |
| Genesis hash | `0xe8399f148c4eb872d24dd5b2773ee3231723c5fd88a96f0295027f904cf5e2ae` | [verified] |
| Chain spec path | `chain-spec-testnet-5val-raw.json` (deleted on disk, fd held open) | [verified via cmdline] |
| Reboot recovery | **FAILED** (genesis mismatch) | [reported, task #31] |
| Risk level | bomba-de-reboot — test safe restore before any kernel upgrade | — |

### 2.3 Mainnet v2 (NOT YET DEPLOYED)

| Property | Value | Source |
|---|---|---|
| Runtime spec_version | 100 → 101 | [reported] |
| Runtime compile | `cargo check -p plim-runtime` PASS (29.68s) | [reported] |
| Release binary | `/mnt/data/cargo-target/release/plim-node` (78361208 bytes, 2026-04-14T15:23) | [verified] |
| Chain spec (raw) | `chain-spec-mainnet-v2-raw.json` (1008525 bytes) | [verified] |
| Chain spec (human) | *not generated separately* | [verified by ls] |
| Systemd unit | *not yet activated* — task #44 is creating the template | [reported] |
| Target RPC port | 9947 (proposed, not yet listening) | [reported] |
| Sudo model | multisig (3-of-5 proposed) | [TBD — see §6] |
| Peers model | single validator at genesis, multi-validator post-launch | [TBD] |
| Public endpoint | `mainnet-v2.protocol.plimlab.ch` (Traefik route planned, not live) | [reported, not verified] |

### 2.4 chain-spec-dev (dev preset)

Still present in the repo (`chain-spec-dev.json` + `chain-spec-dev-raw.json`, both 2026-03-29)
[verified]. Not currently running. Untouched by today's changes.

---

## 3. Token state

This is a condensed view of `/opt/plimlab/plim-protocol/plim-chain/TOKEN_CATALOG.md` (181 lines,
8 TBDs [verified]). Read that file for the full version.

| Token | Role | Status | Pallet | Supply model | Legal | Public UI |
|---|---|---|---|---|---|---|
| **PLIM** | native gas, staking, governance | mainnet v2 genesis (1B total) [reported] | `pallet_balances` | 1B at genesis, no minting beyond | — | TokenDashboard (plimcontrol) |
| **ePL** | energy credits (LiftGrid) | testnet only; mainnet activation gated | custom (pallet_assets wrap) | TBD | TBD | plimliftgrid |
| **gPLIM** | governance wrap of PLIM | spec only | TBD | TBD | TBD | none |
| **pEUR** | regulated stablecoin (primary) | issued on testnet; mainnet pending reserve contract | `pallet_assets` asset ID TBD | fiat-backed 1:1 | issuer-of-record **TBD** (FMA/MFSA/AMF/BaFin) | Stablecoins page |
| **pUSD** | stablecoin (secondary) | plimcontrol UI live; no chain asset yet | `pallet_assets` asset ID TBD | fiat-backed 1:1 | **TBD** (BitLicense / partner-wrap / offshore) | Stablecoins page [reported] |

Proposed mainnet v2 genesis allocation (from TOKEN_CATALOG.md / BOOTSTRAP_ROADMAP.md — **not yet
committed to code**):

| Bucket | PLIM | Notes |
|---|---|---|
| Treasury | 600,000,000 | 60% — multisig-controlled |
| Team vesting | 200,000,000 | 20% — vesting pallet, 4y / 1y cliff TBD |
| Reserve / ecosystem | 200,000,000 | 20% |
| Sudo (gas-only) | 1,000 | ~nothing, just enough to pay extrinsic fees during boot |
| **Total** | **1,000,000,000** | Hardcoded max, no minting policy beyond (**TBD**) |

---

## 4. Build state

| Component | State | Notes |
|---|---|---|
| Runtime (`plim-runtime`) | spec_version 101 ready, `cargo check` PASS [reported] | needs `cargo build --release` — task #39 running in background |
| Runtime configs | `runtime/src/configs/` exists [verified] | pallet-assets@42 added at index 8 [reported] |
| Genesis presets | `runtime/src/genesis_config_presets.rs` (7272 bytes, modified 2026-04-14T15:00) [verified] | `mainnet_genesis()` preset added [reported] |
| PlimPayments | functional: real extrinsic + mock + 3 unit tests [reported] | commit 29755bd |
| PlimIdentity | stub → functional in progress [reported]; Cargo.toml + lib.rs dirty in worktree [verified] | task #40 |
| PlimMandates | stub → functional in progress [reported]; Cargo.toml + lib.rs dirty in worktree [verified] | task #40 |
| PlimChannels | stub → functional in progress [reported]; not in `git status` dirty list [verified] | task #40 |
| PlimDelegation | stub → functional in progress [reported]; not in `git status` dirty list [verified] | task #40 |
| PlimCompliance | stub → functional in progress [reported]; not in `git status` dirty list [verified] | task #40 |
| PlimReputation | stub → functional in progress [reported]; Cargo.toml + lib.rs dirty in worktree [verified] | task #40 |
| PlimTimestamps | stub → functional in progress [reported]; Cargo.toml + lib.rs dirty in worktree [verified] | task #40 |
| Node binary (`plim-node`) | present in `/mnt/data/cargo-target/release/plim-node`, 78 MB, built 2026-04-14T15:23 [verified] | used by both mainnet v1 (via deleted-fd) and testnet |
| Gateway `src/lib/chain.ts` | present [verified] | singleton ApiPromise + signer loader [reported] |
| Gateway `/tokens/chain-info` route | present in `src/routes/tokens.ts` [verified] | POC smoke test endpoint |
| Gateway `@polkadot/api` dep | added to `package.json` [reported] | `npm install` pending — task #41 |
| plimcontrol pUSD UI | bundle index-BrpQ8x0O.js [reported] | public 200 [verified] |
| Mainnet-v2 raw chain spec | `chain-spec-mainnet-v2-raw.json` present, 984 KB [verified] | not yet served to a running node |
| Mainnet-v2 systemd unit | template being written by task #44 [reported] | not activated |

---

## 5. What's done today (2026-04-14)

Commit SHAs in `plim-chain` submodule unless noted. Order is rough chronological.

### 5.1 Earlier in the day (context before this bootstrap session)
1. 9 server-side fixes in `nv-backend` (CRM, profile, notifications, OSINT, media, research, registro-civil) [reported]
2. Meta-graph backbone in `nv-backend`: 10 files + migration applied + Prisma client regenerated + endpoints live [reported]
3. 4 Lovable prompts generated for Natali-Virtual frontend [reported]
4. 4-tier LLM cascade wired into `nv-backend` (plimagent-vllm → plimagent-ollama gemma3:12b → nv-ollama-local qwen2.5:3b → anthropic) [reported]
5. `nv-ollama` running as cold standby [reported]
6. Mailserver `opendmarc` fix + `plimcontrol` login loop fix [reported]
7. WireGuard tunnel monitoring (Prometheus alert + socat watchdog systemd timer) [reported]
8. SSH access added for the user's MacBook public key [reported]
9. Apt safe-tier upgrade (41 packages, no docker restart) [reported]
10. Swap headroom: `+4G swapfile2` in `/etc/fstab` [reported]
11. Backup of mainnet v1 keys to `/root/SECURE/MAINNET_KEYS_BACKUP_2026-04-13.md` [verified — file exists]

### 5.2 This bootstrap batch (done by parallel agents before this agent started)
12. **#30 Runtime overhaul** — commit `29755bd` on `wip/2026-04-14-server-fixes` [verified via `git log`]
    - `pallet-assets@42` added at runtime index 8
    - `spec_version 100 → 101`
    - removed `pallet-template`
    - `PlimPayments::pay` extrinsic implemented (real, with mock + 3 unit tests)
    - `mainnet_genesis()` preset added to `runtime/src/genesis_config_presets.rs`
    - `mainnet_chain_spec()` added in node
    - 5 new keys generated: sudo / treasury / team_vesting / aura / grandpa
    - Keys encrypted at `/root/SECURE/mainnet_v2_keys_2026-04-14.txt` [verified — file exists, 3436 bytes, 600]
    - `cargo check -p plim-runtime` PASS in 29.68s [reported]
13. **#31 Safety evacuation**
    - Alice-sudo mainnet spec files moved to `/root/SECURE/chain-specs-archived-2026-04-14/`
      (contains `chain-spec-mainnet.alice-sudo.json` + `chain-spec-mainnet-raw.alice-sudo.json`) [verified]
    - Testnet 5-validator spec restoration attempted but **FAILED** with genesis mismatch [reported] — flagged as bomba-de-reboot
    - Live mainnet (9946) and testnet (9945) still answer RPC because the running binaries hold open
      file descriptors to the now-deleted spec files (`.MOVED` placeholders at original paths) [verified]
14. **#32 Gateway chain client** — commit `1f6fc40` [verified via `git log -5` in `plim-gateway`]
    - `src/lib/chain.ts` created: singleton `ApiPromise`, `loadSigner` from `/root/SECURE/keys`, 60s submit timeout [reported]
    - `package.json` updated: `@polkadot/api ^16.4.5`, `@polkadot/keyring ^14.0.1` [reported]
    - POC route `GET /tokens/chain-info` added [verified via grep in `src/routes/tokens.ts`]
    - `tsc` clean [reported]
15. **#33 plimcontrol UI pUSD** — commit `cc83beb` on `wip/pusd-stablecoin`
    - 0 pBRL refs (already removed by earlier `028c6b9`) [reported]
    - pUSD added as sibling to pEUR in TokenDashboard, Stablecoins, TokenTransfers [reported]
    - Build OK, deploy via bind mount [reported]
    - Public site returns 200 [verified]
16. **#34 plimliftgrid recon** — SSH refused on 22 / 2222 / 22022 from this box [reported]
    - HTTPS public 200 on `aifsacct.plimliftgrid.eu` [verified]
    - HTTPS public 200 on `plimliftgrid.eu` [reported — not independently verified]
    - Internal substrate state: **UNKNOWN**
17. **#35 Bootstrap docs written**
    - `/opt/plimlab/plim-protocol/plim-chain/BOOTSTRAP_ROADMAP.md` (300 lines) [verified — `wc -l`]
    - `/opt/plimlab/plim-protocol/plim-chain/TOKEN_CATALOG.md` (181 lines, 8 TBDs) [verified]
18. **#36 Memory updates** (MEMORY.md index + 4 project/reference files): `reference_server_91_99_60_74`,
    `project_mainnet_bootstrap`, `reference_server_plimliftgrid`, plus new `project_protocol_bootstrap.md`;
    index now 36 lines [reported]
19. **#37 Chain alerts** — new Prometheus rules at
    `/opt/plimlab/node-tools-ecofi-plimlab/monitoring/prometheus/alerts/chain-bootstrap.yml`
    (205 lines, **11 alert rules** [verified — `grep -c "alert:"`], not 10 as originally reported)
    covering block-stuck, finality-stalled, low-peers, slow blocks, metric-absent, RPC-stale;
    `promtool` PASS + prometheus reloaded [reported]
20. **#38 Backups** — `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/`
    - **16 GB total** (`du -sh /root/SECURE/chain-snapshots/` = 16G) [verified]
    - Contents (from snapshot README [verified]):
      - `mainnet/` — 2.9 GB
      - `testnet-alice/` — 6.6 GB
      - `testnet-bob/` — 1.6 GB
      - `testnet-charlie/` — 1.6 GB
      - `testnet-dave/` — 1.6 GB
      - `testnet-eve/` — 1.6 GB
    - `MANIFEST.sha256`, `SIZES.txt`, `README.md` present [verified]
    - Permissions: root-only 600 [verified — `drwx------`]
    - Caveat: rsync-while-running, RocksDB may be mid-compaction — "emergency rollback, not
      fork/migration"

### 5.3 THIS batch (running in parallel with this STATUS.md)

These are the 7 parallel agents running alongside this one. Status is what was reported at dispatch
time; verify each before assuming done.

- **#39** `cargo build --release` in background — should produce an updated
  `/mnt/data/cargo-target/release/plim-node` and a clean `chain-spec-mainnet-v2-raw.json`.
  **Note:** a `plim-node` binary is already present from an earlier build (2026-04-14T15:23)
  [verified]. The running build will overwrite it.
- **#40** 7 remaining pallets moved from stub → functional + concrete genesis allocations
  (1B PLIM total: 600M treasury, 200M team vesting, 200M reserve, 1k sudo gas). Working tree in
  `plim-chain` confirms 4 of 7 pallets are currently dirty [verified].
- **#41** `npm install` in `plim-gateway` + `systemctl restart plim-gateway` + smoke-test
  `/tokens/chain-info`.
- **#42** Signer key JSON files at `/root/SECURE/keys/<name>.json` for the 5 mainnet v2 keys.
  **Already present**: 5 files confirmed [verified] — `plim_mainnet_sudo.json`,
  `plim_mainnet_treasury.json`, `plim_mainnet_team_vesting.json`, `plim_mainnet_aura.json`,
  `plim_mainnet_grandpa.json`.
- **#43** plimcontrol branch merge: consolidate `wip/pusd-stablecoin` into a single `wip` branch.
- **#44** Mainnet v2 deploy infra: systemd unit template (port 9947) + Traefik dynamic route
  template + `MAINNET_V2_DEPLOY.md` runbook.
- **#45** STATUS.md ← **this document** (agent 6)
- **#46** SSH `plimliftgrid` retry (unknown outcome).

---

## 6. Risks open

1. **Reboot bomb — mainnet v1.** Service is running from a deleted chain spec fd. A host reboot,
   a `systemctl restart plim-node-mainnet`, or even a kernel upgrade that restarts the service
   will kill v1 permanently because systemd will re-exec against a path that no longer exists.
   Mitigation: do not reboot until mainnet v2 is live, and keep the archived v1 spec at
   `/root/SECURE/chain-specs-archived-2026-04-14/` as last-resort restore material.
2. **Reboot bomb — testnet 5-val.** Same deleted-fd situation, plus an earlier restore attempt
   failed with a genesis mismatch. If *any* of the 5 validators restarts, it may refuse to
   rejoin. Mitigation: before any kernel upgrade, reproduce a successful restore in a
   throwaway directory against the 16 GB snapshot at
   `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/`.
3. **Mainnet v1 is a single-validator chain with 0 peers.** Even if it survives, it has no
   Byzantine safety margin. Consider it purely a "museum" chain until v2 takes over.
4. **pEUR legal issuer-of-record is TBD.** Options on the table: Liechtenstein FMA, Malta MFSA,
   France AMF, Germany BaFin. A chain-level asset ID cannot be finalized until the legal wrapper
   is picked. [TBD]
5. **pUSD legal framework is TBD.** BitLicense vs partner-wrap vs offshore-SPV — no decision yet.
   The plimcontrol UI is already showing pUSD, but there is **no on-chain pUSD asset yet**. Risk:
   product-legal desync. [TBD]
6. **Runtime not yet built in release profile for v2.** `cargo check` passed, but the actual
   release binary that will run mainnet v2 is being produced by task #39, which could take
   30–60 minutes. Until then, we don't know if the new pallets link cleanly in release mode.
7. **7 of 8 pallets are still being refactored.** The `plim-chain` working tree shows unstaged
   changes in 4 pallets (`plim-identity`, `plim-mandates`, `plim-reputation`, `plim-timestamps`)
   [verified]. Any checkout / branch switch now will either lose work or trigger a merge conflict.
8. **Multisig threshold for mainnet v2 sudo.** 3-of-5 proposed but not committed. Until it is,
   the sudo key is effectively a single-signer risk. [TBD]
9. **Mainnet v2 epoch length and finality parameters.** No decision on session length / era
   length / GRANDPA voter-set-rotation cadence. [TBD]
10. **ePL / gPLIM mainnet activation gates.** Spec-only. No on-chain asset, no issuance path.
    [TBD]
11. **plimliftgrid internal substrate state UNKNOWN.** SSH is firewalled from this box. We
    cannot verify whether plimliftgrid is running its own chain node, and if so, at what
    version. [TBD]
12. **Chain-bootstrap alerts are defined but not yet battle-tested.** 11 rules in
    `chain-bootstrap.yml` [verified], promtool PASS [reported], but none have fired in
    production yet — unclear whether the label matchers actually hit the `chain-exporter`
    metrics on this box.
13. **Gateway chain client has never actually talked to a running chain.** `npm install` has
    not been run yet, so `@polkadot/api` is not in `node_modules`. The `/tokens/chain-info`
    route will 500 until task #41 completes.

---

## 7. Next actions (prioritized)

**Priority 0 — today, blocking everything else:**
1. Wait for task #39 (`cargo build --release`) to finish. Verify the new binary starts with
   `--chain chain-spec-mainnet-v2-raw.json --dev --tmp` just as a smoke test, in a throwaway
   base path — do **not** point it at a persistent base path yet.
2. Wait for task #40 (7 pallets + genesis allocations) to commit. Review the diff before
   merging into `wip/2026-04-14-server-fixes`.
3. Wait for task #41 (`npm install` in gateway + restart). Smoke-test `GET /tokens/chain-info`
   against mainnet v1 first (still running on 9946), then switch it to v2 once v2 is live.

**Priority 1 — deploy v2 safely:**
4. Finalize genesis allocations in `runtime/src/genesis_config_presets.rs` (§3 proposed split).
5. Confirm multisig sudo threshold (proposed: 3-of-5). Generate the 5 signer keys and the
   multisig address **before** regenerating the raw chain spec.
6. Regenerate `chain-spec-mainnet-v2-raw.json` after final runtime + genesis.
7. Activate the mainnet-v2 systemd unit from task #44 on port 9947, with a **fresh base path**
   like `/mnt/data/plim-chain-mainnet-v2/`. Leave v1 running.
8. Verify `system_health` shows `isSyncing=false, peers=0` (expected for a brand-new singleton).
9. Add Traefik dynamic route `mainnet-v2.protocol.plimlab.ch` → `127.0.0.1:9947` (proposed,
   [TBD]).
10. Wire the gateway `chain.ts` singleton to v2 via env var (was pointing at 9946 in the POC).

**Priority 2 — retire v1:**
11. Freeze v1 state: final block snapshot to `/root/SECURE/chain-snapshots/v1-final/`.
12. Stop `plim-node-mainnet.service`, rename unit to `plim-node-mainnet-v1-archive.service`,
    disable it.
13. Update plimcontrol TokenDashboard to show v2 metrics only (navigation switch).

**Priority 3 — de-risk testnet:**
14. Fix the testnet-5val genesis-mismatch restore bug before the next reboot window. Reproduce
    the failure from the snapshot at `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/` in a
    sandbox base path, identify the exact offending field, and commit the fix as a brand-new
    `chain-spec-testnet-5val-raw.json` checked into git (not left as a deleted-fd ghost).
15. Once the new testnet spec boots from scratch, re-encode it into the systemd unit lines and
    reload-daemon. Only then is the testnet reboot-safe.

**Priority 4 — legal / governance (not blocking code):**
16. Pick pEUR issuer-of-record (§6 risk #4).
17. Pick pUSD legal framework (§6 risk #5).
18. Define PLIM minting / emission beyond genesis (§6 implied by "no minting strategy").
19. Finalize mainnet v2 era / epoch length.
20. Define ePL + gPLIM mainnet activation gates.

**Priority 5 — ecosystem:**
21. Regain SSH to `plimliftgrid` (91.99.55.103) to audit its internal substrate state.
22. Decide whether plimliftgrid should join mainnet v2 as a validator or stay isolated on its
    own LiftGrid chain.

---

## 8. Deploy commands ready (copy-paste, **review before running**)

All commands assume you are root on `91.99.60.74` (`tools-aifirst-v1`).

### 8.1 Activate the mainnet-v2 systemd unit (AFTER #44 lands the template)

```bash
# 1. Install the unit (path TBD — task #44 will write it)
sudo cp /opt/plimlab/plim-protocol/plim-chain/plim-node-mainnet-v2.service.template \
        /etc/systemd/system/plim-node-mainnet-v2.service

# 2. Make sure the base path is fresh (this is important — do NOT reuse v1 dir)
sudo mkdir -p /mnt/data/plim-chain-mainnet-v2
sudo chown plimadmin:plimadmin /mnt/data/plim-chain-mainnet-v2

# 3. Dry run
sudo systemd-analyze verify /etc/systemd/system/plim-node-mainnet-v2.service

# 4. Start
sudo systemctl daemon-reload
sudo systemctl enable --now plim-node-mainnet-v2

# 5. Watch
sudo journalctl -fu plim-node-mainnet-v2

# 6. Smoke RPC (port TBD — 9947 proposed)
curl -sS -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
  http://127.0.0.1:9947
```

### 8.2 Public expose mainnet-v2 via Traefik (AFTER #44 lands the route template)

```bash
# Traefik dynamic config lives in (path TBD — confirm with task #44):
# /opt/plimlab/plim-protocol/traefik/dynamic/mainnet-v2.yml

# After copying, Traefik auto-reloads dynamic configs. Verify:
curl -sSI https://mainnet-v2.protocol.plimlab.ch/
```

### 8.3 Switch plimcontrol navigation to v2

```bash
# The plimcontrol build is at /opt/plimlab/plimcontrol
# Change the chain RPC env var in docker-compose.yml or .env:
#   VITE_CHAIN_RPC_WS=wss://mainnet-v2.protocol.plimlab.ch
# Then rebuild + redeploy:
cd /opt/plimlab/plimcontrol
npm run build
sudo systemctl restart plimcontrol  # or: docker compose up -d
```

### 8.4 Restart gateway to pick up Substrate client

```bash
cd /opt/plimlab/plim-protocol/plim-gateway
npm install                       # ← task #41 does this
sudo systemctl restart plim-gateway
curl -sS http://127.0.0.1:<gateway-port>/tokens/chain-info
```

### 8.5 Emergency rollback — restore mainnet v1 from snapshot

```bash
# Only if v2 is toast AND v1 is also toast (worst case)
sudo systemctl stop plim-node-mainnet
sudo mv /mnt/data/plim-chain-mainnet /mnt/data/plim-chain-mainnet.broken
sudo rsync -a /root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/mainnet/ \
              /mnt/data/plim-chain-mainnet/
# You'll also need to restore the chain spec file from
# /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json
sudo cp /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json
sudo systemctl start plim-node-mainnet
```

### 8.6 Emergency rollback — restore testnet from snapshot

```bash
# Per-validator. Example for Alice:
sudo systemctl stop plim-node-testnet
sudo mv /mnt/data/plim-chain-testnet/alice /mnt/data/plim-chain-testnet/alice.broken
sudo rsync -a /root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/testnet-alice/ \
              /mnt/data/plim-chain-testnet/alice/
sudo systemctl start plim-node-testnet
# KNOWN ISSUE: this will still hit the genesis-mismatch bomb until the
# testnet spec is rebuilt from source. See §7 Priority 3.
```

---

## 9. Servers / endpoints

### 9.1 Host fleet

| Host | IP | SSH | Role | Contains |
|---|---|---|---|---|
| `tools-aifirst-v1` | 91.99.60.74 | :22 | **main protocol box** | mainnet v1, testnet 5-val, plim-gateway, plimcontrol, Natali-Virtual, mail, monitoring, ~46 containers |
| `plimliftgrid` | 91.99.55.103 | firewalled from us | LiftGrid EaaS + dedicated AIFSACCT | substrate state UNKNOWN |
| `srv-mdu-prd-app-01` | 46.225.189.32 | :2222 | 3dplim (ex-MDU) | no substrate |
| plimagent home box | 88.27.18.91 / 10.8.0.2 (WG) | via WireGuard | vLLM, ollama (gemma3:12b primary), auth proxy, CDN | no substrate |

### 9.2 Public endpoints (verified today)

| URL | HTTP | Service | Source |
|---|---|---|---|
| `https://mainnet.protocol.plimlab.ch/` | **200** (expected 405 for GET on a POST-only JSON-RPC — that's actually what we got, `HTTP/2 405 allow: (none specified)`) | mainnet v1 RPC via Traefik | [verified] |
| `https://plimcontrol.plimlab.ch/` | **200** | plimcontrol admin hub | [verified] |
| `https://aifsacct.plimlab.ch/` | **405** (GET → allowed method GET, so 405 is odd — probably a root-endpoint auth) | AIFSACCT on this box | [verified] |
| `https://aifsacct.plimliftgrid.eu/` | **200** | AIFSACCT on plimliftgrid | [verified] |
| `https://plimliftgrid.eu/` | 200 (reported) | LiftGrid marketing | [reported] |
| `https://mainnet-v2.protocol.plimlab.ch/` | **does not exist yet** | mainnet v2 RPC (planned) | [TBD] |
| `https://testnet.protocol.plimlab.ch/` | **DNS does not resolve** | testnet public RPC (not exposed) | [verified — curl: Could not resolve host] |

### 9.3 Local ports (this box)

| Port | Service | State |
|---|---|---|
| 9945 | testnet Alice RPC | LISTENING, 4 peers, healthy [verified] |
| 9946 | mainnet v1 RPC | LISTENING, 0 peers, quarantined [verified] |
| 9947 | mainnet v2 RPC (planned) | not yet listening |
| 9616 | testnet Alice Prometheus | [verified via cmdline] |
| 9617 | mainnet v1 Prometheus | [verified via cmdline] |
| 30334 | testnet Alice libp2p | [verified via cmdline] |
| 30335 | mainnet v1 libp2p | [verified via cmdline] |

---

## 10. File map — new / changed today

All paths are absolute.

### 10.1 Created today

| Path | Size | Purpose | Source |
|---|---|---|---|
| `/opt/plimlab/plim-protocol/plim-chain/BOOTSTRAP_ROADMAP.md` | 300 lines | full mainnet v2 bootstrap plan | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/TOKEN_CATALOG.md` | 181 lines, 8 TBDs | token inventory + legal questions | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/STATUS.md` | this file | single source of truth | [verified — you're reading it] |
| `/opt/plimlab/plim-protocol/plim-chain/CHAIN_SPEC_HARDENING.md` | 4094 bytes | hardening notes | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-v2-raw.json` | 984 KB | raw chain spec for v2 | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/MAINNET_KEYS_BACKUP.md.MOVED` | 122 bytes | placeholder where old backup doc used to be | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json.MOVED` | 216 bytes | placeholder where v1 raw spec used to be | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json.MOVED` | 146 bytes | placeholder where v1 human spec used to be | [verified] |
| `/opt/plimlab/plim-protocol/plim-gateway/src/lib/chain.ts` | (singleton) | `@polkadot/api` wrapper | [verified] |
| `/opt/plimlab/node-tools-ecofi-plimlab/monitoring/prometheus/alerts/chain-bootstrap.yml` | 205 lines, 11 alert rules | chain liveness alerts | [verified] |
| `/root/SECURE/mainnet_v2_keys_2026-04-14.txt` | 3436 bytes, 600 | encrypted v2 keys | [verified] |
| `/root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet.alice-sudo.json` | | archived v1 human spec | [verified] |
| `/root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json` | | archived v1 raw spec | [verified] |
| `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/` | **16 GB total** | rsync snapshot of mainnet + 5 testnet validators | [verified] |
| `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/MANIFEST.sha256` | | sha256 of every file | [verified] |
| `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/SIZES.txt` | | per-directory sizes | [verified] |
| `/root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/README.md` | | restore procedure | [verified] |
| `/root/SECURE/keys/plim_mainnet_sudo.json` | | v2 sudo signer | [verified] |
| `/root/SECURE/keys/plim_mainnet_treasury.json` | | v2 treasury signer | [verified] |
| `/root/SECURE/keys/plim_mainnet_team_vesting.json` | | v2 team-vesting signer | [verified] |
| `/root/SECURE/keys/plim_mainnet_aura.json` | | v2 Aura block-author key | [verified] |
| `/root/SECURE/keys/plim_mainnet_grandpa.json` | | v2 GRANDPA finality key | [verified] |
| `MAINNET_V2_DEPLOY.md` (task #44, location TBD) | planned | runbook for v2 deploy | [reported] |
| `plim-node-mainnet-v2.service.template` (task #44, location TBD) | planned | systemd unit template | [reported] |
| `traefik/dynamic/mainnet-v2.yml` (task #44, location TBD) | planned | Traefik route template | [reported] |

### 10.2 Modified today

| Path | Change | Source |
|---|---|---|
| `/opt/plimlab/plim-protocol/plim-chain/Cargo.toml` | added pallet-assets@42 | [reported] |
| `/opt/plimlab/plim-protocol/plim-chain/Cargo.lock` | touched 2026-04-14T15:01 | [verified — mtime] |
| `/opt/plimlab/plim-protocol/plim-chain/runtime/src/lib.rs` | pallet-assets index 8, removed pallet-template | [reported] |
| `/opt/plimlab/plim-protocol/plim-chain/runtime/src/genesis_config_presets.rs` | added `mainnet_genesis()` | [reported] |
| `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-payments/` | real `pay` extrinsic + 3 unit tests | [reported] |
| `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-identity/{Cargo.toml,src/lib.rs}` | dirty in worktree | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-mandates/{Cargo.toml,src/lib.rs}` | dirty in worktree | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-reputation/{Cargo.toml,src/lib.rs}` | dirty in worktree | [verified] |
| `/opt/plimlab/plim-protocol/plim-chain/pallets/plim-timestamps/{Cargo.toml,src/lib.rs}` | dirty in worktree | [verified] |
| `/opt/plimlab/plim-protocol/plim-gateway/package.json` | added `@polkadot/api`, `@polkadot/keyring` | [reported] |
| `/opt/plimlab/plim-protocol/plim-gateway/package-lock.json` | touched 2026-04-14T15:24 | [verified — mtime] |
| `/opt/plimlab/plim-protocol/plim-gateway/src/routes/tokens.ts` | added `GET /tokens/chain-info` | [verified via grep] |
| `/opt/plimlab/plimcontrol/src/pages/...` (3 token pages) | added pUSD sibling to pEUR | [reported] |
| `/opt/plimlab/plim-protocol/docker-compose.yml` | touched 2026-04-14T15:24 | [verified — mtime] |
| `/opt/plimlab/plim-protocol/nginx.conf` | touched 2026-04-14T14:56 | [verified — mtime] |
| `/etc/fstab` | `+4G swapfile2` added | [reported] |

### 10.3 Commits

| Repo | SHA | Branch | Message |
|---|---|---|---|
| `plim-chain` | `29755bd` | `wip/2026-04-14-server-fixes` | runtime: add pallet-assets, real PlimPayments, mainnet v2 clean genesis [verified] |
| `plim-gateway` | `1f6fc40` | (current) | feat(gateway): add Substrate client foundation + /tokens/chain-info POC [verified] |
| `plimcontrol` | `cc83beb` | `wip/pusd-stablecoin` | pUSD UI | [reported] |
| `plimcontrol` | `028c6b9` | (earlier) | pBRL removal | [reported] |

---

## 11. Rollback procedures

For each major change today: how to undo.

### 11.1 Undo the runtime overhaul (commit 29755bd)

```bash
cd /opt/plimlab/plim-protocol/plim-chain
git checkout wip/2026-04-14-server-fixes
git revert 29755bd
# Then re-run cargo check to confirm revert is clean
cargo check -p plim-runtime
```

**Side effects:** will remove pallet-assets, restore pallet-template, drop PlimPayments::pay,
drop `mainnet_genesis()` preset. Will NOT delete `/root/SECURE/mainnet_v2_keys_2026-04-14.txt` —
those stay because they're out-of-tree.

### 11.2 Undo the chain-spec evacuation (task #31)

```bash
# The archived specs are root-owned in /root/SECURE/
sudo cp /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet.alice-sudo.json \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json
sudo cp /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json
sudo chown plimadmin:plimadmin /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet*.json
# Remove the .MOVED placeholders
sudo rm /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json.MOVED \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json.MOVED
```

**Side effects:** re-arms v1 for reboot. Don't do this unless you're also prepared to stop
v2 and re-bless Alice sudo.

### 11.3 Undo the gateway chain client (commit 1f6fc40)

```bash
cd /opt/plimlab/plim-protocol/plim-gateway
git revert 1f6fc40
# This reverts package.json + src/lib/chain.ts + src/routes/tokens.ts changes
# Then reinstall the old deps
rm -rf node_modules package-lock.json
npm install
sudo systemctl restart plim-gateway
```

### 11.4 Undo the plimcontrol pUSD UI (commit cc83beb)

```bash
cd /opt/plimlab/plimcontrol
git checkout wip/pusd-stablecoin
git revert cc83beb
npm run build
sudo systemctl restart plimcontrol  # or: docker compose up -d
```

### 11.5 Undo the chain-bootstrap alerts (task #37)

```bash
# Remove the alert file
sudo rm /opt/plimlab/node-tools-ecofi-plimlab/monitoring/prometheus/alerts/chain-bootstrap.yml
# Reload prometheus
sudo systemctl reload prometheus  # or: docker compose restart prometheus
```

### 11.6 Delete the 16 GB snapshot (task #38)

```bash
# Only if you need the disk back AND you've verified mainnet v2 is stable
sudo rm -rf /root/SECURE/chain-snapshots/2026-04-14T15-00-17Z/
```

**DO NOT DO THIS UNTIL MAINNET v2 HAS BEEN STABLE FOR AT LEAST 7 DAYS.**

### 11.7 Undo the apt upgrade (not feasible)

The 41-package apt safe-tier upgrade cannot be cleanly reverted. If a regression is found,
identify the specific package and `apt install <pkg>=<old-version>` with `--allow-downgrades`.
Keep `/var/log/apt/history.log` handy.

### 11.8 Undo the swap file addition

```bash
sudo swapoff /mnt/data/swapfile2
sudo sed -i '/swapfile2/d' /etc/fstab
sudo rm /mnt/data/swapfile2
```

### 11.9 Undo the SSH key addition

```bash
# Remove the MacBook public key from authorized_keys
sudo sed -i '/macbook/d' /home/plimadmin/.ssh/authorized_keys
```

(Exact comment string TBD — confirm before running.)

---

## 12. Open TBDs index (for fast triage)

1. PLIM emission policy beyond 1B genesis
2. pEUR issuer-of-record jurisdiction
3. pUSD legal framework
4. Mainnet v2 exact genesis allocations (60/20/20 proposed)
5. Mainnet v2 epoch / era length
6. pEUR reserve buffer percentage
7. Sudo multisig threshold (3-of-5 proposed)
8. ePL mainnet activation gate
9. gPLIM mainnet activation gate
10. plimliftgrid substrate state (SSH blocked)
11. Traefik dynamic route path for `mainnet-v2.protocol.plimlab.ch`
12. Final chain-bootstrap alert label matchers (promtool PASS but not battle-tested)
13. `MAINNET_V2_DEPLOY.md` runbook location (task #44)
14. Systemd unit template location (task #44)
15. `plimcontrol` branch consolidation state (task #43)
16. `npm install` + gateway restart outcome (task #41)
17. `cargo build --release` final artifact diff vs current binary (task #39)
18. 7-pallet refactor: final API surface (task #40)
19. Genesis-mismatch root cause in testnet-5val spec restore

---

## 13. How to update this document

When a task ticks over from "in progress" to "done":

1. Update §5 with the commit SHA and verification source.
2. Update §4 build state table.
3. Remove or downgrade the corresponding §6 risk.
4. Promote / demote §7 next actions.
5. Add any new files to §10 file map.
6. Bump the **Last updated** timestamp at the top.
7. Mark resolved items in §12 (strike-through or remove).

Keep facts verifiable. Prefer `[verified]` tags with the exact command used over `[reported]`
tags that depend on trusting another agent.

---

*End of STATUS.md*
