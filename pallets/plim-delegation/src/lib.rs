//! # pallet-plim-delegation
//!
//! Autonomous-agent spending delegation: a delegator pre-authorises a
//! delegate (e.g. the pLim Agent backend) to spend up to `daily_limit`
//! units of `asset_id` per day, until `expires_at`.
//!
//! Day rolls over when the current block number exceeds the stored `day`
//! marker by `BlocksPerDay` (≈ 14_400 at 6s blocks). A rollover resets
//! `used_today` to zero and updates `day`.
//!
//! NOTE — the actual asset transfer is **not** executed by this pallet;
//! `try_spend` only validates and decrements the daily counter, then emits
//! `DelegationSpent`. Caller is expected to perform the underlying transfer
//! via pallet-assets / pallet-balances.
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct DelegationInfo<T: Config> {
	pub asset_id: u32,
	pub daily_limit: u128,
	pub expires_at: frame_system::pallet_prelude::BlockNumberFor<T>,
	pub used_today: u128,
	pub day: frame_system::pallet_prelude::BlockNumberFor<T>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::sp_runtime::Saturating;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Approximate number of blocks per day (14_400 at 6s blocks).
		///
		/// Declared as `Get<u32>` so the runtime can supply a `ConstU32` directly
		/// without `parameter_types!{}` plumbing — converted at use site via `.into()`.
		#[pallet::constant]
		type BlocksPerDay: Get<u32>;
	}

	#[pallet::storage]
	pub type Delegations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // delegator
		Blake2_128Concat,
		T::AccountId, // delegate
		DelegationInfo<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		DelegationCreated {
			delegator: T::AccountId,
			delegate: T::AccountId,
			asset_id: u32,
			daily_limit: u128,
		},
		DelegationRevoked { delegator: T::AccountId, delegate: T::AccountId },
		DelegationSpent {
			delegator: T::AccountId,
			delegate: T::AccountId,
			amount: u128,
			used_today: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		DelegationNotFound,
		DelegationExpired,
		DailyLimitExceeded,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create or overwrite a delegation from the signer to `delegate`.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn delegate(
			origin: OriginFor<T>,
			delegate: T::AccountId,
			asset_id: u32,
			daily_limit: u128,
			expires_at: BlockNumberFor<T>,
		) -> DispatchResult {
			let delegator = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			let info = DelegationInfo::<T> {
				asset_id,
				daily_limit,
				expires_at,
				used_today: 0,
				day: now,
			};
			Delegations::<T>::insert(&delegator, &delegate, info);
			Self::deposit_event(Event::DelegationCreated {
				delegator,
				delegate,
				asset_id,
				daily_limit,
			});
			Ok(())
		}

		/// Revoke a delegation. Only the delegator may revoke.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn revoke(origin: OriginFor<T>, delegate: T::AccountId) -> DispatchResult {
			let delegator = ensure_signed(origin)?;
			ensure!(
				Delegations::<T>::contains_key(&delegator, &delegate),
				Error::<T>::DelegationNotFound
			);
			Delegations::<T>::remove(&delegator, &delegate);
			Self::deposit_event(Event::DelegationRevoked { delegator, delegate });
			Ok(())
		}

		/// Delegate-initiated spend: validate daily limit and decrement.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn try_spend(
			origin: OriginFor<T>,
			delegator: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			let delegate = ensure_signed(origin)?;
			Delegations::<T>::try_mutate(&delegator, &delegate, |maybe| -> DispatchResult {
				let info = maybe.as_mut().ok_or(Error::<T>::DelegationNotFound)?;
				let now = frame_system::Pallet::<T>::block_number();
				ensure!(now <= info.expires_at, Error::<T>::DelegationExpired);

				// Day rollover.
				let blocks_per_day: BlockNumberFor<T> = T::BlocksPerDay::get().into();
				if now.saturating_sub(info.day) >= blocks_per_day {
					info.day = now;
					info.used_today = 0;
				}

				let new_used = info.used_today.saturating_add(amount);
				ensure!(new_used <= info.daily_limit, Error::<T>::DailyLimitExceeded);
				info.used_today = new_used;

				Self::deposit_event(Event::DelegationSpent {
					delegator: delegator.clone(),
					delegate: delegate.clone(),
					amount,
					used_today: new_used,
				});
				Ok(())
			})
		}
	}
}
