//! # pallet-plim-identity
//!
//! KYC / identity claims for the P:L:I:M:/Protocol.
//!
//! Users self-register with a display name + ISO-3166 country code. A trusted
//! verifier (for now, root) can subsequently attest that an on-chain identity
//! corresponds to a KYC'd off-chain person or legal entity. Revocation is
//! symmetric — a verifier can strip a previous attestation if a user is later
//! found to be non-compliant.
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::BoundedVec;
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct IdentityInfo<T: Config> {
	pub display_name: BoundedVec<u8, <T as Config>::MaxNameLen>,
	pub country: [u8; 3],
	pub verified_at: Option<frame_system::pallet_prelude::BlockNumberFor<T>>,
	pub verifier: Option<<T as frame_system::Config>::AccountId>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Maximum length of the display name.
		#[pallet::constant]
		type MaxNameLen: Get<u32>;

		/// Origin allowed to verify / revoke identities (root for v2).
		type VerifierOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::storage]
	pub type Identities<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, IdentityInfo<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		IdentityRegistered { who: T::AccountId, country: [u8; 3] },
		IdentityVerified { who: T::AccountId },
		IdentityRevoked { who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyRegistered,
		NotRegistered,
		AlreadyVerified,
		NameTooLong,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Self-register an identity. `verified_at` is left `None` until a
		/// verifier attests it.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn register(
			origin: OriginFor<T>,
			display_name: Vec<u8>,
			country: [u8; 3],
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Identities::<T>::contains_key(&who), Error::<T>::AlreadyRegistered);

			let bounded: BoundedVec<u8, T::MaxNameLen> =
				display_name.try_into().map_err(|_| Error::<T>::NameTooLong)?;

			let info = IdentityInfo::<T> {
				display_name: bounded,
				country,
				verified_at: None,
				verifier: None,
			};
			Identities::<T>::insert(&who, info);

			Self::deposit_event(Event::IdentityRegistered { who, country });
			Ok(())
		}

		/// Mark an existing identity as verified. Restricted to `VerifierOrigin`.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn verify(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::VerifierOrigin::ensure_origin(origin)?;
			Identities::<T>::try_mutate(&who, |maybe| -> DispatchResult {
				let info = maybe.as_mut().ok_or(Error::<T>::NotRegistered)?;
				ensure!(info.verified_at.is_none(), Error::<T>::AlreadyVerified);
				info.verified_at = Some(frame_system::Pallet::<T>::block_number());
				// v2: VerifierOrigin is EnsureRoot which yields no AccountId.
				// `verifier` is reserved for v3 once we wire a multi-sig council.
				info.verifier = None;
				Ok(())
			})?;
			Self::deposit_event(Event::IdentityVerified { who });
			Ok(())
		}

		/// Revoke a previously verified identity.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn revoke(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::VerifierOrigin::ensure_origin(origin)?;
			Identities::<T>::try_mutate(&who, |maybe| -> DispatchResult {
				let info = maybe.as_mut().ok_or(Error::<T>::NotRegistered)?;
				info.verified_at = None;
				info.verifier = None;
				Ok(())
			})?;
			Self::deposit_event(Event::IdentityRevoked { who });
			Ok(())
		}
	}
}
