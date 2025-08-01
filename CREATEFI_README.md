# CREATEFI Blockchain

A next-generation Layer 1 blockchain platform focused on resolving the inefficiencies of unpredictable gas fees in DeFi systems. Built entirely in Rust with a WASM-based smart contract engine and a collateral-backed stablecoin (FI), CREATEFI introduces a fixed-fee economy for all operations.

## 🚀 Features

### Core Components

- **FI Stablecoin**: 1:1 backed by top stablecoins (USDT, USDC, DAI, BUSD, TUSD)
- **Fixed Fee System**: Predictable costs for all operations
- **CREATE Governance Token**: Native DAO token with staking and voting
- **DEX Module**: AMM and order book hybrid trading
- **Anti-Whale Protection**: Max 5% cap on DAO holdings
- **WASM Smart Contracts**: Secure and efficient execution

### Fixed Fee Structure

| Service Category | Fixed Fee (FI) | USD Value | Founder (15%) | DAO (85%) |
|------------------|----------------|-----------|---------------|-----------|
| Gas Fee | 0.01 FI | $0.01 | $0.0015 | $0.0085 |
| Bridge < $100 | 0.05 FI | $0.05 | $0.0075 | $0.0425 |
| Bridge $100–$1,000 | 0.1 FI | $0.10 | $0.015 | $0.085 |
| Bridge > $1,000 | 0.5 FI | $0.50 | $0.075 | $0.425 |
| DEX Trading | 0.01 FI | $0.01 | $0.0015 | $0.0085 |
| Pool < $10k TVL | 1.0 FI | $1.00 | $0.15 | $0.85 |
| Pool $10k–$100k TVL | 2.0 FI | $2.00 | $0.30 | $1.70 |
| Pool > $100k TVL | 5.0 FI | $5.00 | $0.75 | $4.25 |
| Token Creation | 0.5 FI | $0.50 | $0.075 | $0.425 |
| NFT Minting (<50 NFTs) | 0.05 FI | $0.05 | $0.0075 | $0.0425 |
| Governance Proposal | 0.5 FI | $0.50 | $0.075 | $0.425 |

## 🏗️ Architecture

### Pallets (Modules)

1. **FI Stablecoin Pallet** (`pallet-fi-stablecoin`)
   - Mint/burn FI tokens with 1:1 collateral backing
   - Collateral management and vault operations
   - Non-transferable between users (fee-only usage)

2. **Fee Engine Pallet** (`pallet-fee-engine`)
   - Fixed fee enforcement for all transaction types
   - Fee distribution (15% founder, 85% DAO)
   - Configurable fee structure

3. **CREATE Token Pallet** (`pallet-create-token`)
   - Governance token with staking rewards
   - Anti-whale protection (max 5% per wallet)
   - Voting power based on staked amount

4. **DEX Pallet** (`pallet-dex`)
   - AMM pools with constant product formula
   - Order book trading
   - Liquidity provision and LP tokens

## 🛠️ Installation & Setup

### Prerequisites

- Rust 1.70+
- Substrate dependencies
- 8GB+ RAM for compilation

### Build Instructions

```bash
# Clone the repository
git clone <repository-url>
cd createfi-blockchain

# Build the project
cargo build --release

# Run development node
./target/release/solochain-template-node --dev
```

### Development Chain

```bash
# Start development chain
./target/release/solochain-template-node --dev

# Purge chain state
./target/release/solochain-template-node purge-chain --dev

# Start with detailed logging
RUST_BACKTRACE=1 ./target/release/solochain-template-node -ldebug --dev
```

## 📊 Tokenomics

### CREATE Token
- **Total Supply**: 1,000,000,000 CREATE
- **Initial Price**: $10.00 USD
- **Distribution**:
  - 15% reserved (10% founder, 5% airdrops)
  - 85% available for public DAO participants
- **Anti-Whale**: Max 5% cap on DAO holdings

### FI Token
- **Nature**: Fixed-supply, protocol-controlled
- **Usage**: Fee payments only
- **Backing**: 1:1 with top 5 stablecoins
- **Non-transferable**: Between users

## 🔧 Usage Examples

### FI Stablecoin Operations

```rust
// Deposit collateral
pallet_fi_stablecoin::Pallet::<Runtime>::deposit_collateral(
    origin,
    1_000_000_000_000, // 1 FI worth of collateral
);

// Mint FI tokens
pallet_fi_stablecoin::Pallet::<Runtime>::mint_fi(
    origin,
    1_000_000_000_000, // 1 FI
);

// Burn FI tokens
pallet_fi_stablecoin::Pallet::<Runtime>::burn_fi(
    origin,
    1_000_000_000_000, // 1 FI
);
```

### CREATE Token Operations

```rust
// Mint CREATE tokens
pallet_create_token::Pallet::<Runtime>::mint_tokens(
    origin,
    recipient,
    1_000_000_000_000_000_000_000_000, // 1 CREATE
);

// Stake CREATE tokens
pallet_create_token::Pallet::<Runtime>::stake_tokens(
    origin,
    1_000_000_000_000_000_000_000_000, // 1 CREATE
);

// Claim staking rewards
pallet_create_token::Pallet::<Runtime>::claim_rewards(origin);
```

### DEX Operations

```rust
// Create AMM pool
pallet_dex::Pallet::<Runtime>::create_pool(
    origin,
    b"TOKEN_A".to_vec(),
    b"TOKEN_B".to_vec(),
    1_000_000_000_000_000_000_000_000, // 1 token A
    1_000_000_000_000_000_000_000_000, // 1 token B
);

// Add liquidity
pallet_dex::Pallet::<Runtime>::add_liquidity(
    origin,
    pool_id,
    1_000_000_000_000_000_000_000_000, // 1 token A
    1_000_000_000_000_000_000_000_000, // 1 token B
);

// Execute AMM trade
pallet_dex::Pallet::<Runtime>::amm_trade(
    origin,
    pool_id,
    b"TOKEN_A".to_vec(),
    1_000_000_000_000_000_000_000_000, // 1 token A
    900_000_000_000_000_000_000_000,   // min output
);
```

## 🔒 Security Features

- **Fixed Fee Enforcement**: No dynamic pricing or gas auctions
- **Anti-Whale Protection**: Prevents concentration of governance power
- **Collateral Backing**: FI tokens always backed 1:1
- **WASM Execution**: Secure smart contract environment
- **Permissionless**: Open access to all features

## 📈 Performance Targets

- **Transaction Throughput**: 100,000+ TPS
- **Block Time**: < 5 seconds
- **API Response Time**: < 100ms
- **Cross-chain Bridge**: < 30 seconds

## 🎯 Roadmap

### Phase 1 ✅ (Completed)
- Rust blockchain core
- FI stablecoin implementation
- DEX with AMM pools
- Frontend and API

### Phase 2-7 🎯 (In Progress)
- Consensus upgrade
- Staking mechanisms
- Full DEX functionality

### Phase 8-10 📋 (Planned)
- Bridge security
- NFT minting and analytics

### Phase 11-13 📋 (Planned)
- Launchpad functionality
- Lending and yield systems

### Phase 14 🚀 (Planned)
- Mainnet launch
- DAO governance
- Treasury management
- Security audits

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

This project is licensed under the MIT-0 License.

## 🆘 Support

For technical support or questions:
- Create an issue on GitHub
- Join our Discord community
- Contact: sammuti.com

## 🔗 Links

- [Documentation](https://docs.createfi.com)
- [Whitepaper](https://createfi.com/whitepaper)
- [Discord](https://discord.gg/createfi)
- [Twitter](https://twitter.com/createfi)

---

**CREATEFI** - Resolving DeFi inefficiencies with predictable, fixed-fee economics.