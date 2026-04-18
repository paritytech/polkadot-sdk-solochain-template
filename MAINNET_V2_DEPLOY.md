# pLim Chain Mainnet v2 — Deployment Runbook

**Date drafted:** 2026-04-14
**Target host:** `tools-aifirst-v1` (91.99.60.74)
**Goal:** Bring up a fresh-genesis mainnet v2 alongside v1 without disturbing v1 until we deliberately cut over.

---

## 0. Port Allocation (confirmed free at draft time)

| Role        | v1 (running) | v2 (this runbook) |
|-------------|--------------|-------------------|
| RPC         | 9946         | **9947**          |
| p2p         | 30335        | **30336**         |
| prometheus  | 9617         | **9618**          |
| base-path   | `/mnt/data/plim-chain-mainnet` | `/mnt/data/plim-chain-mainnet-v2` |
| public host | `mainnet.protocol.plimlab.ch` | `mainnet-v2.protocol.plimlab.ch` |

(The `9615` localhost node and `9616` testnet prometheus are unrelated but noted for completeness.)

---

## 1. Preconditions

Before doing anything below, verify ALL of the following:

```bash
# 1. New binary exists (cargo build finished)
ls -la /mnt/data/cargo-target/release/plim-node
/mnt/data/cargo-target/release/plim-node --version

# 2. Fresh chain spec exists
ls -la /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-v2-raw.json
jq '.name, .id, .chainType' /opt/plimlab/plim-protocol/plim-chain/chain-spec-mainnet-v2-raw.json

# 3. Session keys present (do NOT cat them)
sudo ls -la /root/SECURE/keys/
# Expect: plim_mainnet_aura.json, plim_mainnet_grandpa.json,
#         plim_mainnet_sudo.json, plim_mainnet_team_vesting.json,
#         plim_mainnet_treasury.json

# 4. v2 base-path does NOT exist yet
ls -la /mnt/data/plim-chain-mainnet-v2 2>/dev/null && echo "ERROR: already exists" || echo "OK: fresh"

# 5. Templates are in place
sudo ls -la /etc/systemd/system/plim-node-mainnet-v2.service.template
ls -la /opt/plimlab/node-tools-ecofi-plimlab/traefik/dynamic/mainnet-v2.yml.template
```

---

## 2. Pre-deploy checks — DO NOT BREAK v1

```bash
# v1 must stay green throughout this runbook.
systemctl is-active plim-node-mainnet
sudo ss -tlnp | grep -E ':(9946|30335|9617)'
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
     http://127.0.0.1:9946 | jq .
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_chain","params":[]}' \
     http://127.0.0.1:9946 | jq .
```

Take a screenshot / keep this terminal open — you will re-run these after activation to confirm v1 is unchanged.

Also verify v2 target ports are free:

```bash
sudo ss -tlnp | grep -E ':(9947|30336|9618)' && echo "ERROR: collision" || echo "OK: v2 ports free"
```

---

## 3. Activation

### 3a. Create the fresh base-path

```bash
sudo mkdir -p /mnt/data/plim-chain-mainnet-v2
sudo chown -R plimadmin:plimadmin /mnt/data/plim-chain-mainnet-v2
```

### 3b. Copy template to live systemd unit

```bash
sudo cp /etc/systemd/system/plim-node-mainnet-v2.service.template \
        /etc/systemd/system/plim-node-mainnet-v2.service
sudo systemctl daemon-reload
sudo systemctl enable --now plim-node-mainnet-v2.service
```

### 3c. Watch the logs for ~60s

```bash
sudo journalctl -u plim-node-mainnet-v2 -f
# Ctrl-C once you see:
#   "Running JSON-RPC server: addr=0.0.0.0:9947"
#   "Idle (0 peers)"
#   "Prepared block for proposing at 1 ..."  (single validator seals solo)
```

### 3d. Insert Aura + Grandpa session keys

These are inserted via RPC against the running v2 node. Use the JSON files in `/root/SECURE/keys/`. **DO NOT log the seed to shell history** — use a heredoc or `--data-binary @-` from a root-only file.

```bash
# Example pattern — adjust field names to the actual key file shape.
sudo bash -c '
  AURA=$(jq -r .secretPhrase /root/SECURE/keys/plim_mainnet_aura.json)
  PUB=$(jq -r .publicKey /root/SECURE/keys/plim_mainnet_aura.json)
  curl -s -H "Content-Type: application/json" \
       -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"author_insertKey\",\"params\":[\"aura\",\"${AURA}\",\"${PUB}\"]}" \
       http://127.0.0.1:9947
'
# Repeat for "gran" (Grandpa) with plim_mainnet_grandpa.json
```

After both keys are inserted, restart the node so it picks them up on a fresh session:

```bash
sudo systemctl restart plim-node-mainnet-v2
```

---

## 4. Validation

```bash
# Chain identity (expect "Plim Chain Mainnet v2" or however the spec is named)
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_chain","params":[]}' \
     http://127.0.0.1:9947 | jq .result

# Genesis hash — MUST differ from v1's genesis hash
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"chain_getBlockHash","params":[0]}' \
     http://127.0.0.1:9947 | jq .result

# Current head — should advance every 6s if validator is sealing
for i in 1 2 3; do
  curl -s -H "Content-Type: application/json" \
       -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
       http://127.0.0.1:9947 | jq -r .result.number
  sleep 6
done

# Health
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
     http://127.0.0.1:9947 | jq .

# Re-confirm v1 is unchanged
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
     http://127.0.0.1:9946 | jq .
```

Expected: v2 peers=0 (single validator — this is OK for launch), height advancing, genesis hash ≠ v1.

---

## 5. Public exposure via Traefik

```bash
cd /opt/plimlab/node-tools-ecofi-plimlab/traefik/dynamic/
mv mainnet-v2.yml.template mainnet-v2.yml
# Traefik file provider auto-reloads within ~2s.

# Verify external reachability (after DNS is in place — see §6)
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_chain","params":[]}' \
     https://mainnet-v2.protocol.plimlab.ch | jq .result
```

---

## 6. DNS

`mainnet-v2.protocol.plimlab.ch` must resolve to `91.99.60.74`.

- If `*.protocol.plimlab.ch` wildcard already exists in the zone → nothing to do.
- Otherwise add an A record: `mainnet-v2.protocol.plimlab.ch  A  91.99.60.74  300`.

Verify:

```bash
dig +short mainnet-v2.protocol.plimlab.ch
```

---

## 7. Monitoring

```bash
# Prometheus scrape endpoint
curl -s http://127.0.0.1:9618/metrics | head -30
```

Add a scrape target for `localhost:9618` in the Prometheus config (or whichever label system the chain-bootstrap alerts use). The existing chain-bootstrap alerts should key off `job="plim-node"` with a new `instance` label — no rule changes needed, just a new target line. Confirm v2 metrics flow before silencing any alerts.

---

## 8. Promotion / cutover (v2 → primary, v1 → deprecated)

Only run this AFTER the user explicitly signs off that v2 is healthy.

```bash
# 1. Stop v1
sudo systemctl disable --now plim-node-mainnet

# 2. Swap Traefik: mainnet.protocol.plimlab.ch -> 9947
#    Edit /opt/plimlab/node-tools-ecofi-plimlab/traefik/dynamic/mainnet.yml
#    Change the backend URL from http://host.docker.internal:9946
#                              to http://host.docker.internal:9947
#    (Traefik auto-reloads.)

# 3. Archive the v1 chain data (do NOT delete yet — keep for forensics)
sudo mv /mnt/data/plim-chain-mainnet /mnt/data/plim-chain-mainnet.v1-archived-$(date +%F)

# 4. Rename the v1 systemd unit out of the way
sudo mv /etc/systemd/system/plim-node-mainnet.service \
        /etc/systemd/system/plim-node-mainnet.service.v1-archived
sudo systemctl daemon-reload
```

At this point `mainnet.protocol.plimlab.ch` AND `mainnet-v2.protocol.plimlab.ch` both point at the v2 node. The `-v2` hostname stays as an alias for a grace period, then can be retired.

---

## 9. Rollback

If ANYTHING looks wrong during §3–§5:

```bash
# Disable v2 immediately
sudo systemctl disable --now plim-node-mainnet-v2.service
sudo rm /etc/systemd/system/plim-node-mainnet-v2.service
# (the .template file stays for the next attempt)

# Revert Traefik
cd /opt/plimlab/node-tools-ecofi-plimlab/traefik/dynamic/
mv mainnet-v2.yml mainnet-v2.yml.template 2>/dev/null || true

# Optional: wipe the fresh v2 base-path so the next attempt is clean
sudo rm -rf /mnt/data/plim-chain-mainnet-v2

# v1 is untouched on 9946 — verify
systemctl is-active plim-node-mainnet
curl -s -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
     http://127.0.0.1:9946 | jq .
```

---

## 10. Known limitations at v2 launch

- **Single validator**: `peers=0` at start is expected. Adding more validators is a post-launch task (add Aura authority, sync, then `session.setKeys`).
- **Session keys must be inserted post-start** via `author_insertKey` against the running node (see §3d). The systemd unit alone does NOT inject keys.
- **Sudo is still a single key** (`plim_mainnet_sudo.json`). The DKG / multisig sudo cutover is tracked separately and is NOT part of this runbook.
- **RPC is `Safe` + rate-limited** (100 req/s). Any tooling that relied on `Unsafe` methods against v1 (`unsafe-rpc-external`) will break against v2 — this is deliberate hardening.
- **No mDNS** in prod (`--no-mdns`).
- **Genesis hash differs from v1** — any indexer, subsquid, or explorer pointed at mainnet must be re-pointed and re-indexed from block 0.
- **Asset metadata** is pre-seeded in genesis per `CHAIN_SPEC_HARDENING.md`; verify via `assets.metadata` storage after launch.

---

## Appendix: the 4-command activation sequence

Once every precondition in §1 is green:

```bash
sudo mkdir -p /mnt/data/plim-chain-mainnet-v2 && sudo chown -R plimadmin:plimadmin /mnt/data/plim-chain-mainnet-v2
sudo cp /etc/systemd/system/plim-node-mainnet-v2.service.template /etc/systemd/system/plim-node-mainnet-v2.service
sudo systemctl daemon-reload && sudo systemctl enable --now plim-node-mainnet-v2.service
sudo journalctl -u plim-node-mainnet-v2 -f
```
