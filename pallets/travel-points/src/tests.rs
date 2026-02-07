//! Unit tests for the travel-points pallet.
//!
//! These tests cover all the main functionality:
//! - Awarding points
//! - Spending points with FIFO
//! - Expiration handling
//! - Admin and issuer management
//! - NFT Tickets
//! - Staking

use crate::{mock::*, Error, Event, TicketType, TotalPoints, TravelType, UserPoints};
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

		// Now spend some points (with issuer 2)
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 300, 2));

		// Check balance was updated
		assert_eq!(TotalPoints::<Test>::get(10), 700);

		// Check batch was updated
		let batches = UserPoints::<Test>::get(10);
		assert_eq!(batches[0].remaining_points, 700);

		// Check event
		System::assert_last_event(
			Event::PointsSpent { user: 10, amount_spent: 300, remaining_balance: 700, issuer: 2 }
				.into(),
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
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 600, 2));

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
			TravelPoints::spend_points(RuntimeOrigin::signed(10), 600, 2),
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
			TravelPoints::spend_points(RuntimeOrigin::signed(10), 0, 2),
			Error::<Test>::ZeroAmount
		);
	});
}

/// Test spending with unauthorized issuer fails
#[test]
fn spend_points_unauthorized_issuer_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			500,
			TravelType::Airline,
			None
		));

		// Try to spend with unauthorized issuer (account 5)
		assert_noop!(
			TravelPoints::spend_points(RuntimeOrigin::signed(10), 100, 5),
			Error::<Test>::NotAuthorizedIssuer
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
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 250, 2));

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

// ============================================================================
// NFT TICKET TESTS
// ============================================================================

/// Test minting a ticket NFT
#[test]
fn mint_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// First award some points to the user
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			1000,
			TravelType::Airline,
			None
		));

		// Mint a ticket (costs 500 points)
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2), // issuer
			10,                       // owner
			TicketType::PlaneTicket,
			500,                          // points cost
			Some(2000),                   // expires at
			b"John Doe".to_vec(),         // passenger_name
			b"AB123".to_vec(),            // travel_number
			b"A12".to_vec(),              // gate
			b"15A".to_vec(),              // seat
			b"New York".to_vec(),         // departure
			b"Los Angeles".to_vec(),      // arrival
			b"2024-03-15 10:00".to_vec(), // departure_time
			b"Business Class".to_vec(),   // metadata
		));

		// Check points were deducted
		assert_eq!(TotalPoints::<Test>::get(10), 500);

		// Check ticket was created
		let ticket = TravelPoints::get_ticket(0).expect("Ticket should exist");
		assert_eq!(ticket.owner, 10);
		assert_eq!(ticket.issuer, 2);
		assert_eq!(ticket.ticket_type, TicketType::PlaneTicket);
		assert_eq!(ticket.points_cost, 500);
		assert!(!ticket.is_redeemed);

		// Check user owns the ticket
		let user_tickets = TravelPoints::get_user_tickets(&10);
		assert_eq!(user_tickets.len(), 1);
		assert_eq!(user_tickets[0], 0);
	});
}

/// Test minting a ticket with no point cost (free ticket)
#[test]
fn mint_free_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a free bonus ticket
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::Bonus,
			0,    // free
			None, // no expiration
			b"Jane Doe".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"Lounge Access".to_vec(),
		));

		let ticket = TravelPoints::get_ticket(0).expect("Ticket should exist");
		assert_eq!(ticket.ticket_type, TicketType::Bonus);
		assert_eq!(ticket.points_cost, 0);
	});
}

/// Test redeeming a ticket
#[test]
fn redeem_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::TrainTicket,
			0,
			None,
			b"Test User".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"22B".to_vec(),
			b"Chicago".to_vec(),
			b"Detroit".to_vec(),
			b"2024-04-01 14:00".to_vec(),
			b"".to_vec(),
		));

		// Redeem the ticket
		assert_ok!(TravelPoints::redeem_ticket(RuntimeOrigin::signed(10), 0));

		// Check ticket is redeemed
		let ticket = TravelPoints::get_ticket(0).expect("Ticket should exist");
		assert!(ticket.is_redeemed);

		// Cannot redeem again
		assert_noop!(
			TravelPoints::redeem_ticket(RuntimeOrigin::signed(10), 0),
			Error::<Test>::TicketAlreadyRedeemed
		);
	});
}

/// Test transfer ticket
#[test]
fn transfer_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket for user 10
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
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
		));

		// Transfer to user 20
		assert_ok!(TravelPoints::transfer_ticket(RuntimeOrigin::signed(10), 0, 20));

		// Check new ownership
		let ticket = TravelPoints::get_ticket(0).expect("Ticket should exist");
		assert_eq!(ticket.owner, 20);

		// Check user ticket lists updated
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);
		assert_eq!(TravelPoints::get_user_tickets(&20).len(), 1);
	});
}

/// Test unauthorized issuer cannot mint ticket
#[test]
fn mint_ticket_unauthorized_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::mint_ticket(
				RuntimeOrigin::signed(5), // unauthorized
				10,
				TicketType::PlaneTicket,
				0,
				None,
				b"Test".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
				b"".to_vec(),
			),
			Error::<Test>::NotAuthorizedIssuer
		);
	});
}

// ============================================================================
// STAKING TESTS
// ============================================================================

/// Test basic staking
#[test]
fn stake_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Stake 500 tokens
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 500));

		// Check stake info
		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 500);
		assert_eq!(stake_info.staked_at, 1);
		assert!(!stake_info.is_verifier);

		// Check total staked
		assert_eq!(TravelPoints::total_staked(), 500);

		// Check staker is in list
		let stakers = TravelPoints::get_all_stakers();
		assert!(stakers.contains(&10));
	});
}

/// Test staking below minimum fails
#[test]
fn stake_below_minimum_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Try to stake 50 tokens (below minimum of 100)
		assert_noop!(
			TravelPoints::stake(RuntimeOrigin::signed(10), 50),
			Error::<Test>::StakeBelowMinimum
		);
	});
}

/// Test cannot stake twice
#[test]
fn stake_twice_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 500));

		assert_noop!(
			TravelPoints::stake(RuntimeOrigin::signed(10), 300),
			Error::<Test>::AlreadyStaking
		);
	});
}

/// Test unstaking
#[test]
fn unstake_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// First stake
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 500));
		assert_eq!(TravelPoints::total_staked(), 500);

		// Then unstake
		assert_ok!(TravelPoints::unstake(RuntimeOrigin::signed(10)));

		// Check stake removed
		assert!(TravelPoints::get_stake_info(&10).is_none());
		assert_eq!(TravelPoints::total_staked(), 0);

		// Check removed from staker list
		let stakers = TravelPoints::get_all_stakers();
		assert!(!stakers.contains(&10));
	});
}

/// Test unstaking without stake fails
#[test]
fn unstake_not_staker_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(TravelPoints::unstake(RuntimeOrigin::signed(10)), Error::<Test>::NotStaker);
	});
}

/// Test add to reward pool
#[test]
fn add_to_reward_pool_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::add_to_reward_pool(RuntimeOrigin::signed(10), 1000));
		assert_eq!(TravelPoints::reward_pool(), 1000);

		assert_ok!(TravelPoints::add_to_reward_pool(RuntimeOrigin::signed(20), 500));
		assert_eq!(TravelPoints::reward_pool(), 1500);
	});
}

// ============================================================================
// ISSUER TRACKING TESTS
// ============================================================================

/// Test that spending points tracks issuer spending
#[test]
fn issuer_spending_tracked() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			1000,
			TravelType::Airline,
			None
		));

		// Spend with issuer 2
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 300, 2));

		// Check issuer record
		let period = TravelPoints::current_period();
		let record = TravelPoints::get_issuer_period_record(period, &2);
		assert_eq!(record.points_spent, 300);
		assert_eq!(record.transaction_count, 1);

		// Check period total
		assert_eq!(TravelPoints::get_period_total_spent(period), 300);

		// Spend more
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(10), 200, 2));

		let record = TravelPoints::get_issuer_period_record(period, &2);
		assert_eq!(record.points_spent, 500);
		assert_eq!(record.transaction_count, 2);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - SLASHING
// ============================================================================

/// Test slashing a staker for offline behavior
#[test]
fn slash_staker_offline_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Stake 1000 tokens
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));
		assert_eq!(TravelPoints::total_staked(), 1000);

		// Admin slashes for offline (5% = 50 tokens)
		assert_ok!(TravelPoints::slash_staker(
			RuntimeOrigin::signed(1),
			10,
			crate::SlashReason::Offline
		));

		// Check stake was reduced
		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 950); // 1000 - 50

		// Check total slashed updated
		assert_eq!(TravelPoints::total_slashed(), 50);

		// Check slash record exists
		let records = TravelPoints::get_slash_records(&10);
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].amount, 50);
	});
}

/// Test slashing for invalid verification (10%)
#[test]
fn slash_staker_invalid_verification_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));

		assert_ok!(TravelPoints::slash_staker(
			RuntimeOrigin::signed(1),
			10,
			crate::SlashReason::InvalidVerification
		));

		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 900); // 1000 - 100 (10%)
	});
}

/// Test slashing for malicious behavior (100%)
#[test]
fn slash_staker_malicious_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));

		assert_ok!(TravelPoints::slash_staker(
			RuntimeOrigin::signed(1),
			10,
			crate::SlashReason::Malicious
		));

		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 0); // 1000 - 1000 (100%)
	});
}

/// Test that non-admin cannot slash
#[test]
fn slash_staker_not_admin_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));

		assert_noop!(
			TravelPoints::slash_staker(RuntimeOrigin::signed(5), 10, crate::SlashReason::Offline),
			Error::<Test>::NotAdmin
		);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - UNBONDING
// ============================================================================

/// Test requesting unbonding
#[test]
fn request_unbond_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// First stake
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));

		// Request unbonding of 500
		assert_ok!(TravelPoints::request_unbond(RuntimeOrigin::signed(10), 500));

		// Check stake reduced
		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 500);

		// Check unbonding request created
		let requests = TravelPoints::get_unbonding_requests(&10);
		assert_eq!(requests.len(), 1);
		assert_eq!(requests[0].amount, 500);
		assert_eq!(requests[0].requested_at, 1);
		assert_eq!(requests[0].unlocks_at, 51); // 1 + 50 (unbonding period)
	});
}

/// Test withdrawing unbonded tokens after period ends
#[test]
fn withdraw_unbonded_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));
		assert_ok!(TravelPoints::request_unbond(RuntimeOrigin::signed(10), 500));

		// Cannot withdraw before unbonding period ends
		System::set_block_number(40);
		assert_noop!(
			TravelPoints::withdraw_unbonded(RuntimeOrigin::signed(10)),
			Error::<Test>::UnbondingNotComplete
		);

		// Move past unbonding period
		System::set_block_number(60);

		// Now can withdraw
		assert_ok!(TravelPoints::withdraw_unbonded(RuntimeOrigin::signed(10)));

		// Check unbonding requests cleared
		let requests = TravelPoints::get_unbonding_requests(&10);
		assert_eq!(requests.len(), 0);
	});
}

/// Test cancelling unbonding
#[test]
fn cancel_unbonding_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));
		assert_ok!(TravelPoints::request_unbond(RuntimeOrigin::signed(10), 500));

		// Verify stake reduced
		let stake_before = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_before.amount, 500);

		// Cancel unbonding
		assert_ok!(TravelPoints::cancel_unbonding(RuntimeOrigin::signed(10)));

		// Verify stake restored
		let stake_after = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_after.amount, 1000);

		// Verify requests cleared
		let requests = TravelPoints::get_unbonding_requests(&10);
		assert_eq!(requests.len(), 0);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - DELEGATION AND POOLS
// ============================================================================

/// Test creating a staking pool
#[test]
fn create_pool_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create pool with 1000 stake and 10% commission
		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		// Check pool created
		let pool = TravelPoints::get_pool(0).expect("Pool should exist");
		assert_eq!(pool.operator, 10);
		assert_eq!(pool.total_stake, 1000);
		assert_eq!(pool.operator_stake, 1000);
		assert_eq!(pool.commission, 1000);
		assert!(pool.is_active);
		assert_eq!(pool.delegator_count, 0);

		// Check next pool ID incremented
		assert_eq!(TravelPoints::next_pool_id(), 1);

		// Check total staked updated
		assert_eq!(TravelPoints::total_staked(), 1000);
	});
}

/// Test creating pool with insufficient stake fails
#[test]
fn create_pool_insufficient_stake_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Min pool operator stake is 500 in tests
		assert_noop!(
			TravelPoints::create_pool(RuntimeOrigin::signed(10), 100, 1000),
			Error::<Test>::InsufficientOperatorStake
		);
	});
}

/// Test creating pool with excessive commission fails
#[test]
fn create_pool_excessive_commission_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Max commission is 5000 (50%) in tests
		assert_noop!(
			TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 6000),
			Error::<Test>::CommissionTooHigh
		);
	});
}

/// Test delegating to a pool
#[test]
fn delegate_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create pool first
		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		// Delegate to pool
		assert_ok!(TravelPoints::delegate(RuntimeOrigin::signed(20), 0, 500));

		// Check delegation recorded
		let delegation = TravelPoints::get_delegation(&20).expect("Delegation should exist");
		assert_eq!(delegation.pool_id, 0);
		assert_eq!(delegation.amount, 500);

		// Check pool updated
		let pool = TravelPoints::get_pool(0).expect("Pool should exist");
		assert_eq!(pool.total_stake, 1500);
		assert_eq!(pool.delegator_count, 1);

		// Check delegator list
		let delegators = TravelPoints::get_pool_delegators(0);
		assert!(delegators.contains(&20));
	});
}

/// Test cannot delegate below minimum
#[test]
fn delegate_below_minimum_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		// Min stake is 100 in tests
		assert_noop!(
			TravelPoints::delegate(RuntimeOrigin::signed(20), 0, 50),
			Error::<Test>::DelegationBelowMinimum
		);
	});
}

/// Test undelegating from a pool
#[test]
fn undelegate_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));
		assert_ok!(TravelPoints::delegate(RuntimeOrigin::signed(20), 0, 500));

		// Undelegate
		assert_ok!(TravelPoints::undelegate(RuntimeOrigin::signed(20)));

		// Check delegation removed
		assert!(TravelPoints::get_delegation(&20).is_none());

		// Check pool updated
		let pool = TravelPoints::get_pool(0).expect("Pool should exist");
		assert_eq!(pool.total_stake, 1000);
		assert_eq!(pool.delegator_count, 0);
	});
}

/// Test updating pool commission
#[test]
fn set_pool_commission_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		// Update commission
		assert_ok!(TravelPoints::set_pool_commission(RuntimeOrigin::signed(10), 0, 2000));

		let pool = TravelPoints::get_pool(0).expect("Pool should exist");
		assert_eq!(pool.commission, 2000);
	});
}

/// Test non-operator cannot update commission
#[test]
fn set_pool_commission_not_operator_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		assert_noop!(
			TravelPoints::set_pool_commission(RuntimeOrigin::signed(20), 0, 2000),
			Error::<Test>::NotPoolOperator
		);
	});
}

/// Test closing a pool
#[test]
fn close_pool_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));

		// Close pool
		assert_ok!(TravelPoints::close_pool(RuntimeOrigin::signed(10), 0));

		// Check pool removed
		assert!(TravelPoints::get_pool(0).is_none());

		// Check total staked reduced
		assert_eq!(TravelPoints::total_staked(), 0);
	});
}

/// Test cannot close pool with delegators
#[test]
fn close_pool_with_delegators_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::create_pool(RuntimeOrigin::signed(10), 1000, 1000));
		assert_ok!(TravelPoints::delegate(RuntimeOrigin::signed(20), 0, 500));

		assert_noop!(
			TravelPoints::close_pool(RuntimeOrigin::signed(10), 0),
			Error::<Test>::PoolHasDelegators
		);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - ERA ROTATION AND VERIFIERS
// ============================================================================

/// Test era rotation and verifier selection
#[test]
fn rotate_era_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create multiple stakers with different stakes
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(20), 2000));
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(30), 500));

		// Move past blocks per era (200 in tests)
		System::set_block_number(201);

		// Rotate era
		assert_ok!(TravelPoints::rotate_era(RuntimeOrigin::signed(99)));

		// Check era incremented
		assert_eq!(TravelPoints::current_era(), 1);

		// Check verifiers selected (should select by highest stake)
		let verifiers = TravelPoints::get_current_verifiers();
		assert!(!verifiers.is_empty());

		// Account 20 should be a verifier (highest stake)
		assert!(TravelPoints::is_current_verifier(&20));
	});
}

/// Test era rotation not due yet
#[test]
fn rotate_era_not_due_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100); // Less than 200 blocks per era

		assert_noop!(
			TravelPoints::rotate_era(RuntimeOrigin::signed(99)),
			Error::<Test>::EraRotationNotDue
		);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - REWARDS
// ============================================================================

/// Test distributing rewards
#[test]
fn distribute_rewards_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Setup: Add staker and issuer spending
		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));

		// Add to reward pool
		assert_ok!(TravelPoints::add_to_reward_pool(RuntimeOrigin::signed(99), 10000));

		// Award and spend points to create issuer tracking
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			30,
			1000,
			crate::TravelType::Airline,
			None
		));
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(30), 500, 2));

		let period = TravelPoints::current_period();

		// Distribute rewards
		assert_ok!(TravelPoints::distribute_rewards(RuntimeOrigin::signed(1), period));

		// Check reward pool emptied
		assert_eq!(TravelPoints::reward_pool(), 0);

		// Check pending rewards created (staker gets 80%, issuer gets 20%)
		let staker_rewards = TravelPoints::pending_staker_rewards(&10);
		assert!(staker_rewards > 0);
	});
}

/// Test claiming rewards
#[test]
fn claim_rewards_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 1000));
		assert_ok!(TravelPoints::add_to_reward_pool(RuntimeOrigin::signed(99), 10000));

		// Award and spend points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			30,
			1000,
			crate::TravelType::Airline,
			None
		));
		assert_ok!(TravelPoints::spend_points(RuntimeOrigin::signed(30), 500, 2));

		let period = TravelPoints::current_period();
		assert_ok!(TravelPoints::distribute_rewards(RuntimeOrigin::signed(1), period));

		// Claim rewards
		assert_ok!(TravelPoints::claim_rewards(RuntimeOrigin::signed(10)));

		// Check pending rewards cleared
		assert_eq!(TravelPoints::pending_staker_rewards(&10), 0);
	});
}

/// Test claim rewards with no pending fails
#[test]
fn claim_rewards_none_pending_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::claim_rewards(RuntimeOrigin::signed(10)),
			Error::<Test>::NoRewardsToClaim
		);
	});
}

// ============================================================================
// ADVANCED STAKING TESTS - INCREASE STAKE
// ============================================================================

/// Test increasing stake
#[test]
fn increase_stake_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TravelPoints::stake(RuntimeOrigin::signed(10), 500));
		assert_eq!(TravelPoints::total_staked(), 500);

		// Increase stake
		assert_ok!(TravelPoints::increase_stake(RuntimeOrigin::signed(10), 300));

		let stake_info = TravelPoints::get_stake_info(&10).expect("Stake should exist");
		assert_eq!(stake_info.amount, 800);
		assert_eq!(TravelPoints::total_staked(), 800);
	});
}

/// Test increasing stake without existing stake fails
#[test]
fn increase_stake_not_staker_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::increase_stake(RuntimeOrigin::signed(10), 300),
			Error::<Test>::NotStaker
		);
	});
}

// ============================================================================
// TICKET UNMINT (BURN) TESTS
// ============================================================================

/// Test unminting a ticket by owner
#[test]
fn unmint_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket for user 10
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			0,
			None,
			b"Test User".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"Business Class".to_vec(),
		));

		// Verify ticket exists
		assert!(TravelPoints::get_ticket(0).is_some());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 1);

		// Unmint the ticket
		assert_ok!(TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0));

		// Verify ticket was removed
		assert!(TravelPoints::get_ticket(0).is_none());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);

		// Check event
		System::assert_last_event(Event::TicketUnminted { ticket_id: 0, owner: 10 }.into());
	});
}

/// Test unminting a redeemed ticket works
#[test]
fn unmint_redeemed_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint and redeem a ticket
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::TrainTicket,
			0,
			None,
			b"Test User".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"22B".to_vec(),
			b"Chicago".to_vec(),
			b"Detroit".to_vec(),
			b"2024-04-01 14:00".to_vec(),
			b"".to_vec(),
		));

		assert_ok!(TravelPoints::redeem_ticket(RuntimeOrigin::signed(10), 0));

		// Can still unmint the redeemed ticket
		assert_ok!(TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0));

		assert!(TravelPoints::get_ticket(0).is_none());
	});
}

/// Test unminting non-existent ticket fails
#[test]
fn unmint_ticket_not_found_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 999),
			Error::<Test>::TicketNotFound
		);
	});
}

/// Test unminting ticket by non-owner fails
#[test]
fn unmint_ticket_not_owner_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket for user 10
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::BusTicket,
			0,
			None,
			b"Test".to_vec(),
			b"BUS001".to_vec(),
			b"".to_vec(),
			b"5".to_vec(),
			b"City A".to_vec(),
			b"City B".to_vec(),
			b"2024-05-01 09:00".to_vec(),
			b"".to_vec(),
		));

		// User 20 tries to unmint (not owner)
		assert_noop!(
			TravelPoints::unmint_ticket(RuntimeOrigin::signed(20), 0),
			Error::<Test>::NotTicketOwner
		);
	});
}

/// Test double unmint fails
#[test]
fn unmint_ticket_double_unmint_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::Bonus,
			0,
			None,
			b"Test".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"Lounge Access".to_vec(),
		));

		// First unmint succeeds
		assert_ok!(TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0));

		// Second unmint fails (ticket already removed)
		assert_noop!(
			TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0),
			Error::<Test>::TicketNotFound
		);
	});
}

// ============================================================================
// FORCE UNMINT (ADMIN) TESTS
// ============================================================================

/// Test force unminting by admin
#[test]
fn force_unmint_ticket_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket for user 10
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			0,
			None,
			b"Test User".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"".to_vec(),
		));

		// Admin (account 1) force unmints the ticket
		assert_ok!(TravelPoints::force_unmint_ticket(RuntimeOrigin::signed(1), 0));

		// Verify ticket was removed
		assert!(TravelPoints::get_ticket(0).is_none());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);

		// Check event
		System::assert_last_event(
			Event::TicketForceUnminted { ticket_id: 0, owner: 10, admin: 1 }.into(),
		);
	});
}

/// Test force unminting by non-admin fails
#[test]
fn force_unmint_ticket_not_admin_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::TrainTicket,
			0,
			None,
			b"Test".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"22B".to_vec(),
			b"Chicago".to_vec(),
			b"Detroit".to_vec(),
			b"2024-04-01 14:00".to_vec(),
			b"".to_vec(),
		));

		// Non-admin (account 5) tries to force unmint
		assert_noop!(
			TravelPoints::force_unmint_ticket(RuntimeOrigin::signed(5), 0),
			Error::<Test>::NotAdmin
		);
	});
}

/// Test force unminting non-existent ticket fails
#[test]
fn force_unmint_ticket_not_found_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			TravelPoints::force_unmint_ticket(RuntimeOrigin::signed(1), 999),
			Error::<Test>::TicketNotFound
		);
	});
}

// ============================================================================
// EXPIRED TICKET CLEANUP TESTS
// ============================================================================

/// Test cleanup of expired tickets
#[test]
fn cleanup_expired_tickets_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket that expires at block 100
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			0,
			Some(100), // Expires at block 100
			b"Test User".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"".to_vec(),
		));

		// Verify ticket exists
		assert!(TravelPoints::get_ticket(0).is_some());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 1);

		// Move past expiration
		System::set_block_number(150);

		// Cleanup expired tickets
		assert_ok!(TravelPoints::cleanup_expired_tickets(RuntimeOrigin::signed(99), 10));

		// Verify ticket was removed
		assert!(TravelPoints::get_ticket(0).is_none());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);

		// Check event
		System::assert_last_event(
			Event::ExpiredTicketsCleaned { user: 10, tickets_removed: 1 }.into(),
		);
	});
}

/// Test cleanup with no expired tickets does nothing
#[test]
fn cleanup_expired_tickets_no_expired() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket that expires at block 1000
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::TrainTicket,
			0,
			Some(1000), // Expires at block 1000
			b"Test".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"22B".to_vec(),
			b"Chicago".to_vec(),
			b"Detroit".to_vec(),
			b"2024-04-01 14:00".to_vec(),
			b"".to_vec(),
		));

		// Still before expiration
		System::set_block_number(500);

		// Cleanup - but nothing to clean
		assert_ok!(TravelPoints::cleanup_expired_tickets(RuntimeOrigin::signed(99), 10));

		// Ticket should still exist
		assert!(TravelPoints::get_ticket(0).is_some());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 1);
	});
}

/// Test cleanup with tickets that have no expiration
#[test]
fn cleanup_expired_tickets_no_expiration_date() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint a ticket with no expiration
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::Bonus,
			0,
			None, // No expiration
			b"Test".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"Lounge Access".to_vec(),
		));

		// Move far into the future
		System::set_block_number(1000000);

		// Cleanup - should not remove ticket without expiration
		assert_ok!(TravelPoints::cleanup_expired_tickets(RuntimeOrigin::signed(99), 10));

		// Ticket should still exist
		assert!(TravelPoints::get_ticket(0).is_some());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 1);
	});
}

/// Test cleanup removes only expired tickets (partial cleanup)
#[test]
fn cleanup_expired_tickets_partial() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint ticket that expires at block 50
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			0,
			Some(50), // Expires at block 50
			b"Early Ticket".to_vec(),
			b"AB123".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
		));

		// Mint ticket that expires at block 200
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::TrainTicket,
			0,
			Some(200), // Expires at block 200
			b"Late Ticket".to_vec(),
			b"TR456".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
		));

		// Mint ticket with no expiration
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::Bonus,
			0,
			None, // No expiration
			b"Bonus".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
			b"".to_vec(),
		));

		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 3);

		// Move to block 100 (only first ticket expired)
		System::set_block_number(100);

		// Cleanup
		assert_ok!(TravelPoints::cleanup_expired_tickets(RuntimeOrigin::signed(99), 10));

		// Only 2 tickets should remain
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 2);
		assert!(TravelPoints::get_ticket(0).is_none()); // First ticket removed
		assert!(TravelPoints::get_ticket(1).is_some()); // Second ticket still exists
		assert!(TravelPoints::get_ticket(2).is_some()); // Bonus ticket still exists
	});
}

/// Test cleanup for user with no tickets
#[test]
fn cleanup_expired_tickets_no_tickets() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Cleanup for user with no tickets - should succeed without error
		assert_ok!(TravelPoints::cleanup_expired_tickets(RuntimeOrigin::signed(99), 10));

		// No event should be emitted (no tickets cleaned)
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);
	});
}

// ============================================================================
// COMPLETE TICKET LIFECYCLE TESTS
// ============================================================================

/// Test full ticket lifecycle: mint -> transfer -> unmint
#[test]
fn ticket_lifecycle_mint_transfer_unmint() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Mint ticket for user 10
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			0,
			None,
			b"Original Owner".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"".to_vec(),
		));

		// Transfer to user 20
		assert_ok!(TravelPoints::transfer_ticket(RuntimeOrigin::signed(10), 0, 20));

		// User 10 cannot unmint (no longer owner)
		assert_noop!(
			TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0),
			Error::<Test>::NotTicketOwner
		);

		// User 20 can unmint
		assert_ok!(TravelPoints::unmint_ticket(RuntimeOrigin::signed(20), 0));

		// Ticket is gone
		assert!(TravelPoints::get_ticket(0).is_none());
		assert_eq!(TravelPoints::get_user_tickets(&10).len(), 0);
		assert_eq!(TravelPoints::get_user_tickets(&20).len(), 0);
	});
}

/// Test ticket lifecycle with points: mint with cost -> unmint (no refund)
#[test]
fn ticket_lifecycle_mint_with_points_unmint() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Award points
		assert_ok!(TravelPoints::award_points(
			RuntimeOrigin::signed(2),
			10,
			1000,
			TravelType::Airline,
			None
		));

		// Mint ticket with points cost
		assert_ok!(TravelPoints::mint_ticket(
			RuntimeOrigin::signed(2),
			10,
			TicketType::PlaneTicket,
			500, // Points cost
			None,
			b"Test User".to_vec(),
			b"AB123".to_vec(),
			b"A12".to_vec(),
			b"15A".to_vec(),
			b"New York".to_vec(),
			b"Los Angeles".to_vec(),
			b"2024-03-15 10:00".to_vec(),
			b"".to_vec(),
		));

		// Points were deducted
		assert_eq!(TotalPoints::<Test>::get(10), 500);

		// Unmint the ticket
		assert_ok!(TravelPoints::unmint_ticket(RuntimeOrigin::signed(10), 0));

		// Points are NOT refunded (unmint doesn't restore points)
		assert_eq!(TotalPoints::<Test>::get(10), 500);

		// Ticket is gone
		assert!(TravelPoints::get_ticket(0).is_none());
	});
}
