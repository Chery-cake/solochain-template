//! # Travel Points Pallet
//!
//! A pallet for managing travel loyalty points similar to airline mileage programs.
//! This pallet allows users to earn, spend, and track travel points with expiration dates.
//!
//! ## Overview
//!
//! The Travel Points pallet provides functionality for:
//! - **Earning points**: Points can be awarded to users with a specific expiration period
//! - **Spending points**: Points are deducted using FIFO (First In, First Out) - oldest points are used first
//! - **Expiration tracking**: Each point batch tracks when it was earned and when it expires
//! - **Smart contract interface**: Authorized issuers (like smart contracts) can award points
//! - **Multi-travel support**: Designed to support various travel types (planes, trains, buses)
//! - **NFT Tickets**: Store travel tickets and bonuses as NFTs with detailed metadata
//! - **Staking System**: Verifiers/stakers can stake tokens to earn rewards
//! - **Issuer Rewards**: Issuers earn rewards based on point spending proportions
//!
//! ## Key Concepts
//!
//! ### Point Batches
//! Points are stored in "batches" - each time a user earns points, a new batch is created with:
//! - The block number when points were earned
//! - The block number when points will expire
//! - The remaining amount of points in this batch
//!
//! ### FIFO Deduction
//! When spending points, the system automatically uses the oldest (earliest expiring) points first.
//! This ensures users don't lose points due to expiration when they have newer points available.
//!
//! ### Authorized Issuers
//! Only authorized accounts (which could be smart contracts) can issue points.
//! This allows integration with travel booking systems and loyalty programs.
//!
//! ### NFT Tickets
//! Tickets and bonuses can be purchased with points and/or money. Each ticket is an NFT
//! containing detailed metadata (e.g., passenger name, flight number, gate for plane tickets).
//!
//! ### Staking and Rewards
//! - Verifiers/stakers stake tokens and receive rewards based on contribution
//! - Issuers receive rewards proportional to how much users spend points with them
//! - Daily tracking of point spending per issuer for fair reward distribution

#![cfg_attr(not(feature = "std"), no_std)]

// Import alloc for no_std Vec support
extern crate alloc;
use alloc::vec::Vec;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// Mock runtime for testing
#[cfg(test)]
mod mock;

// Unit tests
#[cfg(test)]
mod tests;

// Benchmarking module
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// Weights module - placeholder for now
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// DecodeWithMemTracking is required for enum types used in storage and events
	// to enable memory-safe decoding in the FRAME runtime
	use codec::DecodeWithMemTracking;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Saturating, Zero};

	// ============================================================================
	// TYPES AND STRUCTS
	// ============================================================================

	/// Represents the type of travel for which points were earned.
	/// This allows the system to categorize and potentially apply different rules
	/// based on travel type.
	#[derive(
		Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug,
	)]
	pub enum TravelType {
		Airline,
		Train,
		Bus,
		/// General/other travel types
		Other,
	}

	impl Default for TravelType {
		fn default() -> Self {
			TravelType::Other
		}
	}

	/// Represents the type of ticket/bonus NFT
	#[derive(
		Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug,
	)]
	pub enum TicketType {
		/// Plane ticket with flight details
		PlaneTicket,
		/// Train ticket
		TrainTicket,
		/// Bus ticket
		BusTicket,
		/// Bonus/reward (e.g., lounge access, upgrades)
		Bonus,
		/// Other type of ticket/voucher
		Other,
	}

	impl Default for TicketType {
		fn default() -> Self {
			TicketType::Other
		}
	}

	/// A single batch of points awarded to a user.
	/// Each batch tracks when points were earned, when they expire, and how many remain.
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct PointBatch<BlockNumber> {
		/// The block number when these points were earned
		pub earned_at_block: BlockNumber,
		/// The block number when these points will expire
		pub expires_at_block: BlockNumber,
		/// The remaining points in this batch (can decrease as points are spent)
		pub remaining_points: u128,
		/// The type of travel that earned these points
		pub travel_type: TravelType,
	}

	/// Maximum length for string fields in tickets
	pub const MAX_STRING_LEN: u32 = 128;

	/// NFT Ticket structure storing all relevant ticket information
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct Ticket<AccountId, BlockNumber> {
		/// Unique ticket ID
		pub id: u128,
		/// Owner of the ticket
		pub owner: AccountId,
		/// Issuer who created/sold this ticket
		pub issuer: AccountId,
		/// Type of ticket
		pub ticket_type: TicketType,
		/// Block when ticket was created
		pub created_at: BlockNumber,
		/// Block when ticket expires (if applicable)
		pub expires_at: Option<BlockNumber>,
		/// Points cost of the ticket (if purchased with points)
		pub points_cost: u128,
		/// Whether the ticket has been used/redeemed
		pub is_redeemed: bool,
		/// Passenger/holder name (for travel tickets)
		pub passenger_name: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Flight/train/bus number
		pub travel_number: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Gate information (for plane tickets)
		pub gate: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Seat number
		pub seat: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Departure location
		pub departure: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Arrival location
		pub arrival: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Departure time as encoded string (e.g., "2024-03-15 10:00", ISO 8601, or custom format)
		pub departure_time: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
		/// Additional metadata/notes
		pub metadata: BoundedVec<u8, ConstU32<MAX_STRING_LEN>>,
	}

	/// Staking info for a staker
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct StakeInfo<BlockNumber> {
		/// Amount staked
		pub amount: u128,
		/// Block when stake was made
		pub staked_at: BlockNumber,
		/// Whether this staker is selected as verifier for current period
		pub is_verifier: bool,
	}

	/// Daily issuer spending record for reward distribution
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	pub struct IssuerDailyRecord {
		/// Total points spent through this issuer today
		pub points_spent: u128,
		/// Number of transactions/redemptions
		pub transaction_count: u32,
	}

	// ============================================================================
	// ADVANCED STAKING TYPES (Slashing, Unbonding, Delegation, Eras)
	// ============================================================================

	/// Reason for slashing a staker
	#[derive(
		Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug,
	)]
	pub enum SlashReason {
		/// Staker was offline during verification duties
		Offline,
		/// Staker submitted invalid verification
		InvalidVerification,
		/// Staker engaged in malicious behavior
		Malicious,
		/// Other configurable reason
		Other,
	}

	impl Default for SlashReason {
		fn default() -> Self {
			SlashReason::Other
		}
	}

	/// Record of a slash event
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct SlashRecord<BlockNumber> {
		/// Amount slashed
		pub amount: u128,
		/// Block when slash occurred
		pub slashed_at: BlockNumber,
		/// Reason for the slash
		pub reason: SlashReason,
	}

	/// Info for unbonding/unstaking request
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct UnbondingInfo<BlockNumber> {
		/// Amount being unbonded
		pub amount: u128,
		/// Block when unbonding was requested
		pub requested_at: BlockNumber,
		/// Block when unbonding can be completed (requested_at + UnbondingPeriod)
		pub unlocks_at: BlockNumber,
	}

	/// Staking pool structure for delegation
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
	#[scale_info(skip_type_params(T))]
	pub struct StakingPool<AccountId, BlockNumber> {
		/// Pool operator/validator account
		pub operator: AccountId,
		/// Total stake in the pool (operator + delegators)
		pub total_stake: u128,
		/// Operator's own stake
		pub operator_stake: u128,
		/// Commission rate in basis points (e.g., 1000 = 10%)
		pub commission: u32,
		/// Block when pool was created
		pub created_at: BlockNumber,
		/// Whether pool is active (accepting delegations)
		pub is_active: bool,
		/// Number of current delegators
		pub delegator_count: u32,
	}

	impl<AccountId: Default, BlockNumber: Default> Default for StakingPool<AccountId, BlockNumber> {
		fn default() -> Self {
			Self {
				operator: AccountId::default(),
				total_stake: 0,
				operator_stake: 0,
				commission: 0,
				created_at: BlockNumber::default(),
				is_active: false,
				delegator_count: 0,
			}
		}
	}

	/// Delegation info for a delegator in a pool
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct DelegationInfo<BlockNumber> {
		/// Pool ID the delegation is to
		pub pool_id: u32,
		/// Amount delegated
		pub amount: u128,
		/// Block when delegation was made
		pub delegated_at: BlockNumber,
	}

	/// Enhanced stake info with unbonding support
	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct EnhancedStakeInfo<BlockNumber> {
		/// Active staked amount
		pub active: u128,
		/// Block when stake was made
		pub staked_at: BlockNumber,
		/// Whether this staker is selected as verifier for current era
		pub is_verifier: bool,
		/// Total slashed amount (historical)
		pub total_slashed: u128,
	}

	// ============================================================================
	// PALLET CONFIGURATION
	// ============================================================================

	/// The pallet struct - placeholder for implementing traits and dispatchables
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configuration trait for the pallet.
	/// Defines all types and constants that the pallet depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;

		/// Maximum number of point batches a single user can have.
		/// This prevents unbounded storage growth.
		/// Default: 100 batches per user
		#[pallet::constant]
		type MaxPointBatches: Get<u32>;

		/// Default expiration period for points in blocks.
		/// For example, if blocks are 6 seconds, 365 days ≈ 5,256,000 blocks
		#[pallet::constant]
		type DefaultExpirationPeriod: Get<BlockNumberFor<Self>>;

		/// Maximum number of tickets a user can own
		#[pallet::constant]
		type MaxTicketsPerUser: Get<u32>;

		/// Maximum number of stakers
		#[pallet::constant]
		type MaxStakers: Get<u32>;

		/// Minimum stake amount required
		#[pallet::constant]
		type MinStakeAmount: Get<u128>;

		/// Percentage of rewards going to stakers (rest goes to issuers)
		/// Stored as basis points (e.g., 3000 = 30%)
		#[pallet::constant]
		type StakerRewardPercent: Get<u32>;

		/// Blocks per reward period (e.g., 1 day worth of blocks)
		#[pallet::constant]
		type BlocksPerRewardPeriod: Get<BlockNumberFor<Self>>;

		// ============================================================================
		// ADVANCED STAKING CONFIGURATION
		// ============================================================================

		/// Unbonding period in blocks - time between unstake request and fund withdrawal
		/// Default: ~7 days worth of blocks
		#[pallet::constant]
		type UnbondingPeriod: Get<BlockNumberFor<Self>>;

		/// Slash percentage for offline validators (basis points, e.g., 500 = 5%)
		#[pallet::constant]
		type OfflineSlashPercent: Get<u32>;

		/// Slash percentage for invalid verification (basis points, e.g., 1000 = 10%)
		#[pallet::constant]
		type InvalidVerificationSlashPercent: Get<u32>;

		/// Slash percentage for malicious behavior (basis points, e.g., 10000 = 100%)
		#[pallet::constant]
		type MaliciousSlashPercent: Get<u32>;

		/// Maximum number of staking pools
		#[pallet::constant]
		type MaxPools: Get<u32>;

		/// Maximum delegators per pool
		#[pallet::constant]
		type MaxDelegatorsPerPool: Get<u32>;

		/// Minimum pool operator stake
		#[pallet::constant]
		type MinPoolOperatorStake: Get<u128>;

		/// Maximum commission a pool operator can charge (basis points)
		#[pallet::constant]
		type MaxPoolCommission: Get<u32>;

		/// Number of verifiers selected per era
		#[pallet::constant]
		type VerifiersPerEra: Get<u32>;

		/// Blocks per era for verifier rotation
		#[pallet::constant]
		type BlocksPerEra: Get<BlockNumberFor<Self>>;

		/// Percentage of rewards going to issuers (basis points, e.g., 2000 = 20%)
		/// Stakers receive the remainder (10000 - IssuerRewardPercent)
		#[pallet::constant]
		type IssuerRewardPercent: Get<u32>;

		/// Maximum unbonding requests per account
		#[pallet::constant]
		type MaxUnbondingRequests: Get<u32>;
	}

	// ============================================================================
	// STORAGE ITEMS
	// ============================================================================

	/// Stores the point batches for each user.
	/// Key: AccountId
	/// Value: BoundedVec of PointBatch (ordered by expiration date, oldest first)
	#[pallet::storage]
	#[pallet::getter(fn user_points)]
	pub type UserPoints<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<PointBatch<BlockNumberFor<T>>, T::MaxPointBatches>,
		ValueQuery,
	>;

	/// Stores the total points balance for each user (sum of all non-expired batches).
	/// This is a cached value for quick balance lookups.
	#[pallet::storage]
	#[pallet::getter(fn total_points)]
	pub type TotalPoints<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	/// Stores which accounts are authorized to issue points.
	/// These could be smart contracts or admin accounts.
	#[pallet::storage]
	#[pallet::getter(fn authorized_issuers)]
	pub type AuthorizedIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	/// Stores the admin/root account that can manage authorized issuers.
	/// This is set during genesis or by sudo.
	#[pallet::storage]
	#[pallet::getter(fn admin)]
	pub type Admin<T: Config> = StorageValue<_, T::AccountId>;

	// ============================================================================
	// NFT TICKET STORAGE
	// ============================================================================

	/// Next available ticket ID
	#[pallet::storage]
	#[pallet::getter(fn next_ticket_id)]
	pub type NextTicketId<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Storage for all tickets by ID
	#[pallet::storage]
	#[pallet::getter(fn tickets)]
	pub type Tickets<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, Ticket<T::AccountId, BlockNumberFor<T>>, OptionQuery>;

	/// Tickets owned by each user (list of ticket IDs)
	#[pallet::storage]
	#[pallet::getter(fn user_tickets)]
	pub type UserTickets<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<u128, T::MaxTicketsPerUser>,
		ValueQuery,
	>;

	// ============================================================================
	// STAKING STORAGE
	// ============================================================================

	/// Staking information for each staker
	#[pallet::storage]
	#[pallet::getter(fn stakes)]
	pub type Stakes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StakeInfo<BlockNumberFor<T>>, OptionQuery>;

	/// Total amount staked in the system
	#[pallet::storage]
	#[pallet::getter(fn total_staked)]
	pub type TotalStaked<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// List of all stakers
	#[pallet::storage]
	#[pallet::getter(fn staker_list)]
	pub type StakerList<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxStakers>, ValueQuery>;

	// ============================================================================
	// ISSUER REWARD TRACKING STORAGE
	// ============================================================================

	/// Daily spending records per issuer (keyed by period number and issuer)
	/// Period number = block_number / BlocksPerRewardPeriod
	#[pallet::storage]
	#[pallet::getter(fn issuer_daily_records)]
	pub type IssuerDailyRecords<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>, // Period number
		Blake2_128Concat,
		T::AccountId, // Issuer account
		IssuerDailyRecord,
		ValueQuery,
	>;

	/// Total points spent in a period (for calculating issuer proportions)
	#[pallet::storage]
	#[pallet::getter(fn period_total_spent)]
	pub type PeriodTotalSpent<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>, // Period number
		u128,
		ValueQuery,
	>;

	/// Accumulated rewards pool for distribution
	#[pallet::storage]
	#[pallet::getter(fn reward_pool)]
	pub type RewardPool<T: Config> = StorageValue<_, u128, ValueQuery>;

	// ============================================================================
	// ADVANCED STAKING STORAGE (Slashing, Unbonding, Pools, Eras)
	// ============================================================================

	/// Enhanced staking information using StorageMap for scalability (replaces Vec-based list)
	#[pallet::storage]
	#[pallet::getter(fn enhanced_stakes)]
	pub type EnhancedStakes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		EnhancedStakeInfo<BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Count of stakers for efficient iteration
	#[pallet::storage]
	#[pallet::getter(fn staker_count)]
	pub type StakerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Unbonding requests per staker
	#[pallet::storage]
	#[pallet::getter(fn unbonding_requests)]
	pub type UnbondingRequests<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<UnbondingInfo<BlockNumberFor<T>>, T::MaxUnbondingRequests>,
		ValueQuery,
	>;

	/// Slash records for each staker
	#[pallet::storage]
	#[pallet::getter(fn slash_records)]
	pub type SlashRecords<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<SlashRecord<BlockNumberFor<T>>, ConstU32<100>>,
		ValueQuery,
	>;

	/// Total amount slashed (for statistics)
	#[pallet::storage]
	#[pallet::getter(fn total_slashed)]
	pub type TotalSlashed<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Staking pools - keyed by pool ID
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		StakingPool<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Next pool ID
	#[pallet::storage]
	#[pallet::getter(fn next_pool_id)]
	pub type NextPoolId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Delegations by delegator account
	#[pallet::storage]
	#[pallet::getter(fn delegations)]
	pub type Delegations<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		DelegationInfo<BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Pool delegators - maps pool_id to list of delegator accounts
	#[pallet::storage]
	#[pallet::getter(fn pool_delegators)]
	pub type PoolDelegators<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BoundedVec<T::AccountId, T::MaxDelegatorsPerPool>,
		ValueQuery,
	>;

	/// Current era number
	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Selected verifiers for current era
	#[pallet::storage]
	#[pallet::getter(fn era_verifiers)]
	pub type EraVerifiers<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32, // Era number
		BoundedVec<T::AccountId, T::VerifiersPerEra>,
		ValueQuery,
	>;

	/// Last era rotation block
	#[pallet::storage]
	#[pallet::getter(fn last_era_block)]
	pub type LastEraBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Pending rewards for stakers (accumulated but not yet claimed)
	#[pallet::storage]
	#[pallet::getter(fn pending_staker_rewards)]
	pub type PendingStakerRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	/// Pending rewards for issuers
	#[pallet::storage]
	#[pallet::getter(fn pending_issuer_rewards)]
	pub type PendingIssuerRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	// ============================================================================
	// GENESIS CONFIGURATION
	// ============================================================================

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// The admin account that can manage authorized issuers
		pub admin: Option<T::AccountId>,
		/// Initial list of authorized issuers
		pub authorized_issuers: Vec<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Set the admin if provided
			if let Some(ref admin) = self.admin {
				Admin::<T>::put(admin);
			}
			// Register initial authorized issuers
			for issuer in &self.authorized_issuers {
				AuthorizedIssuers::<T>::insert(issuer, true);
			}
		}
	}

	// ============================================================================
	// EVENTS
	// ============================================================================

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Points were awarded to a user
		/// [recipient, amount, expires_at_block, travel_type]
		PointsEarned {
			/// The account that received the points
			recipient: T::AccountId,
			/// The amount of points earned
			amount: u128,
			/// The block number when these points expire
			expires_at_block: BlockNumberFor<T>,
			/// The type of travel that earned these points
			travel_type: TravelType,
		},

		/// Points were spent/used by a user (with issuer tracking)
		/// [user, amount_spent, remaining_balance, issuer]
		PointsSpent {
			/// The account that spent the points
			user: T::AccountId,
			/// The amount of points spent
			amount_spent: u128,
			/// The remaining point balance after spending
			remaining_balance: u128,
			/// The issuer where points were spent
			issuer: T::AccountId,
		},

		/// Points expired for a user (removed from their balance)
		/// [user, amount_expired, batches_removed]
		PointsExpired {
			/// The account whose points expired
			user: T::AccountId,
			/// The total amount of points that expired
			amount_expired: u128,
			/// The number of batches removed
			batches_removed: u32,
		},

		/// An account was authorized to issue points
		/// [issuer]
		IssuerAuthorized {
			/// The account that was authorized
			issuer: T::AccountId,
		},

		/// An account's authorization to issue points was revoked
		/// [issuer]
		IssuerRevoked {
			/// The account whose authorization was revoked
			issuer: T::AccountId,
		},

		/// Admin account was changed
		/// [old_admin, new_admin]
		AdminChanged {
			/// The previous admin (if any)
			old_admin: Option<T::AccountId>,
			/// The new admin
			new_admin: T::AccountId,
		},

		/// A new ticket was minted
		TicketMinted {
			/// Ticket ID
			ticket_id: u128,
			/// Owner of the ticket
			owner: T::AccountId,
			/// Issuer who created the ticket
			issuer: T::AccountId,
			/// Type of ticket
			ticket_type: TicketType,
			/// Points cost
			points_cost: u128,
		},

		/// A ticket was redeemed/used
		TicketRedeemed {
			/// Ticket ID
			ticket_id: u128,
			/// Owner who redeemed it
			owner: T::AccountId,
		},

		/// A ticket was transferred to a new owner
		TicketTransferred {
			/// Ticket ID
			ticket_id: u128,
			/// Previous owner
			from: T::AccountId,
			/// New owner
			to: T::AccountId,
		},

		/// Tokens were staked
		Staked {
			/// Staker account
			staker: T::AccountId,
			/// Amount staked
			amount: u128,
		},

		/// Tokens were unstaked
		Unstaked {
			/// Staker account
			staker: T::AccountId,
			/// Amount unstaked
			amount: u128,
		},

		/// Rewards were distributed
		RewardsDistributed {
			/// Period for which rewards were distributed
			period: BlockNumberFor<T>,
			/// Total rewards distributed to stakers
			staker_rewards: u128,
			/// Total rewards distributed to issuers
			issuer_rewards: u128,
		},

		/// Reward claimed by an account
		RewardClaimed {
			/// Account that claimed
			account: T::AccountId,
			/// Amount claimed
			amount: u128,
		},

		// ============================================================================
		// ADVANCED STAKING EVENTS
		// ============================================================================

		/// A staker was slashed
		Slashed {
			/// Staker account that was slashed
			staker: T::AccountId,
			/// Amount slashed
			amount: u128,
			/// Reason for slash
			reason: SlashReason,
		},

		/// Unbonding initiated (stake locked until unbonding period ends)
		UnbondingInitiated {
			/// Staker account
			staker: T::AccountId,
			/// Amount being unbonded
			amount: u128,
			/// Block when unbonding can be completed
			unlocks_at: BlockNumberFor<T>,
		},

		/// Unbonded funds withdrawn
		UnbondingWithdrawn {
			/// Staker account
			staker: T::AccountId,
			/// Amount withdrawn
			amount: u128,
		},

		/// Unbonding request was cancelled
		UnbondingCancelled {
			/// Staker account
			staker: T::AccountId,
			/// Amount re-staked
			amount: u128,
		},

		/// A new staking pool was created
		PoolCreated {
			/// Pool ID
			pool_id: u32,
			/// Operator account
			operator: T::AccountId,
			/// Initial stake from operator
			initial_stake: u128,
			/// Commission rate in basis points
			commission: u32,
		},

		/// Stake delegated to a pool
		Delegated {
			/// Delegator account
			delegator: T::AccountId,
			/// Pool ID
			pool_id: u32,
			/// Amount delegated
			amount: u128,
		},

		/// Delegation withdrawn from a pool
		Undelegated {
			/// Delegator account
			delegator: T::AccountId,
			/// Pool ID
			pool_id: u32,
			/// Amount withdrawn
			amount: u128,
		},

		/// Pool commission was updated
		PoolCommissionUpdated {
			/// Pool ID
			pool_id: u32,
			/// New commission rate in basis points
			new_commission: u32,
		},

		/// Pool was closed/deactivated
		PoolClosed {
			/// Pool ID
			pool_id: u32,
			/// Operator account
			operator: T::AccountId,
		},

		/// New era started and verifiers rotated
		EraRotated {
			/// New era number
			era: u32,
			/// Number of verifiers selected
			verifier_count: u32,
		},

		/// Verifier selected for the current era
		VerifierSelected {
			/// Era number
			era: u32,
			/// Selected verifier account
			verifier: T::AccountId,
		},

		/// Staker added additional stake
		StakeIncreased {
			/// Staker account
			staker: T::AccountId,
			/// Additional amount staked
			amount: u128,
			/// New total stake
			new_total: u128,
		},
	}

	// ============================================================================
	// ERRORS
	// ============================================================================

	/// Errors that can be returned by this pallet
	#[pallet::error]
	pub enum Error<T> {
		/// The caller is not authorized to issue points
		NotAuthorizedIssuer,
		/// The caller is not the admin
		NotAdmin,
		/// User does not have enough points for the requested operation
		InsufficientPoints,
		/// The user has reached the maximum number of point batches
		TooManyBatches,
		/// Arithmetic overflow occurred during calculation
		ArithmeticOverflow,
		/// Arithmetic underflow occurred during calculation
		ArithmeticUnderflow,
		/// The amount must be greater than zero
		ZeroAmount,
		/// No admin has been set
		NoAdmin,
		/// The issuer is already authorized
		AlreadyAuthorized,
		/// The issuer is not authorized (can't revoke)
		NotAuthorized,
		/// Ticket not found
		TicketNotFound,
		/// Not the ticket owner
		NotTicketOwner,
		/// Ticket already redeemed
		TicketAlreadyRedeemed,
		/// Ticket has expired
		TicketExpired,
		/// User has too many tickets
		TooManyTickets,
		/// Stake amount below minimum
		StakeBelowMinimum,
		/// Already staking
		AlreadyStaking,
		/// Not a staker
		NotStaker,
		/// Cannot unstake yet
		CannotUnstakeYet,
		/// Too many stakers
		TooManyStakers,
		/// No rewards to claim
		NoRewardsToClaim,
		/// String too long for bounded vec
		StringTooLong,

		// ============================================================================
		// ADVANCED STAKING ERRORS
		// ============================================================================

		/// Unbonding period not yet complete
		UnbondingNotComplete,
		/// No unbonding requests found
		NoUnbondingRequests,
		/// Maximum unbonding requests reached
		TooManyUnbondingRequests,
		/// Pool not found
		PoolNotFound,
		/// Not the pool operator
		NotPoolOperator,
		/// Pool is not active
		PoolNotActive,
		/// Already delegating to a pool
		AlreadyDelegating,
		/// Not delegating to any pool
		NotDelegating,
		/// Delegation amount below minimum
		DelegationBelowMinimum,
		/// Too many pools
		TooManyPools,
		/// Too many delegators in pool
		TooManyDelegators,
		/// Commission exceeds maximum allowed
		CommissionTooHigh,
		/// Insufficient stake for pool operator
		InsufficientOperatorStake,
		/// Cannot slash zero amount
		SlashAmountZero,
		/// Pool has active delegators, cannot close
		PoolHasDelegators,
		/// Era rotation not yet due
		EraRotationNotDue,
		/// Not a verifier for current era
		NotVerifier,
		/// Insufficient balance for operation
		InsufficientBalance,
	}

	// ============================================================================
	// DISPATCHABLE FUNCTIONS (EXTRINSICS)
	// ============================================================================

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Award points to a user. Only callable by authorized issuers.
		///
		/// This function creates a new point batch for the recipient with:
		/// - Current block as the earned_at_block
		/// - expiration_blocks + current block as expires_at_block
		/// - The specified amount of points
		/// - The specified travel type
		///
		/// ## Parameters
		/// - `origin`: Must be an authorized issuer
		/// - `recipient`: The account to receive the points
		/// - `amount`: The number of points to award (must be > 0)
		/// - `travel_type`: The type of travel that earned these points
		/// - `custom_expiration`: Optional custom expiration period in blocks.
		///   If None, uses the default expiration period.
		///
		/// ## Emits
		/// - `PointsEarned` on success
		///
		/// ## Errors
		/// - `NotAuthorizedIssuer` if the caller is not authorized
		/// - `ZeroAmount` if amount is 0
		/// - `TooManyBatches` if the user already has max batches
		/// - `ArithmeticOverflow` if calculations overflow
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::award_points())]
		pub fn award_points(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			amount: u128,
			travel_type: TravelType,
			custom_expiration: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Verify the caller is an authorized issuer
			let issuer = ensure_signed(origin)?;
			ensure!(AuthorizedIssuers::<T>::get(&issuer), Error::<T>::NotAuthorizedIssuer);

			// Amount must be greater than zero
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// Get current block number
			let current_block = frame_system::Pallet::<T>::block_number();

			// Calculate expiration block
			let expiration_period = custom_expiration.unwrap_or(T::DefaultExpirationPeriod::get());
			let expires_at_block = current_block.saturating_add(expiration_period);

			// Create the new point batch
			let new_batch = PointBatch {
				earned_at_block: current_block,
				expires_at_block,
				remaining_points: amount,
				travel_type: travel_type.clone(),
			};

			// Add the batch to the user's batches
			UserPoints::<T>::try_mutate(&recipient, |batches| -> DispatchResult {
				// First, clean up any expired batches to make room
				Self::remove_expired_batches_internal(&recipient, batches, current_block);

				// Try to add the new batch
				batches.try_push(new_batch).map_err(|_| Error::<T>::TooManyBatches)?;

				// Sort batches by expiration date (oldest first) for FIFO deduction
				batches.sort_by(|a, b| a.expires_at_block.cmp(&b.expires_at_block));

				Ok(())
			})?;

			// Update total points balance
			TotalPoints::<T>::try_mutate(&recipient, |total| -> DispatchResult {
				*total = total.checked_add(amount).ok_or(Error::<T>::ArithmeticOverflow)?;
				Ok(())
			})?;

			// Emit event
			Self::deposit_event(Event::PointsEarned {
				recipient,
				amount,
				expires_at_block,
				travel_type,
			});

			Ok(())
		}

		/// Spend points from a user's balance. Uses FIFO (oldest points first).
		///
		/// This function deducts points starting from the oldest (earliest expiring)
		/// batches first, ensuring users don't lose points to expiration when they
		/// have newer points available.
		///
		/// ## Parameters
		/// - `origin`: The signed origin (the user spending their points)
		/// - `amount`: The number of points to spend (must be > 0)
		///
		/// ## Emits
		/// - `PointsSpent` on success
		///
		/// ## Errors
		/// - `ZeroAmount` if amount is 0
		/// - `InsufficientPoints` if user doesn't have enough points
		/// - `ArithmeticUnderflow` if calculations underflow
		/// - `NotAuthorizedIssuer` if issuer is not authorized
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::spend_points())]
		pub fn spend_points(
			origin: OriginFor<T>,
			amount: u128,
			issuer: T::AccountId,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;

			// Amount must be greater than zero
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// Verify the issuer is authorized
			ensure!(AuthorizedIssuers::<T>::get(&issuer), Error::<T>::NotAuthorizedIssuer);

			// Get current block for expiration checking
			let current_block = frame_system::Pallet::<T>::block_number();

			let mut remaining_to_spend = amount;

			// Deduct points from batches (FIFO - oldest first)
			UserPoints::<T>::try_mutate(&user, |batches| -> DispatchResult {
				// First, remove expired batches
				Self::remove_expired_batches_internal(&user, batches, current_block);

				// Calculate total available points (non-expired)
				let available: u128 = batches.iter().map(|b| b.remaining_points).sum();
				ensure!(available >= amount, Error::<T>::InsufficientPoints);

				// Deduct from batches (they're already sorted by expiration - oldest first)
				// We iterate through and deduct from each batch until we've spent enough
				for batch in batches.iter_mut() {
					if remaining_to_spend == 0 {
						break;
					}

					// How much can we take from this batch?
					let deduction = remaining_to_spend.min(batch.remaining_points);
					batch.remaining_points = batch
						.remaining_points
						.checked_sub(deduction)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;
					remaining_to_spend = remaining_to_spend
						.checked_sub(deduction)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;
				}

				// Remove any batches that are now empty
				batches.retain(|b| b.remaining_points > 0);

				Ok(())
			})?;

			// Update total points balance
			let new_balance =
				TotalPoints::<T>::try_mutate(&user, |total| -> Result<u128, DispatchError> {
					*total = total.checked_sub(amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
					Ok(*total)
				})?;

			// Track spending for issuer reward distribution
			let period = Self::current_period();
			IssuerDailyRecords::<T>::mutate(period, &issuer, |record| {
				record.points_spent = record.points_spent.saturating_add(amount);
				record.transaction_count = record.transaction_count.saturating_add(1);
			});
			PeriodTotalSpent::<T>::mutate(period, |total| {
				*total = total.saturating_add(amount);
			});

			// Emit event
			Self::deposit_event(Event::PointsSpent {
				user,
				amount_spent: amount,
				remaining_balance: new_balance,
				issuer,
			});

			Ok(())
		}

		/// Clean up expired point batches for a user.
		///
		/// This is a maintenance function that can be called by anyone to remove
		/// expired batches from a user's storage. This helps keep storage clean
		/// and reduces storage costs.
		///
		/// ## Parameters
		/// - `origin`: Any signed origin
		/// - `user`: The account whose expired batches should be cleaned
		///
		/// ## Emits
		/// - `PointsExpired` if any batches were removed
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::cleanup_expired())]
		pub fn cleanup_expired(origin: OriginFor<T>, user: T::AccountId) -> DispatchResult {
			ensure_signed(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();

			UserPoints::<T>::mutate(&user, |batches| {
				Self::remove_expired_batches_internal(&user, batches, current_block);
			});

			Ok(())
		}

		/// Authorize an account to issue points.
		///
		/// ## Parameters
		/// - `origin`: Must be the admin
		/// - `issuer`: The account to authorize
		///
		/// ## Emits
		/// - `IssuerAuthorized` on success
		///
		/// ## Errors
		/// - `NotAdmin` if caller is not the admin
		/// - `AlreadyAuthorized` if the issuer is already authorized
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::authorize_issuer())]
		pub fn authorize_issuer(origin: OriginFor<T>, issuer: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::ensure_admin(&caller)?;

			ensure!(!AuthorizedIssuers::<T>::get(&issuer), Error::<T>::AlreadyAuthorized);

			AuthorizedIssuers::<T>::insert(&issuer, true);

			Self::deposit_event(Event::IssuerAuthorized { issuer });
			Ok(())
		}

		/// Revoke an account's authorization to issue points.
		///
		/// ## Parameters
		/// - `origin`: Must be the admin
		/// - `issuer`: The account to revoke
		///
		/// ## Emits
		/// - `IssuerRevoked` on success
		///
		/// ## Errors
		/// - `NotAdmin` if caller is not the admin
		/// - `NotAuthorized` if the issuer wasn't authorized
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::revoke_issuer())]
		pub fn revoke_issuer(origin: OriginFor<T>, issuer: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::ensure_admin(&caller)?;

			ensure!(AuthorizedIssuers::<T>::get(&issuer), Error::<T>::NotAuthorized);

			AuthorizedIssuers::<T>::remove(&issuer);

			Self::deposit_event(Event::IssuerRevoked { issuer });
			Ok(())
		}

		/// Set a new admin account. Can be called by current admin or root.
		///
		/// ## Parameters
		/// - `origin`: Must be the current admin or root
		/// - `new_admin`: The new admin account
		///
		/// ## Emits
		/// - `AdminChanged` on success
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_admin())]
		pub fn set_admin(origin: OriginFor<T>, new_admin: T::AccountId) -> DispatchResult {
			// Allow both root and current admin to change the admin
			let caller = ensure_signed(origin.clone()).ok();

			// Check if caller is root or admin
			let is_root = ensure_root(origin.clone()).is_ok();
			let is_admin = caller.as_ref().is_some_and(|c| Self::is_admin(c));

			ensure!(is_root || is_admin, Error::<T>::NotAdmin);

			let old_admin = Admin::<T>::get();
			Admin::<T>::put(&new_admin);

			Self::deposit_event(Event::AdminChanged { old_admin, new_admin });
			Ok(())
		}

		// ============================================================================
		// NFT TICKET FUNCTIONS
		// ============================================================================

		/// Mint a new ticket NFT. Only callable by authorized issuers.
		///
		/// ## Parameters
		/// - `origin`: Must be an authorized issuer
		/// - `owner`: The account that will own the ticket
		/// - `ticket_type`: Type of ticket (plane, train, bus, bonus, etc.)
		/// - `points_cost`: Points cost of the ticket (deducted from owner if > 0)
		/// - `expires_at`: Optional expiration block for the ticket
		/// - `passenger_name`: Name of the passenger/holder
		/// - `travel_number`: Flight/train/bus number
		/// - `gate`: Gate information (for plane tickets)
		/// - `seat`: Seat number
		/// - `departure`: Departure location
		/// - `arrival`: Arrival location
		/// - `departure_time`: Departure time
		/// - `metadata`: Additional metadata
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::mint_ticket())]
		pub fn mint_ticket(
			origin: OriginFor<T>,
			owner: T::AccountId,
			ticket_type: TicketType,
			points_cost: u128,
			expires_at: Option<BlockNumberFor<T>>,
			passenger_name: Vec<u8>,
			travel_number: Vec<u8>,
			gate: Vec<u8>,
			seat: Vec<u8>,
			departure: Vec<u8>,
			arrival: Vec<u8>,
			departure_time: Vec<u8>,
			metadata: Vec<u8>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(AuthorizedIssuers::<T>::get(&issuer), Error::<T>::NotAuthorizedIssuer);

			let current_block = frame_system::Pallet::<T>::block_number();

			// If points_cost > 0, deduct from owner using internal spend
			if points_cost > 0 {
				Self::spend_points_internal(&owner, points_cost, &issuer)?;
			}

			// Get and increment ticket ID
			let ticket_id = NextTicketId::<T>::get();
			NextTicketId::<T>::put(ticket_id.saturating_add(1));

			// Create the ticket
			let ticket = Ticket {
				id: ticket_id,
				owner: owner.clone(),
				issuer: issuer.clone(),
				ticket_type: ticket_type.clone(),
				created_at: current_block,
				expires_at,
				points_cost,
				is_redeemed: false,
				passenger_name: BoundedVec::try_from(passenger_name)
					.map_err(|_| Error::<T>::StringTooLong)?,
				travel_number: BoundedVec::try_from(travel_number)
					.map_err(|_| Error::<T>::StringTooLong)?,
				gate: BoundedVec::try_from(gate).map_err(|_| Error::<T>::StringTooLong)?,
				seat: BoundedVec::try_from(seat).map_err(|_| Error::<T>::StringTooLong)?,
				departure: BoundedVec::try_from(departure)
					.map_err(|_| Error::<T>::StringTooLong)?,
				arrival: BoundedVec::try_from(arrival).map_err(|_| Error::<T>::StringTooLong)?,
				departure_time: BoundedVec::try_from(departure_time)
					.map_err(|_| Error::<T>::StringTooLong)?,
				metadata: BoundedVec::try_from(metadata).map_err(|_| Error::<T>::StringTooLong)?,
			};

			// Store the ticket
			Tickets::<T>::insert(ticket_id, ticket);

			// Add to user's ticket list
			UserTickets::<T>::try_mutate(&owner, |tickets| -> DispatchResult {
				tickets.try_push(ticket_id).map_err(|_| Error::<T>::TooManyTickets)?;
				Ok(())
			})?;

			Self::deposit_event(Event::TicketMinted {
				ticket_id,
				owner,
				issuer,
				ticket_type,
				points_cost,
			});

			Ok(())
		}

		/// Redeem/use a ticket. Only the owner can redeem their ticket.
		///
		/// ## Parameters
		/// - `origin`: Must be the ticket owner
		/// - `ticket_id`: ID of the ticket to redeem
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::redeem_ticket())]
		pub fn redeem_ticket(origin: OriginFor<T>, ticket_id: u128) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			Tickets::<T>::try_mutate(ticket_id, |maybe_ticket| -> DispatchResult {
				let ticket = maybe_ticket.as_mut().ok_or(Error::<T>::TicketNotFound)?;
				ensure!(ticket.owner == owner, Error::<T>::NotTicketOwner);
				ensure!(!ticket.is_redeemed, Error::<T>::TicketAlreadyRedeemed);

				// Check if ticket has expired
				if let Some(expires_at) = ticket.expires_at {
					let current_block = frame_system::Pallet::<T>::block_number();
					ensure!(current_block < expires_at, Error::<T>::TicketExpired);
				}

				ticket.is_redeemed = true;
				Ok(())
			})?;

			Self::deposit_event(Event::TicketRedeemed { ticket_id, owner });

			Ok(())
		}

		/// Transfer a ticket to another account.
		///
		/// ## Parameters
		/// - `origin`: Must be the ticket owner
		/// - `ticket_id`: ID of the ticket to transfer
		/// - `to`: The new owner
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::spend_points())]
		pub fn transfer_ticket(
			origin: OriginFor<T>,
			ticket_id: u128,
			to: T::AccountId,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			Tickets::<T>::try_mutate(ticket_id, |maybe_ticket| -> DispatchResult {
				let ticket = maybe_ticket.as_mut().ok_or(Error::<T>::TicketNotFound)?;
				ensure!(ticket.owner == from, Error::<T>::NotTicketOwner);
				ensure!(!ticket.is_redeemed, Error::<T>::TicketAlreadyRedeemed);

				ticket.owner = to.clone();
				Ok(())
			})?;

			// Update user ticket lists
			UserTickets::<T>::mutate(&from, |tickets| {
				tickets.retain(|&id| id != ticket_id);
			});

			UserTickets::<T>::try_mutate(&to, |tickets| -> DispatchResult {
				tickets.try_push(ticket_id).map_err(|_| Error::<T>::TooManyTickets)?;
				Ok(())
			})?;

			Self::deposit_event(Event::TicketTransferred { ticket_id, from, to });

			Ok(())
		}

		// ============================================================================
		// STAKING FUNCTIONS
		// ============================================================================

		/// Stake tokens to become a verifier/staker and earn rewards.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		/// - `amount`: Amount to stake (must be >= MinStakeAmount)
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::stake())]
		pub fn stake(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			ensure!(amount >= T::MinStakeAmount::get(), Error::<T>::StakeBelowMinimum);
			ensure!(Stakes::<T>::get(&staker).is_none(), Error::<T>::AlreadyStaking);

			let current_block = frame_system::Pallet::<T>::block_number();

			let stake_info = StakeInfo { amount, staked_at: current_block, is_verifier: false };

			Stakes::<T>::insert(&staker, stake_info);

			// Add to staker list
			StakerList::<T>::try_mutate(|stakers| -> DispatchResult {
				stakers.try_push(staker.clone()).map_err(|_| Error::<T>::TooManyStakers)?;
				Ok(())
			})?;

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_add(amount);
			});

			Self::deposit_event(Event::Staked { staker, amount });

			Ok(())
		}

		/// Unstake tokens and withdraw from staking.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::unstake())]
		pub fn unstake(origin: OriginFor<T>) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			let stake_info = Stakes::<T>::get(&staker).ok_or(Error::<T>::NotStaker)?;
			let amount = stake_info.amount;

			// Remove stake
			Stakes::<T>::remove(&staker);

			// Remove from staker list
			StakerList::<T>::mutate(|stakers| {
				stakers.retain(|s| s != &staker);
			});

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_sub(amount);
			});

			Self::deposit_event(Event::Unstaked { staker, amount });

			Ok(())
		}

		/// Add tokens to the reward pool. Can be called by anyone.
		///
		/// ## Parameters
		/// - `origin`: Any signed origin
		/// - `amount`: Amount to add to the reward pool
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::add_to_reward_pool())]
		pub fn add_to_reward_pool(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			RewardPool::<T>::mutate(|pool| {
				*pool = pool.saturating_add(amount);
			});

			Ok(())
		}

		// ============================================================================
		// ADVANCED STAKING EXTRINSICS
		// ============================================================================

		/// Request unbonding of staked tokens. Initiates the unbonding period.
		/// Tokens will be locked until the unbonding period ends.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		/// - `amount`: Amount to unbond (must be <= current stake)
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::request_unbond())]
		pub fn request_unbond(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// Check if staker has a stake
			let stake_info = Stakes::<T>::get(&staker).ok_or(Error::<T>::NotStaker)?;
			ensure!(stake_info.amount >= amount, Error::<T>::InsufficientBalance);

			let current_block = frame_system::Pallet::<T>::block_number();
			let unlocks_at = current_block.saturating_add(T::UnbondingPeriod::get());

			// Create unbonding request
			let unbonding_info = UnbondingInfo { amount, requested_at: current_block, unlocks_at };

			// Add to unbonding requests
			UnbondingRequests::<T>::try_mutate(&staker, |requests| -> DispatchResult {
				requests
					.try_push(unbonding_info)
					.map_err(|_| Error::<T>::TooManyUnbondingRequests)?;
				Ok(())
			})?;

			// Reduce active stake
			Stakes::<T>::try_mutate(&staker, |maybe_info| -> DispatchResult {
				let info = maybe_info.as_mut().ok_or(Error::<T>::NotStaker)?;
				info.amount = info.amount.saturating_sub(amount);
				Ok(())
			})?;

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_sub(amount);
			});

			Self::deposit_event(Event::UnbondingInitiated { staker, amount, unlocks_at });

			Ok(())
		}

		/// Withdraw unbonded tokens after the unbonding period has ended.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::withdraw_unbonded())]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();
			let mut total_withdrawn: u128 = 0;

			UnbondingRequests::<T>::try_mutate(&staker, |requests| -> DispatchResult {
				ensure!(!requests.is_empty(), Error::<T>::NoUnbondingRequests);

				// Process all completed unbonding requests
				let mut remaining = Vec::new();
				for req in requests.iter() {
					if req.unlocks_at <= current_block {
						total_withdrawn = total_withdrawn.saturating_add(req.amount);
					} else {
						remaining.push(req.clone());
					}
				}

				ensure!(total_withdrawn > 0, Error::<T>::UnbondingNotComplete);

				// Replace with remaining requests
				*requests = BoundedVec::try_from(remaining)
					.map_err(|_| Error::<T>::TooManyUnbondingRequests)?;

				Ok(())
			})?;

			// Clean up staker if no stake and no unbonding requests remain
			if let Some(info) = Stakes::<T>::get(&staker) {
				if info.amount == 0 && UnbondingRequests::<T>::get(&staker).is_empty() {
					Stakes::<T>::remove(&staker);
					StakerList::<T>::mutate(|stakers| {
						stakers.retain(|s| s != &staker);
					});
				}
			}

			Self::deposit_event(Event::UnbondingWithdrawn { staker, amount: total_withdrawn });

			Ok(())
		}

		/// Cancel pending unbonding requests and re-stake the tokens.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::cancel_unbonding())]
		pub fn cancel_unbonding(origin: OriginFor<T>) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			let mut total_rebonded: u128 = 0;

			UnbondingRequests::<T>::try_mutate(&staker, |requests| -> DispatchResult {
				ensure!(!requests.is_empty(), Error::<T>::NoUnbondingRequests);

				// Sum all unbonding amounts
				total_rebonded = requests.iter().map(|r| r.amount).sum();

				// Clear all requests
				*requests = BoundedVec::default();

				Ok(())
			})?;

			// Re-add to stake
			Stakes::<T>::try_mutate(&staker, |maybe_info| -> DispatchResult {
				let info = maybe_info.as_mut().ok_or(Error::<T>::NotStaker)?;
				info.amount = info.amount.saturating_add(total_rebonded);
				Ok(())
			})?;

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_add(total_rebonded);
			});

			Self::deposit_event(Event::UnbondingCancelled { staker, amount: total_rebonded });

			Ok(())
		}

		/// Slash a staker for misbehavior. Admin only.
		///
		/// ## Parameters
		/// - `origin`: Must be admin
		/// - `staker`: Account to slash
		/// - `reason`: Reason for slashing
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::slash_staker())]
		pub fn slash_staker(
			origin: OriginFor<T>,
			staker: T::AccountId,
			reason: SlashReason,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::ensure_admin(&caller)?;

			// Get slash percentage based on reason
			let slash_percent = match reason {
				SlashReason::Offline => T::OfflineSlashPercent::get(),
				SlashReason::InvalidVerification => T::InvalidVerificationSlashPercent::get(),
				SlashReason::Malicious => T::MaliciousSlashPercent::get(),
				SlashReason::Other => T::OfflineSlashPercent::get(), // Use offline as default
			};

			let stake_info = Stakes::<T>::get(&staker).ok_or(Error::<T>::NotStaker)?;
			let slash_amount = stake_info
				.amount
				.saturating_mul(slash_percent as u128)
				.saturating_div(10_000);

			ensure!(slash_amount > 0, Error::<T>::SlashAmountZero);

			let current_block = frame_system::Pallet::<T>::block_number();

			// Record slash
			SlashRecords::<T>::try_mutate(&staker, |records| -> DispatchResult {
				let record = SlashRecord {
					amount: slash_amount,
					slashed_at: current_block,
					reason: reason.clone(),
				};
				let _ = records.try_push(record); // Ignore if full
				Ok(())
			})?;

			// Reduce stake
			Stakes::<T>::mutate(&staker, |maybe_info| {
				if let Some(info) = maybe_info {
					info.amount = info.amount.saturating_sub(slash_amount);
				}
			});

			// Update totals
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_sub(slash_amount);
			});
			TotalSlashed::<T>::mutate(|total| {
				*total = total.saturating_add(slash_amount);
			});

			Self::deposit_event(Event::Slashed { staker, amount: slash_amount, reason });

			Ok(())
		}

		/// Create a new staking pool. Caller becomes the pool operator.
		///
		/// ## Parameters
		/// - `origin`: The operator account
		/// - `initial_stake`: Initial stake from operator
		/// - `commission`: Commission rate in basis points (max: MaxPoolCommission)
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::create_pool())]
		pub fn create_pool(
			origin: OriginFor<T>,
			initial_stake: u128,
			commission: u32,
		) -> DispatchResult {
			let operator = ensure_signed(origin)?;

			ensure!(
				initial_stake >= T::MinPoolOperatorStake::get(),
				Error::<T>::InsufficientOperatorStake
			);
			ensure!(commission <= T::MaxPoolCommission::get(), Error::<T>::CommissionTooHigh);

			let pool_id = NextPoolId::<T>::get();
			ensure!(pool_id < T::MaxPools::get(), Error::<T>::TooManyPools);

			let current_block = frame_system::Pallet::<T>::block_number();

			let pool = StakingPool {
				operator: operator.clone(),
				total_stake: initial_stake,
				operator_stake: initial_stake,
				commission,
				created_at: current_block,
				is_active: true,
				delegator_count: 0,
			};

			Pools::<T>::insert(pool_id, pool);
			NextPoolId::<T>::put(pool_id.saturating_add(1));

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_add(initial_stake);
			});

			Self::deposit_event(Event::PoolCreated { pool_id, operator, initial_stake, commission });

			Ok(())
		}

		/// Delegate stake to a pool.
		///
		/// ## Parameters
		/// - `origin`: The delegator account
		/// - `pool_id`: Pool to delegate to
		/// - `amount`: Amount to delegate
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::delegate())]
		pub fn delegate(origin: OriginFor<T>, pool_id: u32, amount: u128) -> DispatchResult {
			let delegator = ensure_signed(origin)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
			ensure!(amount >= T::MinStakeAmount::get(), Error::<T>::DelegationBelowMinimum);
			ensure!(Delegations::<T>::get(&delegator).is_none(), Error::<T>::AlreadyDelegating);

			let current_block = frame_system::Pallet::<T>::block_number();

			// Update pool
			Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				ensure!(pool.is_active, Error::<T>::PoolNotActive);

				pool.total_stake = pool.total_stake.saturating_add(amount);
				pool.delegator_count = pool.delegator_count.saturating_add(1);

				Ok(())
			})?;

			// Add delegator to pool
			PoolDelegators::<T>::try_mutate(pool_id, |delegators| -> DispatchResult {
				delegators
					.try_push(delegator.clone())
					.map_err(|_| Error::<T>::TooManyDelegators)?;
				Ok(())
			})?;

			// Record delegation
			let delegation_info =
				DelegationInfo { pool_id, amount, delegated_at: current_block };
			Delegations::<T>::insert(&delegator, delegation_info);

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_add(amount);
			});

			Self::deposit_event(Event::Delegated { delegator, pool_id, amount });

			Ok(())
		}

		/// Remove delegation from a pool.
		///
		/// ## Parameters
		/// - `origin`: The delegator account
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::undelegate())]
		pub fn undelegate(origin: OriginFor<T>) -> DispatchResult {
			let delegator = ensure_signed(origin)?;

			let delegation =
				Delegations::<T>::get(&delegator).ok_or(Error::<T>::NotDelegating)?;
			let pool_id = delegation.pool_id;
			let amount = delegation.amount;

			// Update pool
			Pools::<T>::mutate(pool_id, |maybe_pool| {
				if let Some(pool) = maybe_pool {
					pool.total_stake = pool.total_stake.saturating_sub(amount);
					pool.delegator_count = pool.delegator_count.saturating_sub(1);
				}
			});

			// Remove from pool delegators
			PoolDelegators::<T>::mutate(pool_id, |delegators| {
				delegators.retain(|d| d != &delegator);
			});

			// Remove delegation record
			Delegations::<T>::remove(&delegator);

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_sub(amount);
			});

			Self::deposit_event(Event::Undelegated { delegator, pool_id, amount });

			Ok(())
		}

		/// Update pool commission. Operator only.
		///
		/// ## Parameters
		/// - `origin`: Must be pool operator
		/// - `pool_id`: Pool ID
		/// - `new_commission`: New commission rate in basis points
		#[pallet::call_index(19)]
		#[pallet::weight(T::WeightInfo::set_pool_commission())]
		pub fn set_pool_commission(
			origin: OriginFor<T>,
			pool_id: u32,
			new_commission: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(new_commission <= T::MaxPoolCommission::get(), Error::<T>::CommissionTooHigh);

			Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				ensure!(pool.operator == caller, Error::<T>::NotPoolOperator);

				pool.commission = new_commission;

				Ok(())
			})?;

			Self::deposit_event(Event::PoolCommissionUpdated { pool_id, new_commission });

			Ok(())
		}

		/// Close/deactivate a pool. Operator only. Pool must have no delegators.
		///
		/// ## Parameters
		/// - `origin`: Must be pool operator
		/// - `pool_id`: Pool ID
		#[pallet::call_index(20)]
		#[pallet::weight(T::WeightInfo::close_pool())]
		pub fn close_pool(origin: OriginFor<T>, pool_id: u32) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
			ensure!(pool.operator == caller, Error::<T>::NotPoolOperator);
			ensure!(pool.delegator_count == 0, Error::<T>::PoolHasDelegators);

			// Return operator stake
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_sub(pool.operator_stake);
			});

			// Remove pool
			Pools::<T>::remove(pool_id);

			Self::deposit_event(Event::PoolClosed { pool_id, operator: caller });

			Ok(())
		}

		/// Trigger era rotation and verifier selection. Can be called by anyone when due.
		/// Selects verifiers based on stake-weighted randomness.
		#[pallet::call_index(21)]
		#[pallet::weight(T::WeightInfo::rotate_era())]
		pub fn rotate_era(origin: OriginFor<T>) -> DispatchResult {
			ensure_signed(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();
			let last_era_block = LastEraBlock::<T>::get();
			let blocks_per_era = T::BlocksPerEra::get();

			// Check if era rotation is due
			ensure!(
				current_block >= last_era_block.saturating_add(blocks_per_era),
				Error::<T>::EraRotationNotDue
			);

			let new_era = CurrentEra::<T>::get().saturating_add(1);
			CurrentEra::<T>::put(new_era);
			LastEraBlock::<T>::put(current_block);

			// Select verifiers using stake-weighted selection
			let selected = Self::select_verifiers_for_era(new_era);
			let verifier_count = selected.len() as u32;

			// Store selected verifiers
			EraVerifiers::<T>::insert(
				new_era,
				BoundedVec::try_from(selected).unwrap_or_default(),
			);

			Self::deposit_event(Event::EraRotated { era: new_era, verifier_count });

			Ok(())
		}

		/// Distribute rewards for a completed period. Admin only.
		/// Distributes rewards to stakers and issuers based on their proportions.
		///
		/// ## Parameters
		/// - `origin`: Must be admin
		/// - `period`: Period number to distribute rewards for
		#[pallet::call_index(22)]
		#[pallet::weight(T::WeightInfo::distribute_rewards())]
		pub fn distribute_rewards(
			origin: OriginFor<T>,
			period: BlockNumberFor<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::ensure_admin(&caller)?;

			let reward_pool = RewardPool::<T>::get();
			ensure!(reward_pool > 0, Error::<T>::NoRewardsToClaim);

			let issuer_percent = T::IssuerRewardPercent::get();
			let issuer_share = reward_pool
				.saturating_mul(issuer_percent as u128)
				.saturating_div(10_000);
			let staker_share = reward_pool.saturating_sub(issuer_share);

			// Distribute to issuers based on period spending
			let period_total = PeriodTotalSpent::<T>::get(period);
			if period_total > 0 && issuer_share > 0 {
				// Iterate through authorized issuers and distribute based on spending
				// Note: In production, this should use pagination for large numbers
				for (issuer, is_authorized) in AuthorizedIssuers::<T>::iter() {
					if is_authorized {
						let record = IssuerDailyRecords::<T>::get(period, &issuer);
						if record.points_spent > 0 {
							let issuer_reward = issuer_share
								.saturating_mul(record.points_spent)
								.saturating_div(period_total);
							PendingIssuerRewards::<T>::mutate(&issuer, |pending| {
								*pending = pending.saturating_add(issuer_reward);
							});
						}
					}
				}
			}

			// Distribute to stakers based on stake
			let total_staked = TotalStaked::<T>::get();
			if total_staked > 0 && staker_share > 0 {
				for (staker, stake_info) in Stakes::<T>::iter() {
					if stake_info.amount > 0 {
						let staker_reward = staker_share
							.saturating_mul(stake_info.amount)
							.saturating_div(total_staked);
						PendingStakerRewards::<T>::mutate(&staker, |pending| {
							*pending = pending.saturating_add(staker_reward);
						});
					}
				}
			}

			// Clear reward pool
			RewardPool::<T>::put(0u128);

			Self::deposit_event(Event::RewardsDistributed {
				period,
				staker_rewards: staker_share,
				issuer_rewards: issuer_share,
			});

			Ok(())
		}

		/// Claim pending rewards (for stakers or issuers).
		#[pallet::call_index(23)]
		#[pallet::weight(T::WeightInfo::claim_rewards())]
		pub fn claim_rewards(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let staker_reward = PendingStakerRewards::<T>::get(&caller);
			let issuer_reward = PendingIssuerRewards::<T>::get(&caller);
			let total_reward = staker_reward.saturating_add(issuer_reward);

			ensure!(total_reward > 0, Error::<T>::NoRewardsToClaim);

			// Clear pending rewards
			PendingStakerRewards::<T>::remove(&caller);
			PendingIssuerRewards::<T>::remove(&caller);

			Self::deposit_event(Event::RewardClaimed { account: caller, amount: total_reward });

			Ok(())
		}

		/// Add additional stake to existing stake.
		///
		/// ## Parameters
		/// - `origin`: The staker account
		/// - `amount`: Additional amount to stake
		#[pallet::call_index(24)]
		#[pallet::weight(T::WeightInfo::increase_stake())]
		pub fn increase_stake(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			let mut new_total: u128 = 0;

			Stakes::<T>::try_mutate(&staker, |maybe_info| -> DispatchResult {
				let info = maybe_info.as_mut().ok_or(Error::<T>::NotStaker)?;
				info.amount = info.amount.saturating_add(amount);
				new_total = info.amount;
				Ok(())
			})?;

			// Update total staked
			TotalStaked::<T>::mutate(|total| {
				*total = total.saturating_add(amount);
			});

			Self::deposit_event(Event::StakeIncreased { staker, amount, new_total });

			Ok(())
		}
	}

	// ============================================================================
	// INTERNAL HELPER FUNCTIONS
	// ============================================================================

	impl<T: Config> Pallet<T> {
		/// Check if an account is the admin
		pub fn is_admin(account: &T::AccountId) -> bool {
			Admin::<T>::get().as_ref().is_some_and(|admin| admin == account)
		}

		/// Ensure the caller is the admin
		fn ensure_admin(account: &T::AccountId) -> DispatchResult {
			ensure!(Self::is_admin(account), Error::<T>::NotAdmin);
			Ok(())
		}

		/// Remove expired batches from a user's batch list.
		/// This updates both the batch list and the total points.
		/// Returns the amount of points that expired.
		fn remove_expired_batches_internal(
			user: &T::AccountId,
			batches: &mut BoundedVec<PointBatch<BlockNumberFor<T>>, T::MaxPointBatches>,
			current_block: BlockNumberFor<T>,
		) -> u128 {
			// Calculate how many points are expiring
			let expired_amount: u128 = batches
				.iter()
				.filter(|b| b.expires_at_block <= current_block)
				.map(|b| b.remaining_points)
				.sum();

			let batches_before = batches.len();

			// Remove expired batches
			batches.retain(|batch| batch.expires_at_block > current_block);

			let batches_removed = (batches_before - batches.len()) as u32;

			// Update total points if any expired
			if expired_amount > 0 {
				TotalPoints::<T>::mutate(user, |total| {
					*total = total.saturating_sub(expired_amount);
				});

				// Emit event
				Self::deposit_event(Event::PointsExpired {
					user: user.clone(),
					amount_expired: expired_amount,
					batches_removed,
				});
			}

			expired_amount
		}

		/// Get the total non-expired points for a user at the current block.
		/// This recalculates from batches, useful for verification.
		pub fn get_available_points(user: &T::AccountId) -> u128 {
			let current_block = frame_system::Pallet::<T>::block_number();
			UserPoints::<T>::get(user)
				.iter()
				.filter(|b| b.expires_at_block > current_block)
				.map(|b| b.remaining_points)
				.sum()
		}

		/// Get detailed point information for a user.
		/// Returns a vector of (remaining_points, expires_at_block, travel_type) tuples.
		pub fn get_point_details(
			user: &T::AccountId,
		) -> Vec<(u128, BlockNumberFor<T>, TravelType)> {
			let current_block = frame_system::Pallet::<T>::block_number();
			UserPoints::<T>::get(user)
				.iter()
				.filter(|b| b.expires_at_block > current_block)
				.map(|b| (b.remaining_points, b.expires_at_block, b.travel_type.clone()))
				.collect()
		}

		/// Get the current reward period number based on block number.
		/// Periods are used for tracking issuer rewards and staker distributions.
		///
		/// Note: If BlocksPerRewardPeriod is configured as zero, falls back to
		/// using the current block number as the period (each block is its own period).
		/// This should be avoided in production configurations.
		pub fn current_period() -> BlockNumberFor<T> {
			let current_block = frame_system::Pallet::<T>::block_number();
			let blocks_per_period = T::BlocksPerRewardPeriod::get();
			if blocks_per_period.is_zero() {
				// Fallback: treat each block as its own period
				// This is not recommended for production use
				return current_block;
			}
			current_block / blocks_per_period
		}

		/// Internal function to spend points (used by mint_ticket and other internal operations)
		/// This tracks spending for issuer reward distribution
		fn spend_points_internal(
			user: &T::AccountId,
			amount: u128,
			issuer: &T::AccountId,
		) -> DispatchResult {
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			let current_block = frame_system::Pallet::<T>::block_number();
			let mut remaining_to_spend = amount;

			UserPoints::<T>::try_mutate(user, |batches| -> DispatchResult {
				Self::remove_expired_batches_internal(user, batches, current_block);

				let available: u128 = batches.iter().map(|b| b.remaining_points).sum();
				ensure!(available >= amount, Error::<T>::InsufficientPoints);

				for batch in batches.iter_mut() {
					if remaining_to_spend == 0 {
						break;
					}
					let deduction = remaining_to_spend.min(batch.remaining_points);
					batch.remaining_points = batch
						.remaining_points
						.checked_sub(deduction)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;
					remaining_to_spend = remaining_to_spend
						.checked_sub(deduction)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;
				}

				batches.retain(|b| b.remaining_points > 0);
				Ok(())
			})?;

			let new_balance =
				TotalPoints::<T>::try_mutate(user, |total| -> Result<u128, DispatchError> {
					*total = total.checked_sub(amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
					Ok(*total)
				})?;

			// Track spending for issuer reward distribution
			let period = Self::current_period();
			IssuerDailyRecords::<T>::mutate(period, issuer, |record| {
				record.points_spent = record.points_spent.saturating_add(amount);
				record.transaction_count = record.transaction_count.saturating_add(1);
			});
			PeriodTotalSpent::<T>::mutate(period, |total| {
				*total = total.saturating_add(amount);
			});

			Self::deposit_event(Event::PointsSpent {
				user: user.clone(),
				amount_spent: amount,
				remaining_balance: new_balance,
				issuer: issuer.clone(),
			});

			Ok(())
		}

		// ============================================================================
		// CONTRACT INTERFACE FUNCTIONS
		// ============================================================================

		/// Contract interface: Award points to a user
		/// This is a helper function that can be called by smart contracts
		pub fn contract_award_points(
			issuer: T::AccountId,
			recipient: T::AccountId,
			amount: u128,
			travel_type: TravelType,
			custom_expiration: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			ensure!(AuthorizedIssuers::<T>::get(&issuer), Error::<T>::NotAuthorizedIssuer);

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			let current_block = frame_system::Pallet::<T>::block_number();
			let expiration_period = custom_expiration.unwrap_or(T::DefaultExpirationPeriod::get());
			let expires_at_block = current_block.saturating_add(expiration_period);

			let new_batch = PointBatch {
				earned_at_block: current_block,
				expires_at_block,
				remaining_points: amount,
				travel_type: travel_type.clone(),
			};

			UserPoints::<T>::try_mutate(&recipient, |batches| -> DispatchResult {
				Self::remove_expired_batches_internal(&recipient, batches, current_block);
				batches.try_push(new_batch).map_err(|_| Error::<T>::TooManyBatches)?;
				batches.sort_by(|a, b| a.expires_at_block.cmp(&b.expires_at_block));
				Ok(())
			})?;

			TotalPoints::<T>::try_mutate(&recipient, |total| -> DispatchResult {
				*total = total.checked_add(amount).ok_or(Error::<T>::ArithmeticOverflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::PointsEarned {
				recipient,
				amount,
				expires_at_block,
				travel_type,
			});

			Ok(())
		}

		/// Contract interface: Check balance for a user
		pub fn contract_check_balance(user: &T::AccountId) -> u128 {
			Self::get_available_points(user)
		}

		/// Contract interface: Check if an account is an authorized issuer
		pub fn contract_is_authorized_issuer(account: &T::AccountId) -> bool {
			AuthorizedIssuers::<T>::get(account)
		}

		/// Get issuer spending record for a period
		pub fn get_issuer_period_record(
			period: BlockNumberFor<T>,
			issuer: &T::AccountId,
		) -> IssuerDailyRecord {
			IssuerDailyRecords::<T>::get(period, issuer)
		}

		/// Get total points spent in a period
		pub fn get_period_total_spent(period: BlockNumberFor<T>) -> u128 {
			PeriodTotalSpent::<T>::get(period)
		}

		/// Get ticket by ID
		pub fn get_ticket(ticket_id: u128) -> Option<Ticket<T::AccountId, BlockNumberFor<T>>> {
			Tickets::<T>::get(ticket_id)
		}

		/// Get all tickets owned by a user
		pub fn get_user_tickets(user: &T::AccountId) -> Vec<u128> {
			UserTickets::<T>::get(user).to_vec()
		}

		/// Get stake info for a staker
		pub fn get_stake_info(staker: &T::AccountId) -> Option<StakeInfo<BlockNumberFor<T>>> {
			Stakes::<T>::get(staker)
		}

		/// Get list of all stakers
		pub fn get_all_stakers() -> Vec<T::AccountId> {
			StakerList::<T>::get().to_vec()
		}

		// ============================================================================
		// ADVANCED STAKING HELPER FUNCTIONS
		// ============================================================================

		/// Select verifiers for a new era using stake-weighted selection.
		/// Uses a deterministic pseudo-random selection based on block hash and stakes.
		fn select_verifiers_for_era(era: u32) -> Vec<T::AccountId> {
			let max_verifiers = T::VerifiersPerEra::get() as usize;
			let stakers = StakerList::<T>::get();

			if stakers.is_empty() {
				return Vec::new();
			}

			// Build list of (staker, stake_amount) pairs
			let mut candidates: Vec<(T::AccountId, u128)> = Vec::new();
			let mut total_stake: u128 = 0;

			for staker in stakers.iter() {
				if let Some(info) = Stakes::<T>::get(staker) {
					if info.amount > 0 {
						candidates.push((staker.clone(), info.amount));
						total_stake = total_stake.saturating_add(info.amount);
					}
				}
			}

			if candidates.is_empty() || total_stake == 0 {
				return Vec::new();
			}

			// Deterministic stake-weighted selection
			// Sort by stake (descending), then by encoded account for determinism with equal stakes
			candidates.sort_by(|a, b| {
				match b.1.cmp(&a.1) {
					core::cmp::Ordering::Equal => {
						// Use encoded account as deterministic tie-breaker
						let a_encoded = a.0.encode();
						let b_encoded = b.0.encode();
						a_encoded.cmp(&b_encoded)
					}
					other => other,
				}
			});

			let mut selected: Vec<T::AccountId> = Vec::new();

			for (staker, _stake) in candidates.iter().take(max_verifiers) {
				selected.push(staker.clone());

				// Mark as verifier
				Stakes::<T>::mutate(staker, |maybe_info| {
					if let Some(info) = maybe_info {
						info.is_verifier = true;
					}
				});

				// Emit event
				Self::deposit_event(Event::VerifierSelected {
					era,
					verifier: staker.clone(),
				});
			}

			// Clear verifier status for non-selected stakers
			for (staker, _) in candidates.iter().skip(max_verifiers) {
				Stakes::<T>::mutate(staker, |maybe_info| {
					if let Some(info) = maybe_info {
						info.is_verifier = false;
					}
				});
			}

			selected
		}

		/// Get pool information by ID
		pub fn get_pool(pool_id: u32) -> Option<StakingPool<T::AccountId, BlockNumberFor<T>>> {
			Pools::<T>::get(pool_id)
		}

		/// Get delegation info for an account
		pub fn get_delegation(
			account: &T::AccountId,
		) -> Option<DelegationInfo<BlockNumberFor<T>>> {
			Delegations::<T>::get(account)
		}

		/// Get all delegators for a pool
		pub fn get_pool_delegators(pool_id: u32) -> Vec<T::AccountId> {
			PoolDelegators::<T>::get(pool_id).to_vec()
		}

		/// Get unbonding requests for an account
		pub fn get_unbonding_requests(
			account: &T::AccountId,
		) -> Vec<UnbondingInfo<BlockNumberFor<T>>> {
			UnbondingRequests::<T>::get(account).to_vec()
		}

		/// Get slash records for an account
		pub fn get_slash_records(
			account: &T::AccountId,
		) -> Vec<SlashRecord<BlockNumberFor<T>>> {
			SlashRecords::<T>::get(account).to_vec()
		}

		/// Get current era verifiers
		pub fn get_current_verifiers() -> Vec<T::AccountId> {
			let era = CurrentEra::<T>::get();
			EraVerifiers::<T>::get(era).to_vec()
		}

		/// Check if an account is a verifier for the current era
		pub fn is_current_verifier(account: &T::AccountId) -> bool {
			let era = CurrentEra::<T>::get();
			EraVerifiers::<T>::get(era).contains(account)
		}

		/// Get pending rewards for an account (staker + issuer)
		pub fn get_pending_rewards(account: &T::AccountId) -> u128 {
			PendingStakerRewards::<T>::get(account)
				.saturating_add(PendingIssuerRewards::<T>::get(account))
		}
	}
}
