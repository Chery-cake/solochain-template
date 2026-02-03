//! Mock runtime for testing the travel-points pallet.
//!
//! This module sets up a minimal runtime environment for unit testing
//! the travel points functionality.

use crate as pallet_travel_points;
use frame_support::derive_impl;
use sp_runtime::BuildStorage;

// Define the mock block type using the standard testing utilities
type Block = frame_system::mocking::MockBlock<Test>;

// Build the mock runtime with the necessary pallets
#[frame_support::runtime]
mod runtime {
	// The main runtime struct
	#[runtime::runtime]
	// Generate all the necessary runtime types
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Test;

	// System pallet - required by all runtimes
	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Test>;

	// Our travel points pallet
	#[runtime::pallet_index(1)]
	pub type TravelPoints = pallet_travel_points::Pallet<Test>;
}

// Configure the system pallet for the test runtime
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

// Configure our travel points pallet for testing
impl pallet_travel_points::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	// Allow up to 100 point batches per user in tests
	type MaxPointBatches = frame_support::traits::ConstU32<100>;
	// Default expiration: 1000 blocks (about 100 minutes with 6 second blocks)
	// Note: Uses ConstU64 because TestDefaultConfig uses u64 for BlockNumber,
	// while the real runtime uses ConstU32 (runtime BlockNumber is u32)
	type DefaultExpirationPeriod = frame_support::traits::ConstU64<1000>;
	// Maximum 50 tickets per user in tests
	type MaxTicketsPerUser = frame_support::traits::ConstU32<50>;
	// Maximum 100 stakers in tests
	type MaxStakers = frame_support::traits::ConstU32<100>;
	// Minimum stake amount: 100 tokens
	type MinStakeAmount = frame_support::traits::ConstU128<100>;
	// Staker reward percentage: 30% (3000 basis points)
	type StakerRewardPercent = frame_support::traits::ConstU32<3000>;
	// Blocks per reward period: 100 blocks (about 10 minutes with 6 second blocks)
	type BlocksPerRewardPeriod = frame_support::traits::ConstU64<100>;

	// ============================================================================
	// ADVANCED STAKING CONFIGURATION
	// ============================================================================

	// Unbonding period: 50 blocks (~5 minutes in test)
	type UnbondingPeriod = frame_support::traits::ConstU64<50>;
	// Offline slash: 5% (500 basis points)
	type OfflineSlashPercent = frame_support::traits::ConstU32<500>;
	// Invalid verification slash: 10% (1000 basis points)
	type InvalidVerificationSlashPercent = frame_support::traits::ConstU32<1000>;
	// Malicious slash: 100% (10000 basis points)
	type MaliciousSlashPercent = frame_support::traits::ConstU32<10000>;
	// Maximum 50 pools in tests
	type MaxPools = frame_support::traits::ConstU32<50>;
	// Maximum 20 delegators per pool in tests
	type MaxDelegatorsPerPool = frame_support::traits::ConstU32<20>;
	// Minimum pool operator stake: 500 tokens
	type MinPoolOperatorStake = frame_support::traits::ConstU128<500>;
	// Maximum pool commission: 50% (5000 basis points)
	type MaxPoolCommission = frame_support::traits::ConstU32<5000>;
	// 5 verifiers selected per era in tests
	type VerifiersPerEra = frame_support::traits::ConstU32<5>;
	// Blocks per era: 200 blocks (~20 minutes in test)
	type BlocksPerEra = frame_support::traits::ConstU64<200>;
	// Issuer reward percentage: 20% (2000 basis points)
	type IssuerRewardPercent = frame_support::traits::ConstU32<2000>;
	// Maximum 10 unbonding requests per account
	type MaxUnbondingRequests = frame_support::traits::ConstU32<10>;
}

// Helper function to build the genesis storage for tests
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	// Configure the travel points pallet with an admin
	pallet_travel_points::GenesisConfig::<Test> {
		admin: Some(1),              // Account 1 is the admin
		authorized_issuers: vec![2], // Account 2 is pre-authorized to issue points
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	storage.into()
}
