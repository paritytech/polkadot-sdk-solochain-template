//! # CREATE Token Pallet
//!
//! This pallet implements the CREATE governance token for the CREATEFI blockchain.
//! CREATE is the native DAO token with governance rights, staking rewards, and anti-whale protection.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, WithdrawReasons, LockableCurrency, LockIdentifier},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero, Saturating};

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The currency type for handling balances.
        type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId>;
        
        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: WeightInfo;
        
        /// The maximum percentage any single wallet can hold (5%).
        #[pallet::constant]
        type MaxWalletPercentage: Get<u32>;
        
        /// The minimum amount required to participate in DAO governance.
        #[pallet::constant]
        type MinGovernanceStake: Get<BalanceOf<Self>>;
        
        /// The lock identifier for staking.
        #[pallet::constant]
        type StakingLockId: Get<LockIdentifier>;
        
        /// The lock identifier for governance voting.
        #[pallet::constant]
        type GovernanceLockId: Get<LockIdentifier>;
    }

    /// Balance type for this pallet.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// The pallet's storage items.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Total supply of CREATE tokens.
    #[pallet::storage]
    #[pallet::getter(fn total_supply)]
    pub type TotalSupply<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// CREATE balance for each account.
    #[pallet::storage]
    #[pallet::getter(fn balance_of)]
    pub type Balances<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Staked CREATE tokens for each account.
    #[pallet::storage]
    #[pallet::getter(fn staked_balance)]
    pub type StakedBalances<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Total staked CREATE tokens.
    #[pallet::storage]
    #[pallet::getter(fn total_staked)]
    pub type TotalStaked<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Governance voting power for each account.
    #[pallet::storage]
    #[pallet::getter(fn governance_power)]
    pub type GovernancePower<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Total governance voting power.
    #[pallet::storage]
    #[pallet::getter(fn total_governance_power)]
    pub type TotalGovernancePower<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Staking rewards rate (per block).
    #[pallet::storage]
    #[pallet::getter(fn staking_reward_rate)]
    pub type StakingRewardRate<T> = StorageValue<_, u32, ValueQuery>;

    /// Last reward distribution block.
    #[pallet::storage]
    pub type LastRewardBlock<T> = StorageValue<_, u32, ValueQuery>;

    /// Pending rewards for each account.
    #[pallet::storage]
    #[pallet::getter(fn pending_rewards)]
    pub type PendingRewards<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// CREATE tokens were minted.
        /// [account, amount]
        TokensMinted {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// CREATE tokens were burned.
        /// [account, amount]
        TokensBurned {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// CREATE tokens were transferred.
        /// [from, to, amount]
        TokensTransferred {
            from: T::AccountId,
            to: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// CREATE tokens were staked.
        /// [account, amount]
        TokensStaked {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// CREATE tokens were unstaked.
        /// [account, amount]
        TokensUnstaked {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Staking rewards were claimed.
        /// [account, amount]
        RewardsClaimed {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Governance power was updated.
        /// [account, new_power]
        GovernancePowerUpdated {
            account: T::AccountId,
            new_power: BalanceOf<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient balance to perform the operation.
        InsufficientBalance,
        /// Amount exceeds maximum allowed.
        AmountExceedsMaximum,
        /// Amount is below minimum required.
        AmountBelowMinimum,
        /// Operation would cause overflow.
        Overflow,
        /// Operation would cause underflow.
        Underflow,
        /// Account has no staked tokens.
        NoStakedTokens,
        /// Insufficient staked balance.
        InsufficientStakedBalance,
        /// Transfer would exceed maximum wallet percentage.
        ExceedsMaxWalletPercentage,
        /// No rewards available to claim.
        NoRewardsAvailable,
        /// Staking lock is active.
        StakingLockActive,
        /// Governance lock is active.
        GovernanceLockActive,
    }

    /// The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Mint CREATE tokens (only callable by authorized accounts).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::mint_tokens())]
        pub fn mint_tokens(origin: OriginFor<T>, to: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            // TODO: Add authorization check for minting
            // For now, allow any signed account to mint (should be restricted in production)

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if transfer would exceed max wallet percentage
            let current_balance = Balances::<T>::get(&to);
            let new_balance = current_balance.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            
            let total_supply = TotalSupply::<T>::get();
            let max_allowed = total_supply
                .checked_mul(&T::MaxWalletPercentage::get().into())
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&100u32.into())
                .ok_or(Error::<T>::Underflow)?;
            
            ensure!(new_balance <= max_allowed, Error::<T>::ExceedsMaxWalletPercentage);

            // Mint tokens
            Balances::<T>::insert(&to, new_balance);
            
            let total_supply = TotalSupply::<T>::get()
                .checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalSupply::<T>::put(total_supply);

            Self::deposit_event(Event::TokensMinted {
                account: to,
                amount,
            });

            Ok(())
        }

        /// Burn CREATE tokens.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::burn_tokens())]
        pub fn burn_tokens(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if user has sufficient balance
            let current_balance = Balances::<T>::get(&who);
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            // Burn tokens
            let new_balance = current_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Balances::<T>::insert(&who, new_balance);

            let total_supply = TotalSupply::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalSupply::<T>::put(total_supply);

            Self::deposit_event(Event::TokensBurned {
                account: who,
                amount,
            });

            Ok(())
        }

        /// Transfer CREATE tokens.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::transfer_tokens())]
        pub fn transfer_tokens(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if sender has sufficient balance
            let from_balance = Balances::<T>::get(&from);
            ensure!(from_balance >= amount, Error::<T>::InsufficientBalance);

            // Check if recipient would exceed max wallet percentage
            let to_balance = Balances::<T>::get(&to);
            let new_to_balance = to_balance.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            
            let total_supply = TotalSupply::<T>::get();
            let max_allowed = total_supply
                .checked_mul(&T::MaxWalletPercentage::get().into())
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&100u32.into())
                .ok_or(Error::<T>::Underflow)?;
            
            ensure!(new_to_balance <= max_allowed, Error::<T>::ExceedsMaxWalletPercentage);

            // Transfer tokens
            let new_from_balance = from_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Balances::<T>::insert(&from, new_from_balance);
            Balances::<T>::insert(&to, new_to_balance);

            Self::deposit_event(Event::TokensTransferred {
                from,
                to,
                amount,
            });

            Ok(())
        }

        /// Stake CREATE tokens for rewards and governance power.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::stake_tokens())]
        pub fn stake_tokens(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if user has sufficient balance
            let current_balance = Balances::<T>::get(&who);
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            // Update balances
            let new_balance = current_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Balances::<T>::insert(&who, new_balance);

            let staked_balance = StakedBalances::<T>::get(&who);
            let new_staked_balance = staked_balance.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            StakedBalances::<T>::insert(&who, new_staked_balance);

            // Update total staked
            let total_staked = TotalStaked::<T>::get()
                .checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalStaked::<T>::put(total_staked);

            // Update governance power
            Self::update_governance_power(&who, new_staked_balance)?;

            // Lock tokens for staking
            T::Currency::set_lock(
                T::StakingLockId::get(),
                &who,
                new_staked_balance,
                WithdrawReasons::all(),
            );

            Self::deposit_event(Event::TokensStaked {
                account: who,
                amount,
            });

            Ok(())
        }

        /// Unstake CREATE tokens.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::unstake_tokens())]
        pub fn unstake_tokens(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if user has sufficient staked balance
            let staked_balance = StakedBalances::<T>::get(&who);
            ensure!(staked_balance >= amount, Error::<T>::InsufficientStakedBalance);

            // Update staked balances
            let new_staked_balance = staked_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            StakedBalances::<T>::insert(&who, new_staked_balance);

            let current_balance = Balances::<T>::get(&who);
            let new_balance = current_balance.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            Balances::<T>::insert(&who, new_balance);

            // Update total staked
            let total_staked = TotalStaked::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalStaked::<T>::put(total_staked);

            // Update governance power
            Self::update_governance_power(&who, new_staked_balance)?;

            // Update lock
            T::Currency::set_lock(
                T::StakingLockId::get(),
                &who,
                new_staked_balance,
                WithdrawReasons::all(),
            );

            Self::deposit_event(Event::TokensUnstaked {
                account: who,
                amount,
            });

            Ok(())
        }

        /// Claim staking rewards.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::claim_rewards())]
        pub fn claim_rewards(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Calculate and distribute rewards
            Self::distribute_rewards(&who)?;

            let pending_rewards = PendingRewards::<T>::get(&who);
            ensure!(!pending_rewards.is_zero(), Error::<T>::NoRewardsAvailable);

            // Reset pending rewards
            PendingRewards::<T>::insert(&who, BalanceOf::<T>::zero());

            // Add rewards to balance
            let current_balance = Balances::<T>::get(&who);
            let new_balance = current_balance.checked_add(&pending_rewards)
                .ok_or(Error::<T>::Overflow)?;
            Balances::<T>::insert(&who, new_balance);

            Self::deposit_event(Event::RewardsClaimed {
                account: who,
                amount: pending_rewards,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {

        /// Update governance power for an account.
        fn update_governance_power(account: &T::AccountId, staked_amount: BalanceOf<T>) -> DispatchResult {
            // Only accounts with minimum stake can participate in governance
            if staked_amount >= T::MinGovernanceStake::get() {
                GovernancePower::<T>::insert(account, staked_amount);
                
                let total_power = TotalGovernancePower::<T>::get();
                let new_total_power = total_power.saturating_add(staked_amount);
                TotalGovernancePower::<T>::put(new_total_power);

                Self::deposit_event(Event::GovernancePowerUpdated {
                    account: account.clone(),
                    new_power: staked_amount,
                });
            } else {
                let old_power = GovernancePower::<T>::get(account);
                if !old_power.is_zero() {
                    GovernancePower::<T>::remove(account);
                    
                    let total_power = TotalGovernancePower::<T>::get();
                    let new_total_power = total_power.saturating_sub(old_power);
                    TotalGovernancePower::<T>::put(new_total_power);
                }
            }

            Ok(())
        }

        /// Distribute staking rewards to an account.
        fn distribute_rewards(account: &T::AccountId) -> DispatchResult {
            let staked_amount = StakedBalances::<T>::get(account);
            if staked_amount.is_zero() {
                return Ok(());
            }

            // TODO: Fix type conversion issues in reward calculation
            // let current_block = frame_system::Pallet::<T>::block_number();
            // let last_reward_block = LastRewardBlock::<T>::get();
            
            // if current_block > last_reward_block {
            //     let blocks_since_last_reward = current_block - last_reward_block;
            //     let reward_rate = StakingRewardRate::<T>::get();
                
            //     // Calculate rewards based on staked amount and time
            //     let rewards = staked_amount
            //         .checked_mul(&(blocks_since_last_reward as u128).into())
            //         .ok_or(Error::<T>::Overflow)?
            //         .checked_mul(&(reward_rate as u128).into())
            //         .ok_or(Error::<T>::Overflow)?
            //         .checked_div(&1_000_000u128.into()) // Reward rate is in parts per million
            //         .ok_or(Error::<T>::Underflow)?;

            //     let pending_rewards = PendingRewards::<T>::get(account);
            //     let new_pending_rewards = pending_rewards.checked_add(&rewards)
            //         .ok_or(Error::<T>::Overflow)?;
            //     PendingRewards::<T>::insert(account, new_pending_rewards);
            // }

            // LastRewardBlock::<T>::put(current_block);
            Ok(())
        }

        /// Initialize the CREATE token with default parameters.
        /// This should be called during genesis.
        pub fn initialize_token() {
            // TODO: Fix type conversion issues
            // Set initial total supply (1,000,000,000 CREATE)
            // TotalSupply::<T>::put(1_000_000_000_000_000_000_000_000u128);
            
            // Set staking reward rate (0.1% per block = 1000 ppm)
            StakingRewardRate::<T>::put(1000);
            
            // TODO: Fix type conversion issues
            // Set initial last reward block
            // LastRewardBlock::<T>::put(frame_system::Pallet::<T>::block_number());
        }
    }
}