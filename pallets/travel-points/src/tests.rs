//! Unit tests for the travel-points pallet.
//!
//! These tests cover all the main functionality:
//! - Awarding points
//! - Spending points with FIFO
//! - Expiration handling
//! - Admin and issuer management

use crate::{mock::*, Error, Event, TotalPoints, TravelType, UserPoints};
use frame_support::{assert_noop, assert_ok};

// ============================================================================
// AWARDING POINTS TESTS
// ============================================================================

/// Test that an authorized issuer can award points successfully
#[test]
fn award_points_works() {
	new_test_ext().execute_with(|| {
		// Set block number so events are deposited
		System::set_block_number(1);

		// Account 2 is pre-authorized, award 1000 points to account 10
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,   // recipient
			1000, // amount
			TravelType::Airline,
			None // use default expiration
		));

		// Check that points were recorded
		assert_eq!(TotalPoints::<Test>::get(10), 1000);

		// Check that a batch was created
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches.len(), 1);
		assert_eq!(batches[0].remaining_points, 1000);
		assert_eq!(batches[0].earned_at_block, 1);
		assert_eq!(batches[0].expires_at_block, 1001); // 1 + 1000 (default expiration)
		assert_eq!(batches[0].travel_type, TravelType::Airline);

		// Check that the event was emitted
		System::assert_last_event(
			Event::PointsEarned {
				recipient: 10,
				amount: 1000,
				expires_at_block: 1001,
				travel_type: TravelType::Airline,
			}
			.into(),
		);
	});
}

/// Test that unauthorized accounts cannot award points
#[test]
fn award_points_unauthorized_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Account 5 is not authorized
		assert_noop!(
			TravelPoints::award_points(RuntimeOrigin::signed(5), 10, 1000, TravelType::Train, None),
			Error::<Test>::NotAuthorizedIssuer
		);
	});
}

/// Test that zero amount fails
#[test]
fn award_points_zero_amount_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::award_points(
				RuntimeOrigin::signed(2), // authorized
				10,
				0, // zero amount
				TravelType::Bus,
				None
			),
			Error::<Test>::ZeroAmount
		);
	});
}

/// Test custom expiration period
#[test]
fn award_points_custom_expiration_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(10);

		// Award with custom expiration of 500 blocks
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Other,
			Some(500) // custom expiration
		));

		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches[0].expires_at_block, 510); // 10 + 500
	});
}

// ============================================================================
// SPENDING POINTS TESTS
// ============================================================================

/// Test basic point spending
#[test]
fn spend_points_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// First award some points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			1000,
			TravelType::Airline,
			None
		));

		// Now spend some points
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 300));

		// Check balance was updated
		assert_eq!(TotalPoints::<Test>::get(10), 700);

		// Check batch was updated
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches[0].remaining_points, 700);

		// Check event
		System::assert_last_event(
			Event::PointsSpent { user: 10, amount_spent: 300, remaining_balance: 700 }.into(),
		);
	});
}

/// Test that FIFO works - oldest points are spent first
#[test]
fn spend_points_fifo_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award first batch - will expire at block 1001
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			None
		));

		System::set_block_number(2);

		// Award second batch - will expire at block 1002
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Train,
			None
		));

		// Total is 1000
		assert_eq!(TotalPoints::<Test>::get(10), 1000);

		// Spend 600 points - should take all 500 from first batch and 100 from second
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 600));

		let batches = UserPoints::<Test>::get(10);
		// First batch should be removed (empty)
		assert_eq!(batches.len(), 1);
		// Second batch should have 400 remaining
		assert_eq!(batches[0].remaining_points, 400);
		assert_eq!(batches[0].travel_type, TravelType::Train);
	});
}

/// Test spending more than available fails
#[test]
fn spend_points_insufficient_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award 500 points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			None
		));

		// Try to spend 600
		assert_noop!(
			TravelPoints::spend_points(RuntimeOrigin::signed(10), 600),
			Error::<Test>::InsufficientPoints
		);
	});
}

/// Test spending zero fails
#[test]
fn spend_points_zero_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			None
		));

		assert_noop!(
			TravelPoints::spend_points(RuntimeOrigin::signed(10), 0),
			Error::<Test>::ZeroAmount
		);
	});
}

// ============================================================================
// EXPIRATION TESTS
// ============================================================================

/// Test that expired points are not counted
#[test]
fn expired_points_not_available() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award 500 points with short expiration (100 blocks)
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			Some(100)
		));

		// Move to block 200 (past expiration at block 101)
		System::set_block_number(200);

		// Award some more points (this triggers cleanup)
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			100,
			TravelType::Train,
			None
		));

		// Should only have 100 points (the new ones, old ones expired)
		// The cleanup happens during award_points
		// Note: TotalPoints might still show old value until cleanup
		assert_eq!(TravelPoints::get_available_points(&10), 100);
	});
}

/// Test cleanup_expired function
#[test]
fn cleanup_expired_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award points that will expire at block 101
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Bus,
			Some(100)
		));

		// Move past expiration
		System::set_block_number(150);

		// Call cleanup
		assert_ok!(TravelPoints::cleanup_expired(RuntimeOrigin::signed(99), 10));

		// Batches should be empty
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches.len(), 0);

		// Total should be 0
		assert_eq!(TotalPoints::<Test>::get(10), 0);
	});
}

// ============================================================================
// ADMIN AND ISSUER MANAGEMENT TESTS
// ============================================================================

/// Test authorizing a new issuer
#[test]
fn authorize_issuer_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Account 1 is admin, authorize account 5
		assert_ok!(TravelPoints::authorize_issuer(RuntimeOrigin::signed(1), 5));

		// Account 5 should now be able to issue points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(5),
			10,
			100,
			TravelType::Other,
			None
		));

		// Check event
		System::assert_has_event(Event::IssuerAuthorized { issuer: 5 }.into());
	});
}

/// Test that non-admin cannot authorize issuers
#[test]
fn authorize_issuer_not_admin_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Account 5 is not admin
		assert_noop!(
			TravelPoints::authorize_issuer(RuntimeOrigin::signed(5), 10),
			Error::<Test>::NotAdmin
		);
	});
}

/// Test revoking an issuer
#[test]
fn revoke_issuer_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Account 2 is pre-authorized, revoke them
		assert_ok!(TravelPoints::revoke_issuer(RuntimeOrigin::signed(1), 2));

		// Account 2 should no longer be able to issue points
		assert_noop!(
			TravelPoints::award_points(
				RuntimeOrigin::signed(2),
				10,
				100,
				TravelType::Airline,
				None
			),
			Error::<Test>::NotAuthorizedIssuer
		);
	});
}

/// Test changing admin
#[test]
fn set_admin_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Account 1 is admin, set account 5 as new admin
		assert_ok!(TravelPoints::set_admin(RuntimeOrigin::signed(1), 5));

		// Account 1 should no longer be admin
		assert_noop!(
			TravelPoints::authorize_issuer(RuntimeOrigin::signed(1), 10),
			Error::<Test>::NotAdmin
		);

		// Account 5 should be admin now
		assert_ok!(TravelPoints::authorize_issuer(RuntimeOrigin::signed(5), 10));
	});
}

// ============================================================================
// MULTIPLE BATCHES AND COMPLEX SCENARIOS
// ============================================================================

/// Test having multiple batches with different travel types
#[test]
fn multiple_travel_types_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award airline points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			100,
			TravelType::Airline,
			Some(500)
		));

		System::set_block_number(2);

		// Award train points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			200,
			TravelType::Train,
			Some(600)
		));

		System::set_block_number(3);

		// Award bus points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			150,
			TravelType::Bus,
			Some(700)
		));

		// Check total
		assert_eq!(TotalPoints::<Test>::get(10), 450);

		// Check batches are sorted by expiration (FIFO order)
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches.len(), 3);
		assert_eq!(batches[0].travel_type, TravelType::Airline); // expires first
		assert_eq!(batches[1].travel_type, TravelType::Train);
		assert_eq!(batches[2].travel_type, TravelType::Bus); // expires last
	});
}

/// Test spending across multiple batches completely empties some
#[test]
fn spend_across_batches_removes_empty() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award 3 batches of 100 each
		for i in 0..3 {
			System::set_block_number(1 + i);
			assert_ok!(TravelPoints::award_points(
				RuntimeOrigin::signed(2),
				10,
				100,
				TravelType::Airline,
				None
			));
		}

		assert_eq!(TotalPoints::<Test>::get(10), 300);
		assert_eq!(UserPoints::<Test>::get(10).len(), 3);

		// Spend 250 - should empty first 2 batches and take 50 from third
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 250));

		// Only 1 batch left with 50 points
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches.len(), 1);
		assert_eq!(batches[0].remaining_points, 50);
		assert_eq!(TotalPoints::<Test>::get(10), 50);
	});
}

/// Test the helper function for checking available points
#[test]
fn get_available_points_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			Some(100)
		));

		// Before expiration
		assert_eq!(TravelPoints::get_available_points(&10), 500);

		// After expiration
		System::set_block_number(150);
		assert_eq!(TravelPoints::get_available_points(&10), 0);
	});
}

/// Test the helper function for point details
#[test]
fn get_point_details_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			Some(100)
		));

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			300,
			TravelType::Train,
			Some(200)
		));

		let details = TravelPoints::get_point_details(&10);
		assert_eq!(details.len(), 2);
		assert_eq!(details[0], (500, 101, TravelType::Airline));
		assert_eq!(details[1], (300, 201, TravelType::Train));
	});
}

// ============================================================================
// CONTRACT INTERFACE TESTS
// ============================================================================

/// Test the contract interface for awarding points
#[test]
fn contract_award_points_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Use the contract interface
		assert_ok!(TravelPoints::contract_award_points(
			2,   // issuer (pre-authorized)
			10,  // recipient
			500, // amount
			TravelType::Airline,
			None
		));

		assert_eq!(TotalPoints::<Test>::get(10), 500);
	});
}

/// Test the contract balance check interface
#[test]
fn contract_check_balance_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			1000,
			TravelType::Airline,
			None
		));

		// Use contract interface to check balance
		assert_eq!(TravelPoints::contract_check_balance(&10), 1000);
	});
}

/// Test the contract issuer check interface
#[test]
fn contract_is_authorized_issuer_works() {
	new_test_ext().execute_with(|| {
		// Account 2 is pre-authorized
		assert!(TravelPoints::contract_is_authorized_issuer(&2));

		// Account 5 is not authorized
		assert!(!TravelPoints::contract_is_authorized_issuer(&5));
	});
}
