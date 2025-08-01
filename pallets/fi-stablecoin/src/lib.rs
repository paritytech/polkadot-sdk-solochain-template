//! # FI Stablecoin Pallet
//!
//! This pallet implements the FI stablecoin for the CREATEFI blockchain.
//! FI is a stablecoin backed 1:1 by top stablecoins (USDT, USDC, DAI, BUSD, TUSD).
//! It is non-transferable between users and only used for gas and fee payments.

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
        traits::{Currency, ExistenceRequirement, WithdrawReasons},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero};

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The currency type for handling balances.
        type Currency: Currency<Self::AccountId>;
        
        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: WeightInfo;
        
        /// The minimum amount of collateral required to mint FI.
        #[pallet::constant]
        type MinCollateralAmount: Get<BalanceOf<Self>>;
        
        /// The maximum amount of FI that can be minted per transaction.
        #[pallet::constant]
        type MaxMintAmount: Get<BalanceOf<Self>>;
    }

    /// Balance type for this pallet.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// The pallet's storage items.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Total supply of FI tokens.
    #[pallet::storage]
    #[pallet::getter(fn total_supply)]
    pub type TotalSupply<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// FI balance for each account.
    #[pallet::storage]
    #[pallet::getter(fn balance_of)]
    pub type Balances<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Collateral reserves for each account.
    #[pallet::storage]
    #[pallet::getter(fn collateral_of)]
    pub type Collateral<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Total collateral in the system.
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub type TotalCollateral<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// FI tokens were minted.
        /// [account, amount, collateral_amount]
        Minted {
            account: T::AccountId,
            amount: BalanceOf<T>,
            collateral_amount: BalanceOf<T>,
        },
        /// FI tokens were burned.
        /// [account, amount, collateral_returned]
        Burned {
            account: T::AccountId,
            amount: BalanceOf<T>,
            collateral_returned: BalanceOf<T>,
        },
        /// Collateral was deposited.
        /// [account, amount]
        CollateralDeposited {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Collateral was withdrawn.
        /// [account, amount]
        CollateralWithdrawn {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient balance to perform the operation.
        InsufficientBalance,
        /// Insufficient collateral to mint FI.
        InsufficientCollateral,
        /// Amount exceeds maximum allowed.
        AmountExceedsMaximum,
        /// Amount is below minimum required.
        AmountBelowMinimum,
        /// Operation would cause overflow.
        Overflow,
        /// Operation would cause underflow.
        Underflow,
        /// Account has no FI balance.
        NoFiBalance,
        /// Account has no collateral.
        NoCollateral,
    }

    /// The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Mint FI tokens by depositing collateral.
        /// 
        /// The user must have sufficient collateral balance to mint FI.
        /// The amount of FI minted will be equal to the collateral amount (1:1 ratio).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::mint_fi())]
        pub fn mint_fi(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);
            
            // Ensure amount doesn't exceed maximum
            ensure!(amount <= T::MaxMintAmount::get(), Error::<T>::AmountExceedsMaximum);

            // Check if user has sufficient collateral
            let user_collateral = Collateral::<T>::get(&who);
            ensure!(user_collateral >= amount, Error::<T>::InsufficientCollateral);

            // Update collateral
            let new_collateral = user_collateral.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Collateral::<T>::insert(&who, new_collateral);

            // Update total collateral
            let total_collateral = TotalCollateral::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalCollateral::<T>::put(total_collateral);

            // Mint FI tokens
            let current_balance = Balances::<T>::get(&who);
            let new_balance = current_balance.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            Balances::<T>::insert(&who, new_balance);

            // Update total supply
            let total_supply = TotalSupply::<T>::get()
                .checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalSupply::<T>::put(total_supply);

            Self::deposit_event(Event::Minted {
                account: who,
                amount,
                collateral_amount: amount,
            });

            Ok(())
        }

        /// Burn FI tokens and return collateral.
        /// 
        /// The user must have sufficient FI balance to burn.
        /// The amount of collateral returned will be equal to the FI burned (1:1 ratio).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::burn_fi())]
        pub fn burn_fi(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if user has sufficient FI balance
            let current_balance = Balances::<T>::get(&who);
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            // Burn FI tokens
            let new_balance = current_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Balances::<T>::insert(&who, new_balance);

            // Update total supply
            let total_supply = TotalSupply::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalSupply::<T>::put(total_supply);

            // Return collateral to user's account
            let user_collateral = Collateral::<T>::get(&who);
            let new_collateral = user_collateral.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            Collateral::<T>::insert(&who, new_collateral);

            // Update total collateral
            let total_collateral = TotalCollateral::<T>::get()
                .checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalCollateral::<T>::put(total_collateral);

            Self::deposit_event(Event::Burned {
                account: who,
                amount,
                collateral_returned: amount,
            });

            Ok(())
        }

        /// Deposit collateral into the vault.
        /// 
        /// This function allows users to deposit stablecoins as collateral.
        /// The collateral can later be used to mint FI tokens.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::deposit_collateral())]
        pub fn deposit_collateral(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);
            
            // Ensure amount meets minimum requirement
            ensure!(amount >= T::MinCollateralAmount::get(), Error::<T>::AmountBelowMinimum);

            // Transfer collateral from user to vault
            T::Currency::withdraw(
                &who,
                amount,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::KeepAlive,
            )?;

            // Update user's collateral balance
            let current_collateral = Collateral::<T>::get(&who);
            let new_collateral = current_collateral.checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            Collateral::<T>::insert(&who, new_collateral);

            // Update total collateral
            let total_collateral = TotalCollateral::<T>::get()
                .checked_add(&amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalCollateral::<T>::put(total_collateral);

            Self::deposit_event(Event::CollateralDeposited {
                account: who,
                amount,
            });

            Ok(())
        }

        /// Withdraw collateral from the vault.
        /// 
        /// Users can withdraw their collateral if they have sufficient balance.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::withdraw_collateral())]
        pub fn withdraw_collateral(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);

            // Check if user has sufficient collateral
            let current_collateral = Collateral::<T>::get(&who);
            ensure!(current_collateral >= amount, Error::<T>::InsufficientCollateral);

            // Update user's collateral balance
            let new_collateral = current_collateral.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Collateral::<T>::insert(&who, new_collateral);

            // Update total collateral
            let total_collateral = TotalCollateral::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalCollateral::<T>::put(total_collateral);

            // Transfer collateral back to user
            T::Currency::deposit_creating(&who, amount);

            Self::deposit_event(Event::CollateralWithdrawn {
                account: who,
                amount,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if an account has sufficient FI balance.
        pub fn has_sufficient_balance(account: &<T as frame_system::Config>::AccountId, amount: BalanceOf<T>) -> bool {
            Balances::<T>::get(account) >= amount
        }

        /// Burn FI tokens from an account (internal function for fee payments).
        pub fn burn_fi_internal(account: &<T as frame_system::Config>::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let current_balance = Balances::<T>::get(account);
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            // Burn FI tokens
            let new_balance = current_balance.checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            Balances::<T>::insert(account, new_balance);

            // Update total supply
            let total_supply = TotalSupply::<T>::get()
                .checked_sub(&amount)
                .ok_or(Error::<T>::Underflow)?;
            TotalSupply::<T>::put(total_supply);

            Ok(())
        }
    }
}