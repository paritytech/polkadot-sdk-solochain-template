//! # DEX Pallet
//!
#![cfg_attr(not(feature = "std"), no_std)]

//! This pallet implements the decentralized exchange (DEX) for the CREATEFI blockchain.
//! It provides AMM (Automated Market Maker) pools, order book trading, and liquidity provision.

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
    use sp_runtime::traits::{CheckedAdd, CheckedSub, CheckedMul, CheckedDiv, Zero, IntegerSquareRoot};
    use sp_std::vec::Vec;

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The currency type for handling balances.
        type Currency: Currency<Self::AccountId>;
        
        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: WeightInfo;
        
        /// The trading fee percentage (in basis points, e.g., 30 = 0.3%).
        #[pallet::constant]
        type TradingFeeBps: Get<u32>;
        
        /// The liquidity provider fee percentage (in basis points).
        #[pallet::constant]
        type LpFeeBps: Get<u32>;
        
        /// The minimum liquidity required to create a pool.
        #[pallet::constant]
        type MinLiquidity: Get<BalanceOf<Self>>;
        
        /// The maximum slippage tolerance (in basis points).
        #[pallet::constant]
        type MaxSlippageBps: Get<u32>;
    }

    /// Balance type for this pallet.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Pool identifier.
    pub type PoolId = u64;

    /// Order identifier.
    pub type OrderId = u64;

    /// Token pair for trading.
    pub type TokenPair = BoundedVec<u8, ConstU32<64>>; // Bounded string representation

    /// AMM Pool information.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Pool<T: Config> {
        pub id: PoolId,
        pub token_pair: TokenPair,
        pub reserve_a: BalanceOf<T>,
        pub reserve_b: BalanceOf<T>,
        pub total_liquidity: BalanceOf<T>,
        pub lp_token_supply: BalanceOf<T>,
        pub fee_collector: <T as frame_system::Config>::AccountId,
    }

    /// Order book order.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Order<T: Config> {
        pub id: OrderId,
        pub trader: <T as frame_system::Config>::AccountId,
        pub token_pair: TokenPair,
        pub side: OrderSide,
        pub amount: BalanceOf<T>,
        pub price: BalanceOf<T>,
        pub filled_amount: BalanceOf<T>,
        pub status: OrderStatus,
    }

    /// Order side (buy/sell).
    pub type OrderSide = u8;
    pub const ORDER_SIDE_BUY: OrderSide = 0;
    pub const ORDER_SIDE_SELL: OrderSide = 1;

    /// Order status.
    pub type OrderStatus = u8;
    pub const ORDER_STATUS_OPEN: OrderStatus = 0;
    pub const ORDER_STATUS_PARTIALLY_FILLED: OrderStatus = 1;
    pub const ORDER_STATUS_FILLED: OrderStatus = 2;
    pub const ORDER_STATUS_CANCELLED: OrderStatus = 3;

    /// The pallet's storage items.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Next pool ID.
    #[pallet::storage]
    pub type NextPoolId<T> = StorageValue<_, PoolId, ValueQuery>;

    /// Next order ID.
    #[pallet::storage]
    pub type NextOrderId<T> = StorageValue<_, OrderId, ValueQuery>;

    /// AMM pools by ID.
    #[pallet::storage]
    #[pallet::getter(fn get_pool)]
    pub type Pools<T> = StorageMap<_, Blake2_128Concat, PoolId, Pool<T>, OptionQuery>;

    /// Pool IDs by token pair.
    #[pallet::storage]
    pub type PoolIds<T> = StorageMap<_, Blake2_128Concat, TokenPair, PoolId, OptionQuery>;

    /// LP token balances for each account and pool.
    #[pallet::storage]
    #[pallet::getter(fn get_lp_balance)]
    pub type LpBalances<T> = StorageDoubleMap<_, Blake2_128Concat, PoolId, Blake2_128Concat, <T as frame_system::Config>::AccountId, BalanceOf<T>, ValueQuery>;

    /// Order book orders by ID.
    #[pallet::storage]
    #[pallet::getter(fn get_order)]
    pub type Orders<T> = StorageMap<_, Blake2_128Concat, OrderId, Order<T>, OptionQuery>;

    /// Open buy orders by token pair and price (sorted by price desc).
    #[pallet::storage]
    pub type BuyOrders<T> = StorageDoubleMap<_, Blake2_128Concat, TokenPair, Blake2_128Concat, BalanceOf<T>, BoundedVec<OrderId, ConstU32<100>>, ValueQuery>;

    /// Open sell orders by token pair and price (sorted by price asc).
    #[pallet::storage]
    pub type SellOrders<T> = StorageDoubleMap<_, Blake2_128Concat, TokenPair, Blake2_128Concat, BalanceOf<T>, BoundedVec<OrderId, ConstU32<100>>, ValueQuery>;

    /// Total trading volume by token pair.
    #[pallet::storage]
    #[pallet::getter(fn get_trading_volume)]
    pub type TradingVolume<T> = StorageMap<_, Blake2_128Concat, TokenPair, BalanceOf<T>, ValueQuery>;

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// AMM pool was created.
        /// [pool_id, token_pair, creator, initial_liquidity_a, initial_liquidity_b]
        PoolCreated {
            pool_id: PoolId,
            token_pair: TokenPair,
            creator: T::AccountId,
            initial_liquidity_a: BalanceOf<T>,
            initial_liquidity_b: BalanceOf<T>,
        },
        /// Liquidity was added to a pool.
        /// [pool_id, provider, amount_a, amount_b, lp_tokens_minted]
        LiquidityAdded {
            pool_id: PoolId,
            provider: T::AccountId,
            amount_a: BalanceOf<T>,
            amount_b: BalanceOf<T>,
            lp_tokens_minted: BalanceOf<T>,
        },
        /// Liquidity was removed from a pool.
        /// [pool_id, provider, amount_a, amount_b, lp_tokens_burned]
        LiquidityRemoved {
            pool_id: PoolId,
            provider: T::AccountId,
            amount_a: BalanceOf<T>,
            amount_b: BalanceOf<T>,
            lp_tokens_burned: BalanceOf<T>,
        },
        /// AMM trade was executed.
        /// [pool_id, trader, token_in, token_out, amount_in, amount_out, fee]
        AmmTrade {
            pool_id: PoolId,
            trader: T::AccountId,
            token_in: BoundedVec<u8, ConstU32<32>>,
            token_out: BoundedVec<u8, ConstU32<32>>,
            amount_in: BalanceOf<T>,
            amount_out: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        /// Order book order was placed.
        /// [order_id, trader, token_pair, side, amount, price]
        OrderPlaced {
            order_id: OrderId,
            trader: T::AccountId,
            token_pair: TokenPair,
            side: OrderSide,
            amount: BalanceOf<T>,
            price: BalanceOf<T>,
        },
        /// Order book trade was executed.
        /// [order_id, trader, token_pair, side, amount, price, fee]
        OrderBookTrade {
            order_id: OrderId,
            trader: T::AccountId,
            token_pair: TokenPair,
            side: OrderSide,
            amount: BalanceOf<T>,
            price: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        /// Order was cancelled.
        /// [order_id, trader, remaining_amount]
        OrderCancelled {
            order_id: OrderId,
            trader: T::AccountId,
            remaining_amount: BalanceOf<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Pool already exists for this token pair.
        PoolAlreadyExists,
        /// Pool does not exist.
        PoolNotFound,
        /// Insufficient liquidity in pool.
        InsufficientLiquidity,
        /// Insufficient balance for operation.
        InsufficientBalance,
        /// Amount is below minimum required.
        AmountBelowMinimum,
        /// Slippage tolerance exceeded.
        SlippageExceeded,
        /// Invalid token pair.
        InvalidTokenPair,
        /// Order not found.
        OrderNotFound,
        /// Order is not open.
        OrderNotOpen,
        /// Insufficient order amount.
        InsufficientOrderAmount,
        /// Price impact too high.
        PriceImpactTooHigh,
        /// Operation would cause overflow.
        Overflow,
        /// Operation would cause underflow.
        Underflow,
    }

    /// The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new AMM pool.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_pool())]
        pub fn create_pool(
            origin: OriginFor<T>,
            token_a: BoundedVec<u8, ConstU32<32>>,
            token_b: BoundedVec<u8, ConstU32<32>>,
            initial_liquidity_a: BalanceOf<T>,
            initial_liquidity_b: BalanceOf<T>,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;

            // Ensure minimum liquidity
            ensure!(initial_liquidity_a >= T::MinLiquidity::get(), Error::<T>::AmountBelowMinimum);
            ensure!(initial_liquidity_b >= T::MinLiquidity::get(), Error::<T>::AmountBelowMinimum);

            // Create token pair by concatenating token_a, "-", and token_b
            let mut token_pair_vec = Vec::new();
            token_pair_vec.extend_from_slice(&token_a);
            token_pair_vec.extend_from_slice(b"-");
            token_pair_vec.extend_from_slice(&token_b);
            let token_pair: TokenPair = token_pair_vec.try_into().map_err(|_| Error::<T>::InvalidTokenPair)?;
            
            // Check if pool already exists
            ensure!(PoolIds::<T>::get(&token_pair).is_none(), Error::<T>::PoolAlreadyExists);

            let pool_id = NextPoolId::<T>::get();
            NextPoolId::<T>::put(pool_id + 1);

            // Calculate initial LP tokens (geometric mean)
            let initial_lp_tokens = initial_liquidity_a.checked_mul(&initial_liquidity_b)
                .ok_or(Error::<T>::Overflow)?
                .integer_sqrt();

            let pool = Pool {
                id: pool_id,
                token_pair: token_pair.clone(),
                reserve_a: initial_liquidity_a,
                reserve_b: initial_liquidity_b,
                total_liquidity: initial_lp_tokens,
                lp_token_supply: initial_lp_tokens,
                fee_collector: creator.clone(),
            };

            Pools::<T>::insert(pool_id, pool);
            PoolIds::<T>::insert(&token_pair, pool_id);
            LpBalances::<T>::insert(pool_id, &creator, initial_lp_tokens);

            Self::deposit_event(Event::PoolCreated {
                pool_id,
                token_pair,
                creator,
                initial_liquidity_a,
                initial_liquidity_b,
            });

            Ok(())
        }

        /// Add liquidity to an AMM pool.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_liquidity())]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amount_a: BalanceOf<T>,
            amount_b: BalanceOf<T>,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;

            // Calculate LP tokens to mint based on current reserves
            let lp_tokens = if pool.lp_token_supply.is_zero() {
                amount_a.checked_mul(&amount_b)
                    .ok_or(Error::<T>::Overflow)?
                    .integer_sqrt()
            } else {
                let lp_tokens_a = amount_a.checked_mul(&pool.lp_token_supply)
                    .ok_or(Error::<T>::Overflow)?
                    .checked_div(&pool.reserve_a)
                    .ok_or(Error::<T>::Underflow)?;
                
                let lp_tokens_b = amount_b.checked_mul(&pool.lp_token_supply)
                    .ok_or(Error::<T>::Overflow)?
                    .checked_div(&pool.reserve_b)
                    .ok_or(Error::<T>::Underflow)?;
                
                lp_tokens_a.min(lp_tokens_b)
            };

            // Update pool reserves
            let new_reserve_a = pool.reserve_a.checked_add(&amount_a)
                .ok_or(Error::<T>::Overflow)?;
            let new_reserve_b = pool.reserve_b.checked_add(&amount_b)
                .ok_or(Error::<T>::Overflow)?;
            let new_lp_supply = pool.lp_token_supply.checked_add(&lp_tokens)
                .ok_or(Error::<T>::Overflow)?;

            let updated_pool = Pool {
                reserve_a: new_reserve_a,
                reserve_b: new_reserve_b,
                lp_token_supply: new_lp_supply,
                ..pool
            };

            Pools::<T>::insert(pool_id, updated_pool);

            // Update LP balance
            let current_lp_balance = LpBalances::<T>::get(pool_id, &provider);
            let new_lp_balance = current_lp_balance.checked_add(&lp_tokens)
                .ok_or(Error::<T>::Overflow)?;
            LpBalances::<T>::insert(pool_id, &provider, new_lp_balance);

            Self::deposit_event(Event::LiquidityAdded {
                pool_id,
                provider,
                amount_a,
                amount_b,
                lp_tokens_minted: lp_tokens,
            });

            Ok(())
        }

        /// Execute an AMM trade.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::amm_trade())]
        pub fn amm_trade(
            origin: OriginFor<T>,
            pool_id: PoolId,
            token_in: BoundedVec<u8, ConstU32<32>>,
            amount_in: BalanceOf<T>,
            min_amount_out: BalanceOf<T>,
        ) -> DispatchResult {
            let trader = ensure_signed(origin)?;

            let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;

            // Determine which token is being traded
            // For simplicity, assume token_a is the first part of the token_pair
            // Find the position of the "-" separator
            let mut dash_pos = None;
            for (i, &byte) in pool.token_pair.iter().enumerate() {
                if byte == b'-' {
                    dash_pos = Some(i);
                    break;
                }
            }
            
            let dash_pos = dash_pos.ok_or(Error::<T>::InvalidTokenPair)?;
            
            // Split the token pair into token_a and token_b
            let token_a = pool.token_pair[..dash_pos].to_vec();
            let token_b = pool.token_pair[(dash_pos + 1)..].to_vec();
            
            let (reserve_in, reserve_out, token_out) = if token_in == token_a {
                let token_out_bounded: BoundedVec<u8, ConstU32<32>> = token_b.try_into().map_err(|_| Error::<T>::InvalidTokenPair)?;
                (pool.reserve_a, pool.reserve_b, token_out_bounded)
            } else if token_in == token_b {
                let token_out_bounded: BoundedVec<u8, ConstU32<32>> = token_a.clone().try_into().map_err(|_| Error::<T>::InvalidTokenPair)?;
                (pool.reserve_b, pool.reserve_a, token_out_bounded)
            } else {
                return Err(Error::<T>::InvalidTokenPair.into());
            };

            // Calculate output amount using constant product formula
            let trading_fee = amount_in.checked_mul(&T::TradingFeeBps::get().into())
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&10_000u32.into())
                .ok_or(Error::<T>::Underflow)?;
            
            let amount_in_with_fee = amount_in.checked_sub(&trading_fee)
                .ok_or(Error::<T>::Underflow)?;

            let amount_out = reserve_out.checked_mul(&amount_in_with_fee)
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&(reserve_in.checked_add(&amount_in_with_fee)
                    .ok_or(Error::<T>::Overflow)?))
                .ok_or(Error::<T>::Underflow)?;

            // Check slippage
            ensure!(amount_out >= min_amount_out, Error::<T>::SlippageExceeded);

            // Update pool reserves
            let new_reserve_in = reserve_in.checked_add(&amount_in)
                .ok_or(Error::<T>::Overflow)?;
            let new_reserve_out = reserve_out.checked_sub(&amount_out)
                .ok_or(Error::<T>::Underflow)?;

            // Update trading volume
            let token_pair = pool.token_pair.clone();
            let current_volume = TradingVolume::<T>::get(&token_pair);
            let new_volume = current_volume.checked_add(&amount_in)
                .ok_or(Error::<T>::Overflow)?;
            TradingVolume::<T>::insert(&token_pair, new_volume);

            let updated_pool = if token_in == token_a {
                Pool { reserve_a: new_reserve_in, reserve_b: new_reserve_out, ..pool }
            } else {
                Pool { reserve_a: new_reserve_out, reserve_b: new_reserve_in, ..pool }
            };

            Pools::<T>::insert(pool_id, updated_pool);

            Self::deposit_event(Event::AmmTrade {
                pool_id,
                trader,
                token_in,
                token_out,
                amount_in,
                amount_out,
                fee: trading_fee,
            });

            Ok(())
        }

        /// Place an order book order.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::place_order())]
        pub fn place_order(
            origin: OriginFor<T>,
            token_pair: TokenPair,
            side: OrderSide,
            amount: BalanceOf<T>,
            price: BalanceOf<T>,
        ) -> DispatchResult {
            let trader = ensure_signed(origin)?;

            ensure!(!amount.is_zero(), Error::<T>::AmountBelowMinimum);
            ensure!(!price.is_zero(), Error::<T>::AmountBelowMinimum);

            let order_id = NextOrderId::<T>::get();
            NextOrderId::<T>::put(order_id + 1);

            let order = Order {
                id: order_id,
                trader: trader.clone(),
                token_pair: token_pair.clone(),
                side,
                amount,
                price,
                filled_amount: Zero::zero(),
                status: ORDER_STATUS_OPEN,
            };

            Orders::<T>::insert(order_id, order);

            // Add to order book
            match side {
                ORDER_SIDE_BUY => {
                    let mut buy_orders = BuyOrders::<T>::get(&token_pair, price);
                    buy_orders.try_push(order_id).map_err(|_| Error::<T>::Overflow)?;
                    BuyOrders::<T>::insert(&token_pair, price, buy_orders);
                },
                ORDER_SIDE_SELL => {
                    let mut sell_orders = SellOrders::<T>::get(&token_pair, price);
                    sell_orders.try_push(order_id).map_err(|_| Error::<T>::Overflow)?;
                    SellOrders::<T>::insert(&token_pair, price, sell_orders);
                },
                _ => return Err(Error::<T>::InvalidTokenPair.into()),
            }

            Self::deposit_event(Event::OrderPlaced {
                order_id,
                trader,
                token_pair,
                side,
                amount,
                price,
            });

            Ok(())
        }

        /// Cancel an order book order.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::cancel_order())]
        pub fn cancel_order(origin: OriginFor<T>, order_id: OrderId) -> DispatchResult {
            let trader = ensure_signed(origin)?;

            let mut order = Orders::<T>::get(order_id).ok_or(Error::<T>::OrderNotFound)?;
            
            ensure!(order.trader == trader, Error::<T>::InsufficientBalance);
            ensure!(order.status == ORDER_STATUS_OPEN || order.status == ORDER_STATUS_PARTIALLY_FILLED, Error::<T>::OrderNotOpen);

            let remaining_amount = order.amount.checked_sub(&order.filled_amount)
                .ok_or(Error::<T>::Underflow)?;

            order.status = ORDER_STATUS_CANCELLED;
            Orders::<T>::insert(order_id, order);

            Self::deposit_event(Event::OrderCancelled {
                order_id,
                trader,
                remaining_amount,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the current price of a token pair from AMM pool.
        pub fn get_amm_price(pool_id: PoolId) -> Option<(BalanceOf<T>, BalanceOf<T>)> {
            let pool = Pools::<T>::get(pool_id)?;
            Some((pool.reserve_a, pool.reserve_b))
        }

        /// Calculate the amount out for a given amount in using AMM formula.
        pub fn calculate_amm_output(
            reserve_in: BalanceOf<T>,
            reserve_out: BalanceOf<T>,
            amount_in: BalanceOf<T>,
        ) -> Option<BalanceOf<T>> {
            let trading_fee = amount_in.checked_mul(&T::TradingFeeBps::get().into())?
                .checked_div(&10_000u32.into())?;
            
            let amount_in_with_fee = amount_in.checked_sub(&trading_fee)?;
            
            reserve_out.checked_mul(&amount_in_with_fee)?
                .checked_div(&(reserve_in.checked_add(&amount_in_with_fee)?))
        }

        /// Get the best bid and ask prices for a token pair.
        pub fn get_order_book_prices(token_pair: &TokenPair) -> Option<(BalanceOf<T>, BalanceOf<T>)> {
            // Get best bid (highest buy price)
            let buy_orders = BuyOrders::<T>::iter_prefix(token_pair)
                .max_by_key(|(price, _)| *price)?;
            
            // Get best ask (lowest sell price)
            let sell_orders = SellOrders::<T>::iter_prefix(token_pair)
                .min_by_key(|(price, _)| *price)?;
            
            Some((buy_orders.0, sell_orders.0))
        }
    }
}