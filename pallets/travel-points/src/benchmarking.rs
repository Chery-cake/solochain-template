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
			RawOrigin::Signed(issuer.clone()).into(),
			user.clone(),
			2000,
			TravelType::Airline,
			None,
		);

		let spend_amount: u128 = 500;

		#[extrinsic_call]
		spend_points(RawOrigin::Signed(user.clone()), spend_amount, issuer.clone());

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

	#[benchmark]
	fn mint_ticket() {
		// Setup: Create an admin and authorized issuer
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let owner: T::AccountId = account("owner", 0, 0);

		// Award points to owner first (for points_cost)
		let _ = TravelPoints::<T>::award_points(
			RawOrigin::Signed(issuer.clone()).into(),
			owner.clone(),
			2000,
			TravelType::Airline,
			None,
		);

		let points_cost: u128 = 500;

		#[extrinsic_call]
		mint_ticket(
			RawOrigin::Signed(issuer.clone()),
			owner.clone(),
			TicketType::PlaneTicket,
			points_cost,
			None,
			b"John Doe".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"Business Class".to_vec(),
		);

		// Verify the result - ticket was created
		assert_eq!(NextTicketId::<T>::get(), 1);
		// Points were deducted
		assert_eq!(TotalPoints::<T>::get(&owner), 1500);
	}

	#[benchmark]
	fn redeem_ticket() {
		// Setup: Create a ticket first
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let owner: T::AccountId = account("owner", 0, 0);

		// Mint a free ticket
		let _ = TravelPoints::<T>::mint_ticket(
			RawOrigin::Signed(issuer).into(),
			owner.clone(),
			TicketType::TrainTicket,
			0, // free ticket
			None,
			b"Test User".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"22B".to_vec(),
			b"Chicago".to_vec(),
			b"Detroit".to_vec(),
			b"2024-04-01 14:00".to_vec(),
			b"".to_vec(),
		);

		let ticket_id = 0u128;

		#[extrinsic_call]
		redeem_ticket(RawOrigin::Signed(owner), ticket_id);

		// Verify the ticket is redeemed
		let ticket = Tickets::<T>::get(ticket_id).unwrap();
		assert!(ticket.is_redeemed);
	}

	#[benchmark]
	fn transfer_ticket() {
		// Setup: Create a ticket first
		let admin: T::AccountId = whitelisted_caller();
		Admin::<T>::put(&admin);

		let issuer: T::AccountId = account("issuer", 0, 0);
		AuthorizedIssuers::<T>::insert(&issuer, true);

		let from: T::AccountId = account("from", 0, 0);
		let to: T::AccountId = account("to", 0, 0);

		// Mint a ticket for 'from' account
		let _ = TravelPoints::<T>::mint_ticket(
			RawOrigin::Signed(issuer).into(),
			from.clone(),
			TicketType::BusTicket,
			0,
			None,
			b"Original Owner".to_vec(),
			b"BUS001".to_vec(),
			b"".to_vec(),
			b"5".to_vec(),
			b"City A".to_vec(),
			b"City B".to_vec(),
			b"2024-05-01 09:00".to_vec(),
			b"".to_vec(),
		);

		let ticket_id = 0u128;

		#[extrinsic_call]
		transfer_ticket(RawOrigin::Signed(from.clone()), ticket_id, to.clone());

		// Verify the ticket ownership changed
		let ticket = Tickets::<T>::get(ticket_id).unwrap();
		assert_eq!(ticket.owner, to);
	}

	#[benchmark]
	fn stake() {
		let staker: T::AccountId = whitelisted_caller();
		let amount: u128 = 1000; // Above minimum

		#[extrinsic_call]
		stake(RawOrigin::Signed(staker.clone()), amount);

		// Verify stake was created
		assert!(Stakes::<T>::get(&staker).is_some());
		assert_eq!(TotalStaked::<T>::get(), amount);
	}

	#[benchmark]
	fn unstake() {
		// Setup: First create a stake
		let staker: T::AccountId = whitelisted_caller();
		let amount: u128 = 1000;

		let _ = TravelPoints::<T>::stake(RawOrigin::Signed(staker.clone()).into(), amount);

		#[extrinsic_call]
		unstake(RawOrigin::Signed(staker.clone()));

		// Verify stake was removed
		assert!(Stakes::<T>::get(&staker).is_none());
		assert_eq!(TotalStaked::<T>::get(), 0);
	}

	#[benchmark]
	fn add_to_reward_pool() {
		let contributor: T::AccountId = whitelisted_caller();
		let amount: u128 = 5000;

		#[extrinsic_call]
		add_to_reward_pool(RawOrigin::Signed(contributor), amount);

		// Verify the pool was updated
		assert_eq!(RewardPool::<T>::get(), amount);
	}

	impl_benchmark_test_suite!(TravelPoints, crate::mock::new_test_ext(), crate::mock::Test);
}
