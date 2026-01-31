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
	#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
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
	pub type TotalPoints<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	/// Stores which accounts are authorized to issue points.
	/// These could be smart contracts or admin accounts.
	#[pallet::storage]
	#[pallet::getter(fn authorized_issuers)]
	pub type AuthorizedIssuers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	/// Stores the admin/root account that can manage authorized issuers.
	/// This is set during genesis or by sudo.
	#[pallet::storage]
	#[pallet::getter(fn admin)]
	pub type Admin<T: Config> = StorageValue<_, T::AccountId>;

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

		/// Points were spent/used by a user
		/// [user, amount_spent, remaining_balance]
		PointsSpent {
			/// The account that spent the points
			user: T::AccountId,
			/// The amount of points spent
			amount_spent: u128,
			/// The remaining point balance after spending
			remaining_balance: u128,
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
			ensure!(
				AuthorizedIssuers::<T>::get(&issuer),
				Error::<T>::NotAuthorizedIssuer
			);

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
				batches
					.try_push(new_batch)
					.map_err(|_| Error::<T>::TooManyBatches)?;

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
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::spend_points())]
		pub fn spend_points(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			let user = ensure_signed(origin)?;

			// Amount must be greater than zero
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

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
			let new_balance = TotalPoints::<T>::try_mutate(&user, |total| -> Result<u128, DispatchError> {
				*total = total.checked_sub(amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
				Ok(*total)
			})?;

			// Emit event
			Self::deposit_event(Event::PointsSpent {
				user,
				amount_spent: amount,
				remaining_balance: new_balance,
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

			ensure!(
				!AuthorizedIssuers::<T>::get(&issuer),
				Error::<T>::AlreadyAuthorized
			);

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

			ensure!(
				AuthorizedIssuers::<T>::get(&issuer),
				Error::<T>::NotAuthorized
			);

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

			Self::deposit_event(Event::AdminChanged {
				old_admin,
				new_admin,
			});
			Ok(())
		}
	}

	// ============================================================================
	// INTERNAL HELPER FUNCTIONS
	// ============================================================================

	impl<T: Config> Pallet<T> {
		/// Check if an account is the admin
		pub fn is_admin(account: &T::AccountId) -> bool {
			Admin::<T>::get()
				.as_ref()
				.is_some_and(|admin| admin == account)
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
	}
}
