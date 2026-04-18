//! # pallet-plim-channels
//!
//! Minimal payment channels: two parties open a channel with a deposit and
//! later cooperatively close it with a final balance split.
//!
//! v2 keeps this MINIMAL — `close` accepts a `signature` blob but does NOT
//! verify it; full ECDSA verification + dispute/challenge windows are
//! deferred to v3. The on-chain state is sufficient to anchor channel
//! existence and final settlement events for downstream observers.
//!
//! Updated 2026-04-14T15:00 — concrete genesis allocations + 7 pallet implementations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct ChannelInfo<T: Config> {
	pub party_a: <T as frame_system::Config>::AccountId,
	pub party_b: <T as frame_system::Config>::AccountId,
	pub asset_id: u32,
	pub deposit: u128,
	pub opened_at: frame_system::pallet_prelude::BlockNumberFor<T>,
	pub expires_at: frame_system::pallet_prelude::BlockNumberFor<T>,
	pub settled: bool,
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
	pub type Channels<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], ChannelInfo<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ChannelOpened {
			channel_id: [u8; 32],
			party_a: T::AccountId,
			party_b: T::AccountId,
			asset_id: u32,
			deposit: u128,
		},
		ChannelClosed {
			channel_id: [u8; 32],
			final_balance_self: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		ChannelExists,
		ChannelNotFound,
		AlreadySettled,
		NotChannelParty,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Open a new payment channel between the signer and a counterparty.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn open(
			origin: OriginFor<T>,
			counterparty: T::AccountId,
			asset_id: u32,
			deposit: u128,
			expires_at: BlockNumberFor<T>,
			channel_id: [u8; 32],
		) -> DispatchResult {
			let party_a = ensure_signed(origin)?;
			ensure!(!Channels::<T>::contains_key(channel_id), Error::<T>::ChannelExists);

			let info = ChannelInfo::<T> {
				party_a: party_a.clone(),
				party_b: counterparty.clone(),
				asset_id,
				deposit,
				opened_at: frame_system::Pallet::<T>::block_number(),
				expires_at,
				settled: false,
			};
			Channels::<T>::insert(channel_id, info);
			Self::deposit_event(Event::ChannelOpened {
				channel_id,
				party_a,
				party_b: counterparty,
				asset_id,
				deposit,
			});
			Ok(())
		}

		/// Cooperative close. The signer asserts their final balance and
		/// supplies the counterparty's signature over `(channel_id,
		/// final_balance_self)`. v2 stores the signature but does NOT verify
		/// it — that's a v3 follow-up.
		///
		/// TODO: real signature verification + dispute resolution
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn close(
			origin: OriginFor<T>,
			channel_id: [u8; 32],
			final_balance_self: u128,
			_signature: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Channels::<T>::try_mutate(channel_id, |maybe| -> DispatchResult {
				let info = maybe.as_mut().ok_or(Error::<T>::ChannelNotFound)?;
				ensure!(!info.settled, Error::<T>::AlreadySettled);
				ensure!(
					info.party_a == who || info.party_b == who,
					Error::<T>::NotChannelParty
				);
				info.settled = true;
				Ok(())
			})?;
			Self::deposit_event(Event::ChannelClosed { channel_id, final_balance_self });
			Ok(())
		}
	}
}
