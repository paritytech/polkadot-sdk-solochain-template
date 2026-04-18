# pLim Token Catalog

**Last updated:** 2026-04-14
**Scope:** Authoritative reference for every token that will exist on the pLim Chain. Testnet (chain_id `plim-testnet`) and Mainnet v2 (chain_id `plim-mainnet-2`) share this schema; mainnet v1 is out of scope and being retired.

---

## Native token

### PLIM
- **Type:** Native (`pallet_balances`)
- **Decimals:** 12
- **ss58Format:** 42
- **tokenSymbol:** `PLIM`
- **Genesis:** distributed in `runtime/src/genesis_config_presets.rs` via `mainnet_genesis()` (mainnet v2) and `testnet_genesis()` (testnet)
- **Use:**
  - Gas / transaction fees (burned or redirected to treasury per `pallet_transaction_payment` configuration)
  - Validator staking bond via Aura
  - Governance weight (secondary to gPLIM once governance pallet lands)
  - Unit of account for on-chain PLIM-denominated rewards
- **Mint authority:** none post-genesis (fixed supply at genesis unless a future inflation pallet is added via governance)
- **Burn:** fee burn (if configured); no manual burn dispatch
- **Max supply:** TBD (recommendation: fix at genesis to ~1,000,000,000 PLIM, i.e. `1_000_000_000 * 10^12` planck-equivalent units)
- **Mainnet status:** will launch in mainnet v2 genesis

---

## Asset tokens (`pallet-assets`, available from `spec_version` 101)

All four asset tokens are registered at genesis via the `mainnet_genesis()` preset. Admin, Issuer, Freezer, and Owner slots are set to the sudo multisig at genesis and will be migrated to dedicated role accounts per token after launch.

### ePL (asset_id: `1`)
- **Symbol:** `ePL`
- **Name:** pLim Staked PLIM
- **Decimals:** 12
- **Use:** staking derivative — represents PLIM that has been locked in a staking contract and earns yield. 1 ePL is intended to track ≥1 PLIM as rewards accrue.
- **Mint authority:**
  - **Testnet:** sudo single account (fast iteration)
  - **Mainnet v2:** multisig initially, then the staking pallet itself once that pallet ships
- **Burn authority:** unstake flow only
- **Transferable:** yes
- **Freezable:** yes (compliance pallet may freeze on legal request)
- **Mainnet status:** TBD — ships with pallet-assets registration at genesis, but actual minting waits on staking pallet implementation

### gPLIM (asset_id: `2`)
- **Symbol:** `gPLIM`
- **Name:** pLim Governance Token
- **Decimals:** 12
- **Use:** governance voting weight for on-chain proposals. Distributed as referral rewards, long-term contributor grants, and staking yield.
- **Mint authority:**
  - **Testnet:** sudo
  - **Mainnet v2:** multisig initially, then the governance pallet once `pallet-democracy` / `pallet-collective` lands
- **Burn authority:** holder-initiated burn to remove voting weight
- **Transferable:** yes (may be restricted to whitelist under future governance decision)
- **Freezable:** yes
- **Mainnet status:** launches with genesis asset registration; rewards issuance begins after referral integration is green (see `BOOTSTRAP_ROADMAP.md` Step 5c)

### pEUR (asset_id: `3`) — MAIN FIAT STABLECOIN
- **Symbol:** `pEUR`
- **Name:** pLim Euro
- **Decimals:** 6
- **Peg:** 1 pEUR = 1 EUR
- **Use:** primary fiat-pegged stablecoin. This is the main on-chain unit of account for customer-facing billing flows, Stripe-to-chain mint, bridge settlement, and everyday pLim ecosystem payments.
- **Compliance:** **MiCA** (EU Regulation 2023/1114, Title III — Asset-Referenced and E-Money Tokens). pEUR is classified as an **Electronic Money Token (EMT)** because it references a single official currency (EUR).
- **Issuer-of-record:** TBD — requires authorization as either (a) a credit institution, or (b) an electronic money institution (EMI) under the EU E-Money Directive 2009/110/EC, passported to operate under MiCA. Recommended jurisdictions to evaluate: Liechtenstein (FMA, DLT Act friendly), Malta (MFSA), France (ACPR), Germany (BaFin). Final choice blocks on legal counsel review — DO NOT default without sign-off.
- **Reserve:**
  - 1:1 EUR backing required
  - Held in segregated accounts at one or more EU credit institutions
  - Monthly attestation by an independent auditor, publicly published
  - Reserve must at all times cover circulating supply plus a regulatory buffer (TBD per competent authority)
- **Mint authority:**
  - **Testnet:** sudo single account (flag `PLIM_MINT_MODE=sudo`)
  - **Mainnet v2:** 3-of-5 multisig (issuer officer, CFO, auditor, custodian, legal); `PLIM_MINT_MODE=multisig`
- **Burn authority:** holder-initiated burn through the approved redemption flow. Redemption returns 1 EUR per 1 pEUR, KYC gated, T+5 business days maximum.
- **Stripe integration:** `invoice.paid` webhook on Stripe account `acct_1TFrNZDHf0ymfkgV` triggers a gateway call that submits a mint to the customer's on-chain account. Idempotency is enforced via the existing `plim_subscriptions` event store.
- **Freeze / seize:** yes, via the compliance pallet; required for OFAC / EU sanctions compliance.
- **Transferable:** yes, subject to freeze list and transfer restrictions that MiCA may impose.
- **Mainnet status:** **TBD — do not deploy without legal authorization.** Genesis registration without minting is acceptable as long as `supply == 0` and mint dispatch is gated behind multisig that will refuse until Step 6 of the bootstrap roadmap is complete.

### pUSD (asset_id: `4`) — SECONDARY FIAT STABLECOIN
- **Symbol:** `pUSD`
- **Name:** pLim US Dollar
- **Decimals:** 6
- **Peg:** 1 pUSD = 1 USD
- **Use:** secondary fiat-pegged stablecoin for USD-denominated flows. Used for US customers, USD-priced bridge transactions, and cross-currency hedging when pEUR is not appropriate.
- **Compliance:** **TBD** — pUSD is outside MiCA's EMT scope because its reference currency is not an EU official currency. Possible frameworks:
  - NY DFS BitLicense + Trust Company charter (if US-facing issuance)
  - Partner with an existing licensed US stablecoin issuer and wrap their token 1:1
  - Issue from an offshore entity with clear disclaimers and geographic restrictions
  - Wait for federal US stablecoin legislation
  None of these is a default — legal decision required before mainnet deployment.
- **Issuer-of-record:** TBD
- **Reserve:** 1:1 USD backing required, monthly attestation, same structural requirements as pEUR.
- **Mint authority:**
  - **Testnet:** sudo
  - **Mainnet v2:** 3-of-5 multisig (same pattern as pEUR, possibly different human signers for separation of duties)
- **Burn authority:** holder-initiated redemption, KYC gated.
- **Transferable:** yes, subject to sanctions / compliance freezes
- **Mainnet status:** **TBD — testnet only until legal framework decided.** Genesis registration without minting is acceptable.

---

## Deprecated / removed tokens

### pBRL — REMOVED
- Previously planned as the secondary fiat stablecoin (Brazilian Real).
- Replaced by **pUSD** as of 2026-04-14 per product decision.
- Not reserved in asset_id space; `4` is now pUSD. Future pBRL can take a higher asset_id if ever revived.

---

## Decimals rationale

| Token  | Decimals | Reason                                                           |
|--------|----------|------------------------------------------------------------------|
| PLIM   | 12       | Matches Polkadot DOT convention — ecosystem interop, familiar to Substrate devs and tooling |
| ePL    | 12       | Matches PLIM for clean 1:1 mappings in the staking pallet        |
| gPLIM  | 12       | Matches PLIM so 1 PLIM staked can grant exactly 1 gPLIM vote weight if product chooses |
| pEUR   | 6        | Matches USDC / USDT / EURC convention — interoperability with bridged stablecoins and existing off-chain accounting tools |
| pUSD   | 6        | Same rationale as pEUR; uniformity across fiat stablecoins       |

**Note on mixed-decimal arithmetic:** any code that moves value between PLIM (12dp) and pEUR/pUSD (6dp) MUST explicitly scale. The `PlimPayments` pallet does not auto-convert; it only routes the exact amount requested in the caller's chosen asset.

---

## Asset id reservations

| ID  | Symbol | Status                                         |
|-----|--------|------------------------------------------------|
| 0   | —      | Reserved sentinel for native PLIM routing in `PlimPayments::pay` (not an actual pallet-assets id) |
| 1   | ePL    | Reserved                                       |
| 2   | gPLIM  | Reserved                                       |
| 3   | pEUR   | Reserved (MAIN FIAT, MiCA-gated on mainnet)    |
| 4   | pUSD   | Reserved (SECONDARY FIAT, legal-gated)         |
| 5   | future | Unallocated — candidates: pGBP, pCHF           |
| 6   | future | Unallocated                                    |
| 7   | future | Unallocated                                    |
| 8+  | future | Real-world asset tokens, ecosystem partner tokens, testnet-only experimental tokens |

**Rule:** once an asset_id is used on mainnet v2, it is permanent. Never reuse an id for a different token even if the original is retired — retire in place, never rebind.

---

## Naming convention

- **`p` prefix** — pLim wrapped / pegged fiat or external asset. Examples: `pEUR`, `pUSD`, and the deprecated `pBRL`. The letters after the prefix are the ISO 4217 code of the referenced currency.
- **No prefix** — native PLIM token.
- **`e` prefix** — staking derivative. `ePL` = "escrowed / enshrined PLIM" — PLIM locked in a staking contract that still represents claim on the underlying plus yield.
- **`g` prefix** — governance token. `gPLIM` = governance-weighted PLIM-adjacent token. Holders vote on on-chain proposals.
- **Future prefixes (reserved, do not use casually):**
  - `v` — vested (e.g., `vPLIM` for linear-unlock grants) — not currently planned
  - `r` — receipt (e.g., `rPLIM` for bridge receipts) — not currently planned

---

## On-chain metadata

All asset metadata (`name`, `symbol`, `decimals`) is set at genesis via `pallet_assets::GenesisConfig::metadata`. After genesis, metadata updates require the asset's `Owner` to sign, which on mainnet v2 is the sudo multisig until governance pallet takes over.

Expected `metadata` entries at mainnet v2 genesis:

```
(1, "pLim Staked PLIM",  "ePL",   12)
(2, "pLim Governance",   "gPLIM", 12)
(3, "pLim Euro",         "pEUR",  6)
(4, "pLim US Dollar",    "pUSD",  6)
```

---

## Change control

Any change to this catalog (new asset, decimals change, compliance status change) requires:
1. PR against this file
2. Sign-off from protocol-team lead
3. For mainnet v2 changes that affect on-chain state: a runtime upgrade PR linking to this file
4. For pEUR or pUSD compliance status changes: additional legal counsel sign-off recorded in the PR

---

**End of catalog.**
