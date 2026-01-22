# Travel Points Pallet

A Substrate FRAME pallet for managing travel loyalty points, similar to airline mileage programs.

## Overview

This pallet provides a blockchain-based travel points system that can be used for various types of travel loyalty programs (airlines, trains, buses, etc.). Key features include:

- **Point Batches**: Points are stored in batches with expiration tracking
- **FIFO Deduction**: When spending points, oldest points are used first to prevent expiration
- **Smart Contract Interface**: Authorized issuers (which could be smart contracts) can award points
- **Multi-Travel Support**: Supports different travel types (Airline, Train, Bus, Other)
- **Expiration Management**: Automatic cleanup of expired points

## Key Concepts

### Point Batches
Each time points are awarded, a new "batch" is created containing:
- **earned_at_block**: When the points were earned
- **expires_at_block**: When the points will expire
- **remaining_points**: How many points are left in this batch
- **travel_type**: What type of travel earned these points

### FIFO (First In, First Out) Deduction
When a user spends points, the system automatically deducts from the oldest batches first. This ensures users don't lose points to expiration when they have newer points available.

### Authorized Issuers
Only authorized accounts can issue points. This could be:
- Admin accounts
- Smart contracts (for automatic point allocation from booking systems)
- Partner service accounts

## Storage

| Storage Item | Description |
|-------------|-------------|
| `UserPoints` | Maps account IDs to their point batches |
| `TotalPoints` | Cached total balance per user |
| `AuthorizedIssuers` | Accounts authorized to issue points |
| `Admin` | The admin account that manages issuers |

## Extrinsics

### `award_points`
Award points to a user. Only callable by authorized issuers.
- `recipient`: Account to receive points
- `amount`: Number of points
- `travel_type`: Type of travel (Airline, Train, Bus, Other)
- `custom_expiration`: Optional custom expiration period

### `spend_points`
Spend points from the caller's balance (FIFO).
- `amount`: Number of points to spend

### `cleanup_expired`
Remove expired point batches from a user's storage.
- `user`: Account to clean up

### `authorize_issuer`
Authorize an account to issue points (admin only).
- `issuer`: Account to authorize

### `revoke_issuer`
Revoke an account's issuer authorization (admin only).
- `issuer`: Account to revoke

### `set_admin`
Change the admin account (admin or root only).
- `new_admin`: New admin account

## Configuration

```rust
impl pallet_travel_points::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_travel_points::weights::SubstrateWeight<Runtime>;
    // Maximum point batches per user
    type MaxPointBatches = ConstU32<100>;
    // Default expiration period in blocks
    type DefaultExpirationPeriod = ConstU32<5256000>; // ~1 year
}
```

## Events

| Event | Description |
|-------|-------------|
| `PointsEarned` | Points were awarded to a user |
| `PointsSpent` | Points were spent by a user |
| `PointsExpired` | Points expired for a user |
| `IssuerAuthorized` | An account was authorized to issue points |
| `IssuerRevoked` | An account's authorization was revoked |
| `AdminChanged` | The admin account was changed |

## Smart Contract Integration

The pallet provides functions for smart contract integration:

```rust
// Award points from a smart contract
TravelPoints::contract_award_points(
    issuer_account,
    recipient_account,
    amount,
    TravelType::Airline,
    custom_expiration,
);

// Check a user's balance
let balance = TravelPoints::contract_check_balance(&user_account);

// Check if an account is an authorized issuer
let is_authorized = TravelPoints::contract_is_authorized_issuer(&account);
```

## Example Usage

```rust
// Admin authorizes a booking system contract
TravelPoints::authorize_issuer(origin, booking_contract)?;

// Booking system awards points for a flight
TravelPoints::award_points(
    Origin::signed(booking_contract),
    passenger,
    1000, // points
    TravelType::Airline,
    None, // use default expiration
)?;

// User redeems points for a reward
TravelPoints::spend_points(
    Origin::signed(passenger),
    500, // points to spend
)?;
```

## Future Enhancements

This pallet is designed to be extended with:
- **NFT Integration**: Travel tickets/passes as NFTs
- **Point Transfers**: Allow users to transfer points between accounts
- **Tier System**: VIP tiers based on point accumulation
- **Partner Programs**: Multi-partner point earning/spending
- **Point Conversion**: Convert between different travel point types

## License

Unlicense
