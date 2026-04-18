//! # pallet-plim-mandates
//!
//! Standalone registry of recurring payment mandates. A mandate is an
//! on-chain authorisation by a payer for a specific payee to pull up to
//! `allowance` units of an asset, until `expires_at`. `spent` tracks
//! cumulative drawdown.
//!
//! This pallet is used by `pallet-plim-payments` (or any other call site)
//! via the `try_consume` helper, which performs the allowance check and
//! decrement atomically in one storage mutation.
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub type MandateRef = [u8; 32];

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct MandateInfo<T: Config> {
	pub payer: <T as frame_system::Config>::AccountId,
	pub payee: <T as frame_system::Config>::AccountId,
	pub asset_id: u32,
	pub allowance: u128,
	pub spent: u128,
	pub expires_at: frame_system::pallet_prelude::BlockNumberFor<T>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	pub type Mandates<T: Config> =
		StorageMap<_, Blake2_128Concat, MandateRef, MandateInfo<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MandateCreated {
			mandate_ref: MandateRef,
			payer: T::AccountId,
			payee: T::AccountId,
			asset_id: u32,
			allowance: u128,
		},
		MandateRevoked { mandate_ref: MandateRef },
	}

	#[pallet::error]
	pub enum Error<T> {
		MandateExists,
		MandateNotFound,
		NotAuthorized,
		MandateExpired,
		InsufficientAllowance,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a mandate. Payer (signer) pre-authorises `payee` to draw
		/// up to `allowance` of asset `asset_id` until `expires_at`.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn create(
			origin: OriginFor<T>,
			payee: T::AccountId,
			asset_id: u32,
			allowance: u128,
			expires_at: BlockNumberFor<T>,
			mandate_ref: MandateRef,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			ensure!(!Mandates::<T>::contains_key(mandate_ref), Error::<T>::MandateExists);

			let info = MandateInfo::<T> {
				payer: payer.clone(),
				payee: payee.clone(),
				asset_id,
				allowance,
				spent: 0,
				expires_at,
			};
			Mandates::<T>::insert(mandate_ref, info);
			Self::deposit_event(Event::MandateCreated {
				mandate_ref,
				payer,
				payee,
				asset_id,
				allowance,
			});
			Ok(())
		}

		/// Revoke a mandate. Either payer or payee may revoke.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn revoke(origin: OriginFor<T>, mandate_ref: MandateRef) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let info = Mandates::<T>::get(mandate_ref).ok_or(Error::<T>::MandateNotFound)?;
			ensure!(info.payer == who || info.payee == who, Error::<T>::NotAuthorized);
			Mandates::<T>::remove(mandate_ref);
			Self::deposit_event(Event::MandateRevoked { mandate_ref });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Validate and decrement the mandate's remaining allowance.
		/// Designed to be called from other pallets (e.g. pallet-plim-payments).
		pub fn try_consume(
			mandate_ref: MandateRef,
			spender: &T::AccountId,
			amount: u128,
		) -> DispatchResult {
			Mandates::<T>::try_mutate(mandate_ref, |maybe| -> DispatchResult {
				let info = maybe.as_mut().ok_or(Error::<T>::MandateNotFound)?;
				ensure!(info.payee == *spender, Error::<T>::NotAuthorized);
				let now = frame_system::Pallet::<T>::block_number();
				ensure!(now <= info.expires_at, Error::<T>::MandateExpired);
				let remaining = info.allowance.saturating_sub(info.spent);
				ensure!(remaining >= amount, Error::<T>::InsufficientAllowance);
				info.spent = info.spent.saturating_add(amount);
				Ok(())
			})
		}
	}
}
