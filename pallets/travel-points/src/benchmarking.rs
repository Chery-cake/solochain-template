//! Benchmarking setup for pallet-travel-points
//!
//! This module contains benchmarks for measuring the weight (execution time and storage)
//! of each extrinsic in the travel points pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as TravelPoints;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn award_points() {
		// Setup: Create an admin and authorized issuer
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let recipient: T::AccountId = account("recipient", 0, 0);
		let amount: u128 = 1000;

		#[extrinsic_call]
		award_points(
			RawOrigin::Signed(issuer),
			recipient.clone(),
			amount,
			TravelType::Airline,
			None,
		);

		// Verify the result
		assert_eq!(TotalPoints::<T>::get(&recipient), amount);
	}

	#[benchmark]
	fn spend_points() {
		// Setup: Create a user with points
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let user: T::AccountId = account("user", 0, 0);

		// Award points first
		let _ = TravelPoints::<T>::award_points(
			RawOrigin::Signed(issuer).into(),
			user.clone(),
			2000,
			TravelType::Airline,
			None,
		);

		let spend_amount: u128 = 500;

		#[extrinsic_call]
		spend_points(RawOrigin::Signed(user.clone()), spend_amount);

		// Verify the result
		assert_eq!(TotalPoints::<T>::get(&user), 1500);
	}

	#[benchmark]
	fn cleanup_expired() {
		// Setup: Create a user with some points
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let user: T::AccountId = account("user", 0, 0);

		// Award points with very short expiration
		let _ = TravelPoints::<T>::award_points(
			RawOrigin::Signed(issuer).into(),
			user.clone(),
			1000,
			TravelType::Train,
			Some(1u32.into()),
		);

		let caller: T::AccountId = account("caller", 0, 0);

		#[extrinsic_call]
		cleanup_expired(RawOrigin::Signed(caller), user.clone());
	}

	#[benchmark]
	fn authorize_issuer() {
		// Setup: Create an admin
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let new_issuer: T::AccountId = account("new_issuer", 0, 0);

		#[extrinsic_call]
		authorize_issuer(RawOrigin::Signed(admin), new_issuer.clone());

		// Verify the result
		assert!(AuthorizedIssuers::<T>::get(&new_issuer));
	}

	#[benchmark]
	fn revoke_issuer() {
		// Setup: Create an admin and an authorized issuer
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		#[extrinsic_call]
		revoke_issuer(RawOrigin::Signed(admin), issuer.clone());

		// Verify the result
		assert!(!AuthorizedIssuers::<T>::get(&issuer));
	}

	#[benchmark]
	fn set_admin() {
		// Setup: Create an initial admin
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let new_admin: T::AccountId = account("new_admin", 0, 0);

		#[extrinsic_call]
		set_admin(RawOrigin::Signed(admin), new_admin.clone());

		// Verify the result
		assert_eq!(Admin::<T>::get(), Some(new_admin));
	}

	impl_benchmark_test_suite!(TravelPoints, crate::mock::new_test_ext(), crate::mock::Test);
}
