//! Unit tests for pallet-plim-payments.

#![cfg(test)]

use crate::{mock::*, Error, MandateRef, Mandates};
use frame_support::{assert_noop, assert_ok};

const REF_A: MandateRef = [0xAA; 32];

#[test]
fn create_mandate_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(crate::Pallet::<Test>::create_mandate(
			frame_system::RawOrigin::Signed(1).into(),
			2,
			1, // asset_id
			1_000,
			100, // expires_at
			REF_A,
		));
		assert!(Mandates::<Test>::contains_key(REF_A));
	});
}

#[test]
fn cannot_create_duplicate_mandate() {
	new_test_ext().execute_with(|| {
		assert_ok!(crate::Pallet::<Test>::create_mandate(
			frame_system::RawOrigin::Signed(1).into(),
			2,
			1,
			1_000,
			100,
			REF_A,
		));
		assert_noop!(
			crate::Pallet::<Test>::create_mandate(
				frame_system::RawOrigin::Signed(1).into(),
				2,
				1,
				1_000,
				100,
				REF_A,
			),
			Error::<Test>::MandateAlreadyExists
		);
	});
}

#[test]
fn revoke_mandate_only_by_payer() {
	new_test_ext().execute_with(|| {
		assert_ok!(crate::Pallet::<Test>::create_mandate(
			frame_system::RawOrigin::Signed(1).into(),
			2,
			1,
			1_000,
			100,
			REF_A,
		));
		// Non-payer cannot revoke.
		assert_noop!(
			crate::Pallet::<Test>::revoke_mandate(
				frame_system::RawOrigin::Signed(2).into(),
				REF_A
			),
			Error::<Test>::NotMandateOwner
		);
		// Payer can.
		assert_ok!(crate::Pallet::<Test>::revoke_mandate(
			frame_system::RawOrigin::Signed(1).into(),
			REF_A
		));
		assert!(!Mandates::<Test>::contains_key(REF_A));
	});
}
