# Multisig Sudo Plan — pLim Chain Mainnet v2

**Author:** pLim/lab  ops (2026-04-14)
**Runtime target:** `spec_version 101 -> 102`
**Pallet added:** `pallet-multisig` at index `9`
**Goal:** Replace the single-key sudo on mainnet v2 with a 3-of-5 threshold
multisig, then progressively deprecate sudo in favour of a governance pallet.

> This document is the single maintenance-window runbook. It is intentionally
> self-contained: anyone with SSH+root on `91.99.60.74` and one cosigner
> mnemonic should be able to follow it end-to-end.

---

## 1. Why multisig (the Alice-sudo learning)

Mainnet v1 (now retired) and the genesis of mainnet v2 were bootstrapped with
a single-key `pallet-sudo`. That was fine for the dev phase, but it has two
chronic problems:

1. **One compromised key = full chain compromise.** If the sudo mnemonic leaks,
   the attacker can push any runtime upgrade, mint arbitrary PLIM, rotate
   validators, and drain every mandate. This is the "Alice sudo" anti-pattern
   and we explicitly committed (see `BOOTSTRAP_ROADMAP.md` §6) to leave it
   behind before public onboarding.
2. **No four-eyes review of privileged calls.** Every sudo call today is a
   single-operator decision with no on-chain audit trail of who co-signed it.

A 3-of-5 multisig fixes both problems without introducing a full governance
framework (which is planned for runtime v2, later in 2026). It buys us:

- **Fault tolerance:** losing up to 2 cosigner keys still leaves the chain
  governable.
- **Collusion resistance:** any 2 cosigners cannot push an upgrade alone.
- **On-chain audit trail:** every `Multisig::as_multi` call emits
  `NewMultisig` / `MultisigApproval` / `MultisigExecuted` events that tie each
  cosigner to the call hash.
- **Zero chain-spec change:** the multisig account is derived deterministically
  from the 5 cosigner pubkeys + threshold, so we don't need a new genesis.

---

## 2. The 5 cosigners

Five sr25519 keys were generated offline on 2026-04-14. Mnemonics are stored
as JSON files at:

```
/root/SECURE/multisig_v1_2026-04-14/
    cosigner_1.json   (chmod 600 root:root)
    cosigner_2.json
    cosigner_3.json
    cosigner_4.json
    cosigner_5.json
```

Each file is the raw `plim-node key generate --output-type json` output
(`secretPhrase`, `secretSeed`, `publicKey`, `ss58Address`, etc.) and MUST
never leave this directory in plaintext. Backups go to the Bitwarden vault
under the `pLim Chain / Multisig v1 (2026-04-14)` collection, with each
cosigner mnemonic stored as a separate item.

### Cosigner SS58 addresses (SS58 prefix 42, substrate generic)

| # | Role (proposed)               | SS58 address                                       |
|---|-------------------------------|----------------------------------------------------|
| 1 | Ops lead (primary)            | `5CS3ivzz7P7vvqHgw3nS5FrsZRPT3tsPg3pqb9mCWyD8L4rj` |
| 2 | Protocol lead                 | `5GnVp6CnCuvCYgodFqKjPBVkgJnWmssY6oPTuHa11LF23WXF` |
| 3 | Runtime / chain dev           | `5GqA7bNBYYQJUUhVftrmZXG4BksiTRhs8SjveYffkk417NAx` |
| 4 | Cold backup (offline vault)   | `5EKsSgHU1LnQUTRK237AamSMjpASvPwFzvywEjPtCgskTFbH` |
| 5 | Cold backup (hardware wallet) | `5HDuKhd8mprv97Z6RFdYccXkp3wN5QCLTaCFJw2srcrN8acD` |

Role assignment is a policy decision for the next ops sync — the keys
themselves are fungible until a cosigner claims one.

---

## 3. The 3-of-5 multisig address

Derived deterministically via the polkadot-sdk formula
`(b"modlpy/utilisuba", who_sorted, threshold).using_encoded(blake2_256)`
with `threshold = 3` and `who_sorted` = the 5 public keys above sorted
ascending by raw bytes.

**Multisig SS58 (FUTURE sudo key):**

```
5Fug4agEPcWGE2pjUUv4quz2Aeig1HAKGHWjo3nDCJyVMnoX
```

This is the account that will own `pallet_sudo::Key` starting in step 4.4
of the maintenance window. It has **no on-chain existence** until the first
`as_multi` call lands (pallet-multisig creates the reserved entry lazily).

Polkadot-JS Apps reference (for manual verification after the upgrade):
`Developer -> Utilities -> multisig -> From keys` -> paste the 5 addresses,
set threshold `3`, copy address. It must match the value above exactly.

---

## 4. Runtime upgrade path (spec_version 101 -> 102)

Source changes already committed to `wip/2026-04-14-server-fixes` in
`plim-chain`:

- `runtime/Cargo.toml` — adds `pallet-multisig 40.1.0` to deps, std feature
  list, `runtime-benchmarks`, `try-runtime`.
- `Cargo.toml` (workspace) — workspace dep entry for `pallet-multisig`.
- `runtime/src/configs/mod.rs` — `impl pallet_multisig::Config for Runtime`
  with `DepositBase = 1 UNIT`, `DepositFactor = UNIT/20`, `MaxSignatories =
  16`, `BlockNumberProvider = System`.
- `runtime/src/lib.rs` — `#[runtime::pallet_index(9)] pub type Multisig =
  pallet_multisig;` and `spec_version: 101 -> 102`.

No changes to any pallet, to `node/`, or to the chain-spec JSON. The upgrade
is purely a runtime WASM swap dispatched by the current sudo key.

**Invariants held:**

- Pallet indices 0-8 and 10-17 unchanged (storage migrations not required).
- Existing sudo key `5FeRSViQcSQv6xe4Z7imXtVfTxfVY2z9Cm45NPqwFVJtzTw9`
  still owns `Sudo::Key` until we explicitly call `Sudo::set_key`.
- No migrations block: `type Migrations = ();` — pallet-multisig has no
  genesis state, so empty storage is correct.

---

## 5. Maintenance window runbook (5 steps)

Estimated window: **30 minutes** including verification. Announce to
Discord #ops-mainnet 15 minutes before step 1.

### Step 1 — Build the runtime (~15 min on the validator)

```bash
cd /opt/plimlab/plim-protocol/plim-chain
git checkout wip/2026-04-14-server-fixes
git pull
CARGO_TARGET_DIR=/mnt/data/cargo-target \
  cargo build --release -p plim-runtime --features on-chain-release-build
ls -la /mnt/data/cargo-target/release/wbuild/plim-runtime/plim_runtime.compact.compressed.wasm
```

Copy the compressed WASM somewhere the next step can reach:

```bash
cp /mnt/data/cargo-target/release/wbuild/plim-runtime/plim_runtime.compact.compressed.wasm \
   /tmp/plim_runtime_v102.compact.compressed.wasm
```

### Step 2 — Dry-run the upgrade against a fork (optional but recommended)

Use `try-runtime-cli` to replay the new runtime on a snapshot of the live
state. Skip only if the window is tight.

```bash
try-runtime --runtime /tmp/plim_runtime_v102.compact.compressed.wasm \
  on-runtime-upgrade --checks all \
  live --uri wss://mainnet-v2.protocol.plimlab.ch
```

Abort the whole window if any migration check fails.

### Step 3 — Submit `System::set_code` via the current single-key sudo

From Polkadot-JS Apps connected to `wss://mainnet-v2.protocol.plimlab.ch`
with the existing sudo mnemonic loaded:

```
Developer -> Sudo -> sudoUncheckedWeight
   call:        system.setCode(code)
   code:        <upload /tmp/plim_runtime_v102.compact.compressed.wasm>
   weight:      (ref_time 0, proof_size 0)
```

Wait for 2 finalised blocks. Confirm `spec_version` on-chain is now `102`:

```
RPC -> state.getRuntimeVersion()    -> expect specVersion: 102
```

Also confirm `Multisig` pallet shows up in metadata:

```
RPC -> state.getMetadata()         -> grep "multisig"   (palletIndex 9)
```

### Step 4 — Transfer sudo to the multisig

Still from the existing sudo key, one single transaction:

```
Developer -> Sudo -> sudo
   call:  sudo.setKey(new)
   new:   5Fug4agEPcWGE2pjUUv4quz2Aeig1HAKGHWjo3nDCJyVMnoX
```

Wait 1 finalised block. Verify:

```
Chain State -> sudo.key()
   -> 5Fug4agEPcWGE2pjUUv4quz2Aeig1HAKGHWjo3nDCJyVMnoX
```

From this moment the old mnemonic
`5FeRSViQcSQv6xe4Z7imXtVfTxfVY2z9Cm45NPqwFVJtzTw9` is powerless (but NOT
deleted from `/root/SECURE/mainnet_v2_keys_2026-04-14.txt` — keep it for
90 days as a rollback asset, see §7).

### Step 5 — Smoke-test the multisig

Fund each cosigner with ~5 PLIM for transaction fees (balances.transfer
from the treasury / ops account — this does NOT require sudo). Then run a
no-op privileged call through the multisig to prove the path is live:

```
# cosigner 1 initiates
Developer -> Extrinsics (as cosigner_1)
   multisig.asMulti(
      threshold: 3,
      otherSignatories: [sorted list of cosigners 2,3,4,5],
      maybeTimepoint: null,
      call: sudo.sudo( system.remark( 0x706c696d2d6d756c74697369672d76312d6f6b ) ),  // "plim-multisig-v1-ok"
      maxWeight: { refTime: 1_000_000_000, proofSize: 100_000 }
   )

# cosigner 2 approves
Developer -> Extrinsics (as cosigner_2)
   multisig.approveAsMulti(
      threshold: 3,
      otherSignatories: [sorted list of cosigners 1,3,4,5],
      maybeTimepoint: {height, index}   <-- from step 5a event
      callHash: <hash from step 5a>
      maxWeight: { ... }
   )

# cosigner 3 executes
Developer -> Extrinsics (as cosigner_3)
   multisig.asMulti(
      threshold: 3,
      otherSignatories: [sorted list of cosigners 1,2,4,5],
      maybeTimepoint: {height, index}
      call: sudo.sudo( system.remark( 0x706c696d2d6d756c74697369672d76312d6f6b ) )
      maxWeight: { ... }
   )
```

Expected event sequence:
`multisig.NewMultisig -> multisig.MultisigApproval -> multisig.MultisigExecuted(Ok)`
plus `sudo.Sudid(Ok)` plus `system.Remarked`.

**If the smoke test fails, immediately execute the rollback in §7.**

---

## 6. Eventual deprecation path (runtime v3)

The multisig is a bridge, not a destination. The target state is:

1. **Runtime 103** — add `pallet-collective` + `pallet-scheduler`, wire a
   council origin to the privileged calls currently gated on `EnsureRoot`.
2. **Runtime 104** — add `pallet-democracy` (or `pallet-referenda` +
   `pallet-conviction-voting`) for on-chain token-holder governance.
3. **Runtime 105** — push one final `Sudo::sudo(Sudo::set_key(zero))` — and
   only *after* the governance pallets have exercised at least one
   successful runtime upgrade — then `Sudo::remove_key` via the multisig,
   effectively deleting sudo forever.

Until then, the multisig is the single privileged actor and its 5 cosigners
must rotate keys annually (new mnemonics, new derivation, new
`sudo.set_key`).

---

## 7. Rollback procedure

If **step 3** fails (runtime upgrade refused / chain stalls):
- The chain will simply keep running the old WASM. No action needed beyond
  diagnosing the failure and cancelling the window.

If **step 4** succeeds but **step 5** shows the multisig is non-functional
(e.g. wrong SS58, one mnemonic unrecoverable, deposit too high to pay):
- Immediately use the OLD sudo mnemonic. It is NOT yet deleted from
  `/root/SECURE/mainnet_v2_keys_2026-04-14.txt`.
- `Sudo::set_key(5FeRSViQcSQv6xe4Z7imXtVfTxfVY2z9Cm45NPqwFVJtzTw9)` — wait,
  this fails because the old key is no longer `Sudo::Key`. Instead use:
- From any cosigner that IS working, go through a 3-of-5 `as_multi` wrap of
  `sudo.sudo(sudo.setKey(OLD_KEY))` to hand sudo back to the single-key
  owner. This requires at least 3 cosigners to be operational, which is
  why we insisted on 3-of-5 and not 4-of-5.
- If fewer than 3 cosigners are operational, the chain is stuck on the new
  runtime without a usable sudo until we can recover more keys. In that
  (extreme) scenario, the only escape is a **forked chain-spec** restart —
  hence the 90-day retention of the old sudo mnemonic AND a signed
  chain-spec snapshot backed up in Borg the morning of the window.

**Retention policy:** `/root/SECURE/mainnet_v2_keys_2026-04-14.txt` is
deleted only after 90 days of clean operation with the multisig and only
after the governance pallets land in runtime 103+.

---

## 8. Appendix — checklist

- [ ] Borg snapshot of `/var/lib/plim-node-mainnet` taken within 1 h of window
- [ ] `try-runtime on-runtime-upgrade` dry-run green
- [ ] 5 cosigner SS58 addresses confirmed against
      `/root/SECURE/multisig_v1_2026-04-14/cosigner_{1..5}.json`
- [ ] Multisig SS58 re-computed in Polkadot-JS Apps and matches §3
- [ ] All 5 cosigner mnemonics mirrored to Bitwarden before the window
- [ ] Discord #ops-mainnet announcement sent
- [ ] WASM build artefact size < 5 MB (sanity check — v101 is ~4 MB)
- [ ] Maintenance window timer started

---
