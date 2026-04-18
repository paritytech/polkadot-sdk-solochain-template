//! # pallet-plim-payments
//!
//! Mandate-gated payments pallet for the P:L:I:M:/Protocol.
//!
//! A *mandate* is an off-chain agreement (referenced on-chain by a 32-byte hash)
//! through which a payer pre-authorises an agent or delegatee to spend up to a
//! specified allowance, in a specified asset, until an expiration block.
//!
//! The pallet integrates with `pallet-assets` so mandates can operate over any
//! asset registered in the runtime (ePL, gPLIM, pEUR, pUSD, …). PLIM native
//! transfers continue to be handled directly by `pallet-balances`.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::traits::Currency;
use scale_info::TypeInfo;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Reference to a mandate — a 32-byte hash that uniquely identifies the
/// off-chain agreement underpinning an on-chain allowance.
pub type MandateRef = [u8; 32];

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct MandateInfo<T: Config> {
	pub payer: <T as frame_system::Config>::AccountId,
	pub payee: <T as frame_system::Config>::AccountId,
	pub asset_id: u32,
	pub allowance: u128,
	pub expires_at: frame_system::pallet_prelude::BlockNumberFor<T>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::tokens::{fungibles::Mutate, Preservation},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::SaturatedConversion;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config<AssetId = u32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The native currency (PLIM) — surfaced here so downstream mandates
		/// can gate on PLIM balances in a future extension. Asset transfers
		/// themselves route through `pallet-assets`.
		type Currency: Currency<Self::AccountId>;

		/// Maximum number of open mandates a single payer can hold concurrently.
		#[pallet::constant]
		type MaxMandatesPerAccount: Get<u32>;
	}

	/// mandate_ref → MandateInfo
	#[pallet::storage]
	pub type Mandates<T: Config> =
		StorageMap<_, Blake2_128Concat, MandateRef, MandateInfo<T>, OptionQuery>;

	/// payer → count of open mandates (for `MaxMandatesPerAccount` enforcement)
	#[pallet::storage]
	pub type MandateCount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new mandate was created on-chain.
		MandateCreated {
			payer: T::AccountId,
			payee: T::AccountId,
			asset_id: u32,
			allowance: u128,
			mandate_ref: MandateRef,
		},
		/// A mandate was revoked by the payer.
		MandateRevoked { mandate_ref: MandateRef },
		/// A payment was executed against a mandate.
		PaymentExecuted {
			from: T::AccountId,
			to: T::AccountId,
			asset_id: u32,
			amount: u128,
			mandate_ref: MandateRef,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The referenced mandate does not exist.
		MandateNotFound,
		/// The mandate's expiration block has passed.
		MandateExpired,
		/// The remaining allowance is less than the requested amount.
		InsufficientAllowance,
		/// A mandate with this reference already exists.
		MandateAlreadyExists,
		/// Only the original payer may revoke a mandate.
		NotMandateOwner,
		/// Payer has reached `MaxMandatesPerAccount`.
		TooManyMandates,
		/// The underlying `pallet-assets` transfer failed.
		AssetTransferFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new payment mandate authorising `payee` to pull up to
		/// `allowance` units of asset `asset_id` until block `expires_at`.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn create_mandate(
			origin: OriginFor<T>,
			payee: T::AccountId,
			asset_id: u32,
			allowance: u128,
			expires_at: BlockNumberFor<T>,
			mandate_ref: MandateRef,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			ensure!(!Mandates::<T>::contains_key(mandate_ref), Error::<T>::MandateAlreadyExists);

			let count = MandateCount::<T>::get(&payer);
			ensure!(count < T::MaxMandatesPerAccount::get(), Error::<T>::TooManyMandates);

			let info = MandateInfo::<T> {
				payer: payer.clone(),
				payee: payee.clone(),
				asset_id,
				allowance,
				expires_at,
			};
			Mandates::<T>::insert(mandate_ref, info);
			MandateCount::<T>::insert(&payer, count.saturating_add(1));

			Self::deposit_event(Event::MandateCreated {
				payer,
				payee,
				asset_id,
				allowance,
				mandate_ref,
			});
			Ok(())
		}

		/// Revoke a previously created mandate. Only the original payer may revoke.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn revoke_mandate(origin: OriginFor<T>, mandate_ref: MandateRef) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let info = Mandates::<T>::get(mandate_ref).ok_or(Error::<T>::MandateNotFound)?;
			ensure!(info.payer == who, Error::<T>::NotMandateOwner);

			Mandates::<T>::remove(mandate_ref);
			MandateCount::<T>::mutate(&info.payer, |c| *c = c.saturating_sub(1));

			Self::deposit_event(Event::MandateRevoked { mandate_ref });
			Ok(())
		}

		/// Execute a payment against a mandate. The caller must be the mandate's
		/// `payee`; funds flow `payer → to` in the mandate's asset.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(15_000, 0))]
		pub fn pay(
			origin: OriginFor<T>,
			to: T::AccountId,
			asset_id: u32,
			amount: u128,
			mandate_ref: MandateRef,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let mut info = Mandates::<T>::get(mandate_ref).ok_or(Error::<T>::MandateNotFound)?;
			ensure!(info.payee == caller, Error::<T>::NotMandateOwner);
			ensure!(info.asset_id == asset_id, Error::<T>::MandateNotFound);

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(now <= info.expires_at, Error::<T>::MandateExpired);
			ensure!(info.allowance >= amount, Error::<T>::InsufficientAllowance);

			info.allowance = info.allowance.saturating_sub(amount);
			let payer = info.payer.clone();
			Mandates::<T>::insert(mandate_ref, info);

			<pallet_assets::Pallet<T> as Mutate<T::AccountId>>::transfer(
				asset_id,
				&payer,
				&to,
				amount.saturated_into(),
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::AssetTransferFailed)?;

			Self::deposit_event(Event::PaymentExecuted {
				from: payer,
				to,
				asset_id,
				amount,
				mandate_ref,
			});
			Ok(())
		}
	}
}
