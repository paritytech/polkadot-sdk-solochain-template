//! # pallet-plim-timestamps
//!
//! Content anchoring: stores a mapping from a 32-byte content hash (SHA-256
//! of the off-chain blob) to `(author, anchored_at, kind)`. Used by Natali
//! quest proofs and by AIFSACCT immutable accounting entries.
//!
//! `kind` is a free-form u8 tag — downstream consumers decide its meaning
//! (1 = quest submission, 2 = aifsacct ledger entry, 3 = content engine,
//! etc).
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct AnchorInfo<T: Config> {
	pub author: <T as frame_system::Config>::AccountId,
	pub anchored_at: frame_system::pallet_prelude::BlockNumberFor<T>,
	pub kind: u8,
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
	pub type Anchors<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], AnchorInfo<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Anchored { hash: [u8; 32], author: T::AccountId, kind: u8 },
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyAnchored,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn anchor(origin: OriginFor<T>, hash: [u8; 32], kind: u8) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Anchors::<T>::contains_key(hash), Error::<T>::AlreadyAnchored);
			let info = AnchorInfo::<T> {
				author: who.clone(),
				anchored_at: frame_system::Pallet::<T>::block_number(),
				kind,
			};
			Anchors::<T>::insert(hash, info);
			Self::deposit_event(Event::Anchored { hash, author: who, kind });
			Ok(())
		}
	}
}
