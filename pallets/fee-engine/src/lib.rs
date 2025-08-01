//! # Fee Engine Pallet
//!
//! This pallet implements the fixed fee system for the CREATEFI blockchain.
//! It enforces predictable fees for all operations and handles fee distribution
//! between the founder (15%) and DAO treasury (85%).

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
        traits::Currency,
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
        
        /// The founder account that receives 15% of fees.
        #[pallet::constant]
        type FounderAccount: Get<Self::AccountId>;
        
        /// The DAO treasury account that receives 85% of fees.
        #[pallet::constant]
        type DaoTreasuryAccount: Get<Self::AccountId>;
    }

    /// Balance type for this pallet.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Transaction types for fee calculation.
    pub type TransactionType = u8;
    pub const TX_TYPE_GAS_FEE: TransactionType = 0;
    pub const TX_TYPE_BRIDGE_SMALL: TransactionType = 1;
    pub const TX_TYPE_BRIDGE_MEDIUM: TransactionType = 2;
    pub const TX_TYPE_BRIDGE_LARGE: TransactionType = 3;
    pub const TX_TYPE_NFT_BRIDGE: TransactionType = 4;
    pub const TX_TYPE_DEX_TRADING: TransactionType = 5;
    pub const TX_TYPE_POOL_SMALL: TransactionType = 6;
    pub const TX_TYPE_POOL_MEDIUM: TransactionType = 7;
    pub const TX_TYPE_POOL_LARGE: TransactionType = 8;
    pub const TX_TYPE_POOL_OPERATIONS: TransactionType = 9;
    pub const TX_TYPE_TOKEN_CREATION: TransactionType = 10;
    pub const TX_TYPE_NFT_MINTING_SMALL: TransactionType = 11;
    pub const TX_TYPE_NFT_MINTING_MEDIUM: TransactionType = 12;
    pub const TX_TYPE_NFT_MINTING_LARGE: TransactionType = 13;
    pub const TX_TYPE_NFT_MINTING_XLARGE: TransactionType = 14;
    pub const TX_TYPE_VAULT_CREATION: TransactionType = 15;
    pub const TX_TYPE_GOVERNANCE_PROPOSAL: TransactionType = 16;

    /// The pallet's storage items.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Fixed fees for each transaction type (in FI tokens).
    #[pallet::storage]
    #[pallet::getter(fn get_fee)]
    pub type FixedFees<T> = StorageMap<_, Blake2_128Concat, TransactionType, BalanceOf<T>, ValueQuery>;

    /// Total fees collected by the protocol.
    #[pallet::storage]
    #[pallet::getter(fn total_fees_collected)]
    pub type TotalFeesCollected<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Founder's fee share (15%).
    #[pallet::storage]
    #[pallet::getter(fn founder_fees)]
    pub type FounderFees<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// DAO treasury fee share (85%).
    #[pallet::storage]
    #[pallet::getter(fn dao_fees)]
    pub type DaoFees<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Fee was collected and distributed.
        /// [payer, transaction_type, amount, founder_share, dao_share]
        FeeCollected {
            payer: T::AccountId,
            transaction_type: TransactionType,
            amount: BalanceOf<T>,
            founder_share: BalanceOf<T>,
            dao_share: BalanceOf<T>,
        },
        /// Fee structure was updated.
        /// [transaction_type, new_fee]
        FeeUpdated {
            transaction_type: TransactionType,
            new_fee: BalanceOf<T>,
        },
        /// Founder fees were withdrawn.
        /// [amount]
        FounderFeesWithdrawn {
            amount: BalanceOf<T>,
        },
        /// DAO fees were withdrawn.
        /// [amount]
        DaoFeesWithdrawn {
            amount: BalanceOf<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient fee paid for the transaction.
        InsufficientFee,
        /// Transaction type not found in fee structure.
        UnknownTransactionType,
        /// Fee amount is zero or invalid.
        InvalidFeeAmount,
        /// No fees available to withdraw.
        NoFeesAvailable,
        /// Operation would cause overflow.
        Overflow,
        /// Operation would cause underflow.
        Underflow,
    }

    /// The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Update the fee for a specific transaction type (only callable by founder).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::update_fee())]
        pub fn update_fee(
            origin: OriginFor<T>,
            transaction_type: TransactionType,
            new_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            // Only founder can update fees
            ensure!(who == T::FounderAccount::get(), Error::<T>::InsufficientFee);

            // Ensure fee is not zero
            ensure!(!new_fee.is_zero(), Error::<T>::InvalidFeeAmount);

            // Update the fee
            FixedFees::<T>::insert(transaction_type.clone(), new_fee);

            Self::deposit_event(Event::FeeUpdated {
                transaction_type,
                new_fee,
            });

            Ok(())
        }

        /// Withdraw founder's accumulated fees.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::withdraw_founder_fees())]
        pub fn withdraw_founder_fees(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            // Only founder can withdraw founder fees
            ensure!(who == T::FounderAccount::get(), Error::<T>::InsufficientFee);

            let founder_fees = FounderFees::<T>::get();
            ensure!(!founder_fees.is_zero(), Error::<T>::NoFeesAvailable);

            // Reset founder fees
            FounderFees::<T>::put(BalanceOf::<T>::zero());

            // Transfer fees to founder
            T::Currency::deposit_creating(&who, founder_fees);

            Self::deposit_event(Event::FounderFeesWithdrawn {
                amount: founder_fees,
            });

            Ok(())
        }

        /// Withdraw DAO treasury fees.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::withdraw_dao_fees())]
        pub fn withdraw_dao_fees(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            // Only DAO treasury can withdraw DAO fees
            ensure!(who == T::DaoTreasuryAccount::get(), Error::<T>::InsufficientFee);

            let dao_fees = DaoFees::<T>::get();
            ensure!(!dao_fees.is_zero(), Error::<T>::NoFeesAvailable);

            // Reset DAO fees
            DaoFees::<T>::put(BalanceOf::<T>::zero());

            // Transfer fees to DAO treasury
            T::Currency::deposit_creating(&who, dao_fees);

            Self::deposit_event(Event::DaoFeesWithdrawn {
                amount: dao_fees,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the fixed fee for a transaction type.


        /// Check if the provided fee is sufficient for the transaction type.
        pub fn check_fee(transaction_type: &TransactionType, fee_paid: BalanceOf<T>) -> bool {
            let required_fee = Self::get_fee(transaction_type);
            fee_paid >= required_fee
        }

        /// Collect and distribute fees for a transaction.
        /// This is called internally by other pallets when processing transactions.
        pub fn collect_fee(
            payer: &T::AccountId,
            transaction_type: TransactionType,
            fee_amount: BalanceOf<T>,
        ) -> DispatchResult {
            // Calculate fee distribution (15% founder, 85% DAO)
            let founder_share = fee_amount
                .checked_mul(&15u32.into())
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&100u32.into())
                .ok_or(Error::<T>::Underflow)?;
            
            let dao_share = fee_amount
                .checked_sub(&founder_share)
                .ok_or(Error::<T>::Underflow)?;

            // Update fee tracking
            let total_fees = TotalFeesCollected::<T>::get()
                .checked_add(&fee_amount)
                .ok_or(Error::<T>::Overflow)?;
            TotalFeesCollected::<T>::put(total_fees);

            let founder_fees = FounderFees::<T>::get()
                .checked_add(&founder_share)
                .ok_or(Error::<T>::Overflow)?;
            FounderFees::<T>::put(founder_fees);

            let dao_fees = DaoFees::<T>::get()
                .checked_add(&dao_share)
                .ok_or(Error::<T>::Overflow)?;
            DaoFees::<T>::put(dao_fees);

            Self::deposit_event(Event::FeeCollected {
                payer: payer.clone(),
                transaction_type,
                amount: fee_amount,
                founder_share,
                dao_share,
            });

            Ok(())
        }

        /// Initialize the default fee structure.
        /// This should be called during genesis.
        pub fn initialize_fees() {
            // TODO: Fix type conversion issues
            // Gas Fee: 0.01 FI
            // FixedFees::<T>::insert(TX_TYPE_GAS_FEE, 10_000_000_000u128);
            
            // Bridge fees
            // FixedFees::<T>::insert(TX_TYPE_BRIDGE_SMALL, 50_000_000_000u128);      // 0.05 FI
            // FixedFees::<T>::insert(TX_TYPE_BRIDGE_MEDIUM, 100_000_000_000u128);   // 0.1 FI
            // FixedFees::<T>::insert(TX_TYPE_BRIDGE_LARGE, 500_000_000_000u128);    // 0.5 FI
            // FixedFees::<T>::insert(TX_TYPE_NFT_BRIDGE, 50_000_000_000u128);       // 0.05 FI
            
            // DEX fees
            // FixedFees::<T>::insert(TX_TYPE_DEX_TRADING, 10_000_000_000u128);      // 0.01 FI
            // FixedFees::<T>::insert(TX_TYPE_POOL_SMALL, 1_000_000_000_000u128);    // 1.0 FI
            // FixedFees::<T>::insert(TX_TYPE_POOL_MEDIUM, 2_000_000_000_000u128);   // 2.0 FI
            // FixedFees::<T>::insert(TX_TYPE_POOL_LARGE, 5_000_000_000_000u128);    // 5.0 FI
            // FixedFees::<T>::insert(TX_TYPE_POOL_OPERATIONS, 10_000_000_000u128);  // 0.01 FI
            
            // Token and NFT fees
            // FixedFees::<T>::insert(TX_TYPE_TOKEN_CREATION, 500_000_000_000u128);  // 0.5 FI
            // FixedFees::<T>::insert(TX_TYPE_NFT_MINTING_SMALL, 50_000_000_000u128);  // 0.05 FI
            // FixedFees::<T>::insert(TX_TYPE_NFT_MINTING_MEDIUM, 200_000_000_000u128); // 0.2 FI
            // FixedFees::<T>::insert(TX_TYPE_NFT_MINTING_LARGE, 1_000_000_000_000u128); // 1.0 FI
            // FixedFees::<T>::insert(TX_TYPE_NFT_MINTING_XLARGE, 2_000_000_000_000u128); // 2.0 FI
            
            // Other fees
            // FixedFees::<T>::insert(TX_TYPE_VAULT_CREATION, 50_000_000_000u128);   // 0.05 FI
            // FixedFees::<T>::insert(TX_TYPE_GOVERNANCE_PROPOSAL, 500_000_000_000u128); // 0.5 FI
        }
    }
}