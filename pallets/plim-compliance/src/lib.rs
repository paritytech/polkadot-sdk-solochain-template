//! # pallet-plim-compliance
//!
//! Compliance gating for regulated assets (pEUR, pUSD). Maintains:
//!  - a sanctioned-account denylist (`Sanctioned`)
//!  - a set of regulated asset ids that require compliance checks
//!    (`RegulatedAssets`)
//!
//! `ensure_allowed(asset_id, who)` is the public hook that other pallets
//! (and a future `pallet_assets::Freezer` adapter) call to enforce the
//! denylist before any transfer of a regulated asset.
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin allowed to manage sanctions and regulated assets (root for v2).
		type ComplianceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::storage]
	pub type Sanctioned<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pallet::storage]
	pub type RegulatedAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Sanctioned { who: T::AccountId },
		Unsanctioned { who: T::AccountId },
		AssetRegulated { asset_id: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		AccountSanctioned,
		AssetNotRegulated,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn sanction(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			Sanctioned::<T>::insert(&who, ());
			Self::deposit_event(Event::Sanctioned { who });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn unsanction(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			Sanctioned::<T>::remove(&who);
			Self::deposit_event(Event::Unsanctioned { who });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn mark_regulated(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			RegulatedAssets::<T>::insert(asset_id, ());
			Self::deposit_event(Event::AssetRegulated { asset_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Compliance gate: returns `Ok(())` if `who` is allowed to transact
		/// `asset_id`. Non-regulated assets are always allowed; sanctioned
		/// accounts are blocked from regulated assets.
		pub fn ensure_allowed(asset_id: u32, who: &T::AccountId) -> DispatchResult {
			if RegulatedAssets::<T>::contains_key(asset_id) {
				ensure!(!Sanctioned::<T>::contains_key(who), Error::<T>::AccountSanctioned);
			}
			Ok(())
		}
	}
}
