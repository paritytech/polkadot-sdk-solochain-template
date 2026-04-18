# Chain-Spec Hardening — 2026-04-14

Agent 2 of 9 (parallel hardening run). Performed during the 2026-04-13/14 session to stop a future restart from (a) handing over mainnet sudo to a dev-seed attacker and (b) silently failing the 5-validator testnet.

## What was moved

| Previous path | New path | Reason |
|---|---|---|
| `/opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json` | `/root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet.alice-sudo.json` | sudo key = Alice dev seed `5GrwvaEF...utQY` — publicly known, anyone could take mainnet |
| `/opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json` | `/root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json` | Raw variant of the same compromised spec referenced by `plim-node-mainnet.service` |

Vault perms: `700` on dir, `600 root:root` on files.

SHA256 of archived files:

```
8bff7efe6264d3e1d64fa2e444a71df452026234a344e5aa940048df7bc06e82  chain-spec-mainnet-raw.alice-sudo.json
da5470639e4e3331afebaf46a0f2d2bb9091a9442777cee9919d69eeb5f1d8fc  chain-spec-mainnet.alice-sudo.json
```

Stub breadcrumbs left at old paths (owned by root):

- `chain-spec-mainnet.json.MOVED`
- `chain-spec-mainnet-raw.json.MOVED`

## Why the running mainnet kept running

- `plim-node-mainnet.service` has `Restart=always`.
- Substrate loads the chain-spec JSON once at startup then closes the fd (verified via `/proc/3867217/fd/` — no json handle open).
- Moving the file therefore does NOT affect the running validator. A manual restart (or an OOM → respawn) would fail because the file is gone. That is the desired behaviour — we do not want this spec coming back without explicit operator action.

## What is now BLOCKED on restart

- `plim-node-mainnet.service` will fail-restart until a new `chain-spec-mainnet-raw.json` is produced from a clean, non-Alice sudo key. Document that procedure in `BOOTSTRAP_ROADMAP.md` (not this file's responsibility).

## Testnet 5-val spec — NOT restored (FLAGGED AS RISK)

`plim-node-testnet*.service` all reference `chain-spec-testnet-5val-raw.json`, which does not exist on disk. The running validators (alice/bob/charlie/dave/eve) are alive but only because Substrate already loaded the file during their Apr 8 startup — no fd is open so `/proc/$PID/fd/` cannot be used to recover it.

Compared the existing `chain-spec-testnet-raw.json` genesis hash against live testnet RPC:

- Running testnet (`localhost:9945`) genesis: `0xe8399f148c4eb872d24dd5b2773ee3231723c5fd88a96f0295027f904cf5e2ae`
- `chain-spec-testnet-raw.json` genesis when booted into a tmp base-path: `0xd5c7…8384`

Mismatch — they are different chains. Renaming `chain-spec-testnet-raw.json` to `-5val-raw.json` would cause a genesis-mismatch abort on restart.

**Result: nothing was restored. Testnet will break on next reboot / service restart.** The 5-validator raw spec must be rebuilt from source (requires the correct preset id and matching runtime WASM used at Apr 8 startup).

## Emergency rollback (if mainnet/testnet needs the Alice-sudo spec back)

```bash
sudo cp /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet.alice-sudo.json \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json
sudo cp /root/SECURE/chain-specs-archived-2026-04-14/chain-spec-mainnet-raw.alice-sudo.json \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json
sudo chown plimadmin:plimadmin /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet*.json
sudo rm /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet.json.MOVED \
        /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-raw.json.MOVED
```

Only do this in a break-glass scenario — it re-exposes mainnet sudo to the Alice dev seed.

## Verification after the move

- `systemctl is-active` on all 6 plim-node services: `active` x6
- `http://localhost:9946/system_health` (mainnet): 200, peers=0, not syncing
- `http://localhost:9945/system_health` (testnet alice): 200, peers=4, not syncing
- No service restart triggered during the hardening pass.
