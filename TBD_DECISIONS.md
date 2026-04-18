# TBD_DECISIONS.md — Actionable Defaults for TOKEN_CATALOG.md Open Questions

**Document purpose:** Convert the 8 open TBDs in `TOKEN_CATALOG.md` into concrete, accept-or-reject proposals. Each TBD below ships with a default recommendation, 1–2 alternatives, a trade-off summary, a check-box for user approval, and the exact follow-up action that fires once the box is ticked.

**How to use this doc:**
1. Read each TBD section (they are short on purpose).
2. Tick the box next to the option you want — default recommendation, alternative A, or alternative B.
3. Hand the ticked doc back to the agent pool. Each ticked decision maps 1:1 to a concrete code/config/legal task.
4. If you want to defer any single TBD, leave **all** boxes unchecked on that TBD and write `DEFER` in the Decision line.

**Scope guarantee:** This doc only proposes. No code, runtime, chainspec, or genesis file has been touched by this agent. Approval is required before any of the "Action triggered if accepted" items execute.

**Date:** 2026-04-13
**Target chain:** pLim Mainnet v2 (Substrate L1, spec_version 102 currently)
**Related docs:** `TOKEN_CATALOG.md`, `MAINNET_V2_DEPLOY.md`, `CHAIN_SPEC_HARDENING.md`, `BOOTSTRAP_ROADMAP.md`

---

## Summary table

| # | TBD                              | Default recommendation                          | Blocks               | Risk if deferred           |
| - | -------------------------------- | ------------------------------------------------ | -------------------- | -------------------------- |
| 1 | PLIM max supply                  | 2B cap, 1B genesis + 1B inflation over 20yr     | Runtime upgrade      | Validator reward funding   |
| 2 | pEUR issuer jurisdiction         | Liechtenstein FMA (TVTG)                         | Legal entity         | Cannot issue MiCA-compliant pEUR |
| 3 | pUSD legal framework             | Partner-wrap Circle USDC 1:1                     | Circle partnership   | pUSD remains non-launchable |
| 4 | Genesis allocations split        | 60% treasury / 15% team / 10% val / 10% grants / 5% airdrop | `genesis_config_presets.rs` | Cannot freeze chainspec |
| 5 | Epoch length + consensus         | 6s blocks, 600-block epochs (~1h), 5+ validators by M2 | Runtime params | Finality behavior undefined |
| 6 | Reserve buffer %                 | 102% (100% backing + 2% buffer)                  | Legal disclosure     | Regulator comment exposure |
| 7 | Multisig sudo threshold          | 3-of-5                                           | Multisig sudo design | Sudo remains single-key    |
| 8 | ePL / gPLIM activation gates     | ePL: pallet-staking; gPLIM: pallet-democracy     | spec_version 103     | Assets remain frozen       |

**8/8 TBDs have concrete defaults proposed. 0 left open.**

---

## TBD #1 — PLIM max supply

**Context.** Genesis currently mints 1,000,000,000 PLIM (10^18 smallest units × 10^9). There is no inflation schedule, no cap declaration, and no validator reward funding source beyond transaction fees — which at launch will be near-zero.

### Proposal (default)

**2,000,000,000 PLIM hard cap**, reached via a 20-year inflation schedule:
- Genesis: 1,000,000,000 PLIM (already minted).
- Years 1–4: 5%/year inflation → ~200M new PLIM (validator rewards + treasury top-up).
- Years 5–9: linear decay 5% → 2%/year → ~375M cumulative.
- Years 10–20: 1%/year flat → ~425M cumulative.
- Total minted post-genesis: ~1,000,000,000 PLIM over 20 years, then hard-capped.

**Rationale.**
- Polkadot (DOT) uses ~10% target inflation and is secure; Ethereum post-merge ~0.5–1%; Bitcoin uses halvings + fees. Our ~2.5% average sits conservatively between these.
- Inflation is the only sustainable source of validator rewards for a PoA/PoS chain in its first 5 years, because fee volume will be low.
- A declared hard cap (2B) gives pEUR/pUSD reserve models a predictable denominator and helps the Liechtenstein licensing narrative (fixed monetary base).

### Alternative A — Hard cap at 1B (no inflation)
- Pros: maximally deflationary, simple narrative ("just like Bitcoin pre-halving-zero").
- Cons: validators are paid only from tx fees. At launch, fees approach zero. Realistic risk of chain halt or centralization to single operator. Not recommended for a live L1.

### Alternative B — Uncapped inflation (like early Ethereum)
- Pros: maximum flexibility for protocol-owned rewards.
- Cons: bad optics under MiCA; regulators prefer capped supply for anything adjacent to "currency". Harms pEUR licensing story.

### Trade-off matrix
| Option | Validator security | Regulator optics | Complexity |
| ------ | ------------------ | ---------------- | ---------- |
| Default 2B cap + decaying inflation | HIGH | GOOD | MEDIUM |
| 1B hard cap | LOW | BEST | LOW |
| Uncapped | HIGH | POOR | MEDIUM |

### Decision
- [ ] Accept default (2B cap, decaying inflation schedule)
- [ ] Accept Alternative A (1B hard cap)
- [ ] Accept Alternative B (uncapped)
- [ ] DEFER

### Action triggered if accepted (default)
1. Add `pallet-staking` with `RewardRemainder = Treasury` and era payout fn implementing the decay curve.
2. Add module `runtime/src/inflation.rs` with `era_payout(total_issuance, era_duration_millis) -> (validator_payout, remainder)`.
3. Set `MaxSupply: u128 = 2_000_000_000 * 10u128.pow(18)` as a `parameter_type` and enforce in staking era payout.
4. Update `TOKEN_CATALOG.md` Section "PLIM supply" with the schedule table.
5. Runtime upgrade ships in spec_version 103 (same upgrade as TBD #8).

---

## TBD #2 — pEUR issuer-of-record jurisdiction

**Context.** pEUR is intended as a 1:1 EUR-pegged stablecoin. Under MiCA (Regulation (EU) 2023/1114, in force since 30 June 2024 for stablecoins), an "e-money token" (EMT) referencing a single fiat currency must be issued by an authorized electronic money institution (EMI) or credit institution in the EU/EEA. Issuing pEUR without an authorization is a criminal offense in most member states.

### Proposal (default): Liechtenstein FMA under the TVTG (Token & TT Service Provider Act)

**Rationale.**
- Liechtenstein is an **EEA member** → EU passport for EMT issuance in all 30 EEA countries.
- TVTG (in force since Jan 2020) is the world's first purpose-built token-economy statute. FMA has a dedicated TT-desk; median TT Service Provider license timeline is **4–6 months**.
- Minimum paid-in capital for a TT Service Provider: **CHF 100,000** (TT Issuer tier). For an e-money license tier: **EUR 350,000** (EMI baseline under EU EMI Directive).
- Liechtenstein maintains Swiss-bank-grade custody infrastructure via LGT, VP Bank, and Bank Frick — all of which already custody crypto.
- Post-MiCA, FMA has published an EMT/ART transition path; existing TVTG holders get a streamlined MiCA conversion.

### Alternative A — Malta MFSA (VFA / MiCA CASP)
- Timeline: 10–12 months. Capital: EUR 350k (Class 3). Pros: English-speaking, common-law-ish. Cons: MFSA reputation hit post-2018, slower, and MiCA conversion is not streamlined.

### Alternative B — France AMF (PSAN → MiCA CASP)
- Timeline: 12–18 months. Pros: AMF is the most respected regulator on this list; AMF stamp opens EU-wide institutional doors. Cons: heaviest capital requirement (EUR 350k + proof of ongoing own funds), slowest, French-language filings required for many items.

### Alternative C — Germany BaFin
- Timeline: 18–24 months. Pros: gold-standard regulator, instant institutional credibility. Cons: capital EUR 350k–5M depending on scope, German-language filings, historically hostile to crypto-native applicants.

### Trade-off matrix
| Option | Time to license | Min capital | EU passport | Reputation |
| ------ | --------------- | ----------- | ----------- | ---------- |
| Liechtenstein FMA (default) | 4–6 mo | CHF 100k–EUR 350k | YES (EEA) | GOOD |
| Malta MFSA | 10–12 mo | EUR 350k | YES | FAIR |
| France AMF | 12–18 mo | EUR 350k+ | YES | EXCELLENT |
| Germany BaFin | 18–24 mo | EUR 350k–5M | YES | EXCELLENT |

### Decision
- [ ] Accept default (Liechtenstein FMA / TVTG)
- [ ] Accept Alternative A (Malta MFSA)
- [ ] Accept Alternative B (France AMF)
- [ ] Accept Alternative C (Germany BaFin)
- [ ] DEFER

### Action triggered if accepted (default)
1. Incorporate a Liechtenstein AG (Aktiengesellschaft) in Vaduz — `pLim Issuer AG`, paid-in CHF 100,000.
2. Engage Liechtenstein TT-Law counsel (recommended: Gasser Partner, Niedermüller, or Marxer & Partner) to file TVTG TT Service Provider registration.
3. Open fiat operating account + segregated pEUR reserve account at Bank Frick or VP Bank.
4. File FMA pre-consultation within 30 days.
5. Update `TOKEN_CATALOG.md` pEUR row: `Issuer: pLim Issuer AG (Vaduz, LI), license pending TVTG TT-Issuer + MiCA EMT conversion`.

---

## TBD #3 — pUSD legal framework

**Context.** A USD-pegged stablecoin that reaches any US person triggers either (a) NY DFS BitLicense if any NY user touches it, or (b) potential SEC/CFTC review, or (c) state-by-state money transmitter licenses (~49 of them, ~$2M bond average). Issuing a native USD-backed stablecoin is the single most regulated activity in crypto.

### Proposal (default): Partner-wrap with Circle USDC (1 pUSD = 1 USDC, always)

**Rationale.**
- Circle holds the NY DFS BitLicense, the Treasury-grade USDC reserve, monthly Deloitte attestations, and is MiCA-authorized in France (as of July 2024).
- We never **issue** USD-backed liabilities. We only wrap/unwrap USDC into pUSD on our chain via a 1:1 bridge.
- No BitLicense, no MTL, no SPDI. Custody role only. Custody can sit inside `pLim Issuer AG` (see TBD #2) or a separate Wyoming LLC.
- pUSD total supply on-chain is always ≤ USDC held in bridge contract. Auditable in real time.
- Delivery time to launch: **4–6 weeks** after Circle partnership executed, vs 12–24 months for native licensing.

### Alternative A — Wyoming SPDI (Special Purpose Depository Institution)
- Timeline: 12 months. Capital: USD 5M committed. Pros: full sovereignty over pUSD, no Circle dependency, SPDI has federal preemption over state MTL. Cons: $5M is real money, Wyoming SPDIs have had Fed master-account challenges (Custodia case still pending).

### Alternative B — NY DFS BitLicense (direct issuance)
- Timeline: 24 months. Capital: USD 1–10M depending on scope, ~USD 5k/yr maintenance. Pros: gold standard. Cons: nobody should do this unless they have a 10-person compliance team.

### Alternative C — Bermuda DABA (Digital Asset Business Act)
- Timeline: 6–9 months. Capital: low. Pros: offshore, fast, MFA-respected, good for non-US user base. Cons: zero US user access; DABA is an offshore license, US users must be geofenced out or it triggers FinCEN MSB anyway.

### Alternative D — El Salvador Digital Assets Law (pass-through, USD is legal tender)
- Timeline: 2–3 months. Cost: minimal. Pros: cheapest, fastest. Cons: reputation risk is non-trivial; El Salvador is seen as a last-resort jurisdiction by most institutional counterparties.

### Trade-off matrix
| Option | Time to launch | Capital | US access | Sovereignty | Reputation |
| ------ | -------------- | ------- | --------- | ----------- | ---------- |
| Circle partner-wrap (default) | 4–6 wk | ~USD 0 | YES (via Circle) | LOW | EXCELLENT |
| Wyoming SPDI | 12 mo | USD 5M | YES | HIGH | GOOD |
| NY BitLicense | 24 mo | USD 1–10M | YES | HIGH | BEST |
| Bermuda DABA | 6–9 mo | LOW | NO (geofence) | HIGH | FAIR |
| El Salvador | 2–3 mo | LOW | NO (geofence) | HIGH | POOR |

### Decision
- [ ] Accept default (Circle USDC partner-wrap)
- [ ] Accept Alternative A (Wyoming SPDI)
- [ ] Accept Alternative B (NY DFS BitLicense)
- [ ] Accept Alternative C (Bermuda DABA)
- [ ] Accept Alternative D (El Salvador)
- [ ] DEFER

### Action triggered if accepted (default)
1. Submit Circle Mint institutional account application (≥ USD 100k initial USDC liquidity commitment required to open).
2. Implement `pallet-usdc-bridge` (custom) that accepts USDC deposits on Ethereum mainnet (via Circle CCTP) and mints pUSD on pLim chain, and burns pUSD to release USDC.
3. Reserve contract: Ethereum multisig (Gnosis Safe) holding USDC, controlled by same 3-of-5 as TBD #7.
4. Update `TOKEN_CATALOG.md` pUSD row: `Issuer: None (wrapped), reserve: 1:1 USDC via Circle CCTP bridge, bridge custodian: pLim Issuer AG`.
5. Legal disclaimer in all pUSD UI: "pUSD is a wrapped representation of Circle USDC. Issuer liability rests with Circle Internet Financial, LLC."

---

## TBD #4 — Mainnet v2 genesis allocations split

**Context.** Current draft genesis has 600M treasury / 200M team_vesting / 1k sudo / 200M reserve — but the reserve was an ambiguous "post-launch via treasury proposals" placeholder. We need a **clean, totals-to-1B** allocation before we can freeze the chainspec.

### Proposal (default)

| Bucket               | Amount     | %     | Vesting / Schedule                                  |
| -------------------- | ---------- | ----- | --------------------------------------------------- |
| Treasury             | 600,000,000 | 60.0% | Unlocked, governed by future `pallet-treasury`      |
| Team vesting         | 150,000,000 | 15.0% | 4-year linear, 1-year cliff, per-key schedule       |
| Validator rewards    | 100,000,000 | 10.0% | 4-year schedule, paid per-era via staking pallet    |
| Ecosystem grants     | 100,000,000 | 10.0% | 2-year discretionary grant pool (treasury proposal) |
| Airdrop (early users)| 50,000,000  | 5.0%  | Claim window, 180 days, unclaimed reverts to treasury |
| Sudo key             | 1,000       | ~0%   | For emergency only; migrated to multisig (TBD #7)  |
| **Total**            | **1,000,000,000** | 100%  |                                                     |

Reserve bucket is dropped — its function is better served by the treasury + validator rewards buckets.

**Rationale.**
- 60% treasury matches Polkadot governance philosophy (community-owned chain).
- 15% team vested over 4yr is below the 20% median and signals long-term alignment.
- 10% validator rewards funds the first 4 years of PoS security (before inflation from TBD #1 ramps up).
- 10% ecosystem grants is the minimum viable grant pool to attract 10–15 protocols in year 1.
- 5% airdrop rewards early testnet users and creates a Sybil-resistant distribution moment.

### Alternative A — More decentralized (50/25/25)
- 50% treasury, 25% team, 25% airdrop, 0% validator rewards (pure inflation).
- Trade-off: higher team dilution, no ecosystem grants, validator rewards delayed until TBD #1 inflation kicks in — can work but less smooth.

### Alternative B — Keep current draft (60/20 reserve)
- 60% treasury, 20% team, 20% reserve, no explicit validator/airdrop/grants.
- Trade-off: opaque "reserve" bucket, future governance fights.

### Decision
- [ ] Accept default (60/15/10/10/5)
- [ ] Accept Alternative A (50/25/25)
- [ ] Accept Alternative B (current draft 60/20/20)
- [ ] DEFER

### Action triggered if accepted (default)
1. Edit `runtime/src/genesis_config_presets.rs::mainnet_genesis()` balances vec:
   - Treasury account: 600_000_000 * UNIT
   - Team vesting module account: 150_000_000 * UNIT (with Vec of per-account vesting schedules)
   - Validator rewards pot: 100_000_000 * UNIT → seeded into staking pallet reserve
   - Ecosystem grants: 100_000_000 * UNIT → separate pallet_balances account
   - Airdrop claims pallet: 50_000_000 * UNIT → seeded into `pallet-claims`
2. Add `pallet-vesting` to runtime (if not already present).
3. Add `pallet-claims` to runtime for airdrop (Merkle-root style claim).
4. Regenerate `chain-spec-mainnet-v2-raw.json`.
5. Update `MAINNET_V2_DEPLOY.md` genesis allocation table.

---

## TBD #5 — Epoch length + final consensus parameters

**Context.** Current runtime uses Aura (authority round-robin block production) with 6-second block time and GRANDPA finality. Currently only 2 authorities in the test genesis. Epoch length is effectively undefined (Aura does not use epochs natively; we'll need to define them via `pallet-session` or `pallet-babe`).

### Proposal (default)

| Parameter            | Value                       | Reason                                        |
| -------------------- | --------------------------- | --------------------------------------------- |
| Block time           | 6 seconds                   | Matches Polkadot, sweet spot for UX vs orphan rate |
| Consensus (block)    | Aura (keep)                 | Simpler than BABE, sufficient pre-staking     |
| Consensus (finality) | GRANDPA (keep)              | Battle-tested, 2f+1 Byzantine tolerance       |
| Epoch length         | 600 blocks (~1 hour)        | Balance between rotation freshness and overhead |
| Session length       | 600 blocks                  | 1:1 with epoch                                |
| Min validators       | 5 (by month 2 post-launch) | Below 5 = single-region failure risk          |
| Max validators       | 100 (runtime ceiling)       | Polkadot-style growth path                    |
| Slash reason set     | equivocation + offline      | Standard GRANDPA + Aura misbehavior set       |

**Rationale.**
- 6s blocks with ~1h epochs keeps GRANDPA gossip overhead manageable and gives new validators a predictable join schedule.
- 5 validators is the minimum for meaningful GRANDPA BFT (2f+1 ≥ 3 → f=1 tolerated). Below 5 and the chain is effectively PoA.
- Deferring the BABE upgrade (from Aura) to spec_version 104 or later — Aura is sufficient while validator count ≤ 20.

### Alternative A — 12-second blocks
- Pros: half the gossip overhead, lower energy, safer for low-bandwidth validators.
- Cons: 2× worse UX (wallets feel slow), lower TPS ceiling.

### Alternative B — Upgrade to BABE + NPoS now (Polkadot-grade)
- Pros: unlocks nominated proof-of-stake (NPoS), economic security via bonded tokens.
- Cons: ~3 months of engineering, pallet-babe + pallet-staking + pallet-session + pallet-offences integration, election provider setup.

### Decision
- [ ] Accept default (6s blocks, 600-block epochs, 5+ validators, Aura/GRANDPA)
- [ ] Accept Alternative A (12s blocks)
- [ ] Accept Alternative B (BABE + NPoS now)
- [ ] DEFER

### Action triggered if accepted (default)
1. Runtime `parameter_types!` update:
   ```rust
   pub const MILLISECS_PER_BLOCK: u64 = 6000;
   pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
   pub const EPOCH_DURATION_IN_BLOCKS: u32 = 600;
   pub const SESSION_PERIOD_IN_BLOCKS: u32 = 600;
   pub const MinValidators: u32 = 5;
   pub const MaxValidators: u32 = 100;
   ```
2. Add `pallet-session` to runtime (if not already present).
3. Document validator onboarding procedure in `MAINNET_V2_DEPLOY.md` § "Validator onboarding".
4. Month-2 target: onboard validators #2 through #5 via session key rotation.

---

## TBD #6 — Reserve buffer for pEUR / pUSD

**Context.** MiCA Article 36 requires that reserves for an e-money token be "at least equal" to outstanding tokens, marked-to-market daily, held with credit institutions, and segregated. It does NOT specify a buffer. Issuers typically maintain 100% (USDC) or >100% (historical Tether claims). The buffer question is operational risk, not legal minimum.

### Proposal (default): 102% reserve (100% peg + 2% buffer)

**Rationale.**
- 100% backing is the legal floor under MiCA Article 36.
- 2% buffer covers: (a) custody bank fees, (b) redemption spread during stress events, (c) reserve asset mark-to-market drift (if any reserve is held in short-term EU sovereign bills instead of cash).
- 2% is conservative relative to banking-sector reserve practice. Tether historically claimed ~102–105%.
- Not enforced on-chain. Enforced via **monthly attestation** by a Big-4-or-equivalent auditor (e.g. Grant Thornton Liechtenstein or BDO Malta), published to chain via the treasury multisig.

### Alternative A — 100% strict (USDC model)
- Pros: simpler narrative, matches Circle's approach, auditable with penny-level precision.
- Cons: no buffer → any operational outflow (fees, stress redemption) creates a short-lived under-collateralization.

### Alternative B — 105% buffer (Tether-like)
- Pros: maximum safety margin.
- Cons: ties up 5% capital forever, opportunity cost of ~5% × reserve size × interest rate.

### Decision
- [ ] Accept default (102%)
- [ ] Accept Alternative A (100% strict)
- [ ] Accept Alternative B (105%)
- [ ] DEFER

### Action triggered if accepted (default)
1. Update `TOKEN_CATALOG.md` pEUR/pUSD rows: "Reserve policy: 102% (100% peg + 2% operational buffer), monthly attestation".
2. Draft reserve policy document `docs/legal/reserve-policy.md` (new, but that's a doc change owned by TBD #2/#3 agents, not this one).
3. Include reserve policy in the FMA licensing application (per TBD #2).
4. No chain-level code change — enforcement is legal/custodial.

---

## TBD #7 — Multisig threshold for sudo

**Context.** Currently, mainnet v2 uses `pallet-sudo` with a single sudo key (1k PLIM address). This is acceptable only as a temporary bootstrap posture; single-key sudo on a chain holding ~1B PLIM (~indeterminate USD at launch, plus pEUR/pUSD reserves eventually) is a concentrated custody risk and will fail any security audit.

### Proposal (default): 3-of-5 multisig sudo

**Rationale.**
- Industry standard for protocol sudo/admin keys (used by Uniswap, Optimism, Arbitrum, Aave historically).
- Tolerates loss of **2 keys** (chain still functional) and compromise of **2 keys** (chain still secure). Single-failure and single-compromise safe.
- 5 distributed cosigners: CEO, CTO, CFO, Legal counsel, Independent trustee (e.g. Gasser Partner in Vaduz as independent key-holder).
- Lower thresholds (2-of-3) accept larger blast radius from a single compromise. Higher thresholds (4-of-7) add friction for routine upgrades.

**Recommended 5 cosigners:**
1. CEO (sr25519 key, hardware wallet)
2. CTO (sr25519 key, hardware wallet)
3. CFO (sr25519 key, hardware wallet)
4. Legal counsel (sr25519 key, hardware wallet — can be outside counsel)
5. Independent trustee (sr25519 key, held by Liechtenstein TT Service Provider partner)

### Alternative A — 2-of-3
- Pros: less friction, faster upgrade cadence.
- Cons: compromise of 2 keys = total loss; less comfortable for audit.

### Alternative B — 4-of-7
- Pros: survives 3 key losses, survives 3 compromises.
- Cons: 7 cosigners hard to schedule; operational friction.

### Trade-off matrix
| Option | Tolerates key loss | Tolerates compromise | Friction |
| ------ | ------------------ | --------------------- | -------- |
| 2-of-3 | 1 | 1 | LOW |
| **3-of-5 (default)** | **2** | **2** | **MEDIUM** |
| 4-of-7 | 3 | 3 | HIGH |

### Decision
- [ ] Accept default (3-of-5)
- [ ] Accept Alternative A (2-of-3)
- [ ] Accept Alternative B (4-of-7)
- [ ] DEFER

### Action triggered if accepted (default)
1. Generate 5 new sr25519 keys on hardware wallets; each key-holder runs `subkey inspect` and shares only the SS58 public address.
2. Construct 3-of-5 multisig address via `subkey` or `pallet-multisig` derivation.
3. Run sudo transfer transaction: `sudo.setKey(<multisig_address>)`.
4. Update `MAINNET_V2_DEPLOY.md` § "Sudo control" to list the multisig composition and each cosigner role.
5. Backup procedure document (recovery seed storage, geographic distribution) added to `MAINNET_KEYS_BACKUP.md`.
6. Note: the **multisig-sudo-migration** work is owned by a separate agent. This TBD only fixes the **threshold parameter** for that agent to implement.

---

## TBD #8 — ePL and gPLIM mainnet activation gates

**Context.** `TOKEN_CATALOG.md` lists ePL (economic PLIM, representing staked/bonded PLIM) and gPLIM (governance PLIM, representing locked-for-voting PLIM) as pre-registered assets in the genesis with sudo as admin — but there is currently no pallet that mints, burns, or manages them. They are effectively frozen rows in an asset registry. We need to define the conditions under which each becomes live.

### Proposal (default)

**ePL activation gate:** `pallet-staking` deployed and active with ≥5 validators bonded.
- 1 ePL minted per 1 PLIM bonded, held in user account as a derivative balance.
- Burned proportionally on slashing events (equivocation: 1%, offline: 0.01%).
- Transferable: NO at launch (to avoid restaking loops pre-audit). Activated post spec_version 104.

**gPLIM activation gate:** `pallet-conviction-voting` (preferred) OR `pallet-democracy` (fallback) deployed.
- 1 gPLIM minted per 1 PLIM locked for governance, with conviction multiplier (1x–6x based on lock duration per Polkadot OpenGov model).
- Burned when lock expires or vote is retracted.
- Non-transferable (governance tokens are soul-bound by construction).

**Both gates depend on runtime upgrade to spec_version 103 or later.**

**Rationale.**
- Gating prevents pre-launch "phantom liquidity" (listing ePL/gPLIM on DEXes before they have any economic backing).
- Couples token activation to governance infrastructure readiness, forcing the right build order: multisig → staking → governance → liquid restaking.
- Aligns with Polkadot's historical pattern (DOT staking activated only after NPoS shipped).

### Alternative A — Activate via sudo now (no pallet deps)
- Pros: simple, can demo immediately.
- Cons: no economic backing, easily spoofed, reputation damage if listed anywhere.

### Alternative B — Activate ePL with pallet-staking but defer gPLIM by 6 months
- Pros: less engineering load; ship staking first, governance later.
- Cons: governance-less chain is politically awkward (team retains full control via sudo multisig for 6+ extra months).

### Trade-off matrix
| Option | Engineering load | Governance cred | Timeline to activation |
| ------ | ---------------- | --------------- | ---------------------- |
| Default (staking + governance together) | HIGH | HIGH | ~3 months post multisig |
| Alt A (sudo-only) | NONE | NONE | Immediate |
| Alt B (staking now, governance later) | MEDIUM | MEDIUM | Staking: 6wk, gov: 6mo |

### Decision
- [ ] Accept default (gate ePL on pallet-staking, gate gPLIM on pallet-conviction-voting)
- [ ] Accept Alternative A (sudo-activation now)
- [ ] Accept Alternative B (staking now, governance deferred)
- [ ] DEFER

### Action triggered if accepted (default)
1. Add `pallet-staking` to runtime Cargo.toml and `construct_runtime!` macro.
2. Add `pallet-conviction-voting` + `pallet-referenda` to runtime.
3. Implement `pallet-epl` (custom) that mints/burns ePL as a side-effect of `pallet-staking` bond/unbond.
4. Implement `pallet-gplim` (custom) that mints/burns gPLIM as a side-effect of `pallet-conviction-voting` lock/unlock.
5. Ship as runtime upgrade spec_version 103 (or 104 if it must ship after the multisig sudo landing in 103).
6. Update `TOKEN_CATALOG.md` ePL + gPLIM rows: replace "sudo admin, no minting logic" with "live, gated by pallet-staking / pallet-conviction-voting".

---

## Cross-TBD dependency graph

```
TBD #7 (multisig sudo) ──┬──► TBD #4 (genesis balances) ──► chainspec freeze
                         │
                         └──► TBD #1 (inflation/supply cap) ──┐
                                                              │
TBD #5 (epoch + consensus) ──► TBD #8 (ePL/gPLIM gates) ──────┼──► spec_version 103
                                                              │
TBD #2 (pEUR jurisdiction) ──► TBD #6 (reserve buffer) ───────┤
                                                              │
TBD #3 (pUSD framework) ──────────────────────────────────────┘
```

**Critical path to mainnet v2 go-live:** TBD #7 → TBD #4 → chainspec freeze → genesis ceremony. Everything else can ship in subsequent spec_version upgrades.

---

## Review pass instructions (for the user)

1. Read each TBD section (they're short on purpose).
2. Tick exactly one box per TBD (default, alternative, or DEFER).
3. Save this file.
4. Hand back to the agent pool.

**You can accept all 8 defaults in a single review pass.** If you're happy with every "Accept default" line, just tick those 8 boxes and move on.

---

**End of document.**
