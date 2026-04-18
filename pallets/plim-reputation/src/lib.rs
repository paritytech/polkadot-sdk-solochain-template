//! # pallet-plim-reputation
//!
//! User and agent reputation scores. Two write paths:
//!
//!  1. `award` — admin-only signed delta (positive or negative) used by the
//!     governance / arbitration layer to enforce penalties or rewards.
//!  2. `attest` — peer attestation: any signed account can give +1 to
//!     another account, but at most once per `AttestCooldown` blocks per
//!     (attester, target) pair (default ~1 week worth of blocks at 6s).
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::sp_runtime::Saturating;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin allowed to forcibly award reputation (root for v2).
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Minimum number of blocks between two `attest` calls from the same
		/// attester to the same target. ~1 week at 6s blocks ≈ 100_800.
		///
		/// Declared as `Get<u32>` (not `Get<BlockNumberFor<Self>>`) so the runtime
		/// can supply a plain `ConstU32` without `parameter_types!{}` plumbing.
		/// We convert at the use site via `.into()`.
		#[pallet::constant]
		type AttestCooldown: Get<u32>;
	}

	#[pallet::storage]
	pub type Scores<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, i64, ValueQuery>;

	#[pallet::storage]
	pub type LastAttestation<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		BlockNumberFor<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ReputationChanged { who: T::AccountId, delta: i32, new_total: i64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		RateLimited,
		SelfAttestNotAllowed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Admin-only reputation adjustment (positive or negative delta).
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn award(origin: OriginFor<T>, who: T::AccountId, delta: i32) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let new_total = Scores::<T>::mutate(&who, |s| {
				*s = s.saturating_add(delta as i64);
				*s
			});
			Self::deposit_event(Event::ReputationChanged { who, delta, new_total });
			Ok(())
		}

		/// Peer attestation: signed origin gives +1 to `target`. Rate-limited
		/// per (attester, target) pair to prevent sybil amplification.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn attest(origin: OriginFor<T>, target: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who != target, Error::<T>::SelfAttestNotAllowed);

			let now = frame_system::Pallet::<T>::block_number();
			if let Some(last) = LastAttestation::<T>::get(&who, &target) {
				let elapsed = now.saturating_sub(last);
				let cooldown: BlockNumberFor<T> = T::AttestCooldown::get().into();
				ensure!(elapsed >= cooldown, Error::<T>::RateLimited);
			}

			LastAttestation::<T>::insert(&who, &target, now);
			let new_total = Scores::<T>::mutate(&target, |s| {
				*s = s.saturating_add(1);
				*s
			});
			Self::deposit_event(Event::ReputationChanged {
				who: target,
				delta: 1,
				new_total,
			});
			Ok(())
		}
	}
}
