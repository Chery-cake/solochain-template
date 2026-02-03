# Travel Points Pallet

A Substrate FRAME pallet for managing travel loyalty points, similar to airline mileage programs, with advanced Proof-of-Stake features.

## Overview

This pallet provides a blockchain-based travel points system that can be used for various types of travel loyalty programs (airlines, trains, buses, etc.). Key features include:

### Core Features
- **Point Batches**: Points are stored in batches with expiration tracking
- **FIFO Deduction**: When spending points, oldest points are used first to prevent expiration
- **Smart Contract Interface**: Authorized issuers (which could be smart contracts) can award points
- **Multi-Travel Support**: Supports different travel types (Airline, Train, Bus, Other)
- **Expiration Management**: Automatic cleanup of expired points
- **NFT Tickets**: Store travel tickets and bonuses as NFTs with detailed metadata

### Advanced Staking Features
- **Slashing Mechanism**: Configurable penalties for misbehaving or offline stakers/verifiers
- **Unbonding Period**: Lock period between unstake request and fund withdrawal
- **Delegation and Pool Staking**: Users can delegate stake to validator pools
- **Era-based Verifier Selection**: Stake-weighted verifier selection per era
- **Issuer Reward Retention**: Issuers earn rewards based on point redemptions through them

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

### Staking and Verifiers
- **Stakers** can stake tokens to earn rewards and potentially become verifiers
- **Verifiers** are selected each era based on stake-weighted randomness
- Only selected verifiers perform verification tasks and receive verification rewards
- Misbehaving verifiers face slashing penalties

### Slashing
Configurable slashing penalties for:
- **Offline**: 5% (configurable) for validators that fail to perform duties
- **Invalid Verification**: 10% (configurable) for submitting invalid verifications  
- **Malicious Behavior**: Up to 100% (configurable) for provably malicious actions

### Unbonding Period
- When unstaking, tokens enter an unbonding period (default: ~7 days)
- During unbonding, tokens are locked and non-transferable
- After the period ends, tokens can be withdrawn
- Unbonding can be cancelled to re-stake tokens

### Delegation and Pools
- **Pool Operators**: Create pools with configurable commission rates
- **Delegators**: Stake tokens in pools to share rewards (and slashing risk)
- Commission is taken from delegator rewards before distribution
- Pools can be closed when they have no active delegators

### Issuer Reward Retention
- Issuers receive a share of staking rewards based on point spending through them
- Configurable percentage (default: 20%) of rewards go to issuers
- Rewards are distributed proportionally based on period spending metrics
- This incentivizes issuers to participate in the network and drive adoption

## Storage

| Storage Item | Description |
|-------------|-------------|
| `UserPoints` | Maps account IDs to their point batches |
| `TotalPoints` | Cached total balance per user |
| `AuthorizedIssuers` | Accounts authorized to issue points |
| `Admin` | The admin account that manages issuers |
| `Tickets` | NFT tickets by ID |
| `Stakes` | Staking information per staker |
| `Pools` | Staking pools by ID |
| `Delegations` | Delegation information per delegator |
| `UnbondingRequests` | Pending unbonding requests per staker |
| `EraVerifiers` | Selected verifiers per era |
| `SlashRecords` | Historical slash records per account |

## Extrinsics

### Core Point Functions
| Extrinsic | Description |
|-----------|-------------|
| `award_points` | Award points to a user (issuer only) |
| `spend_points` | Spend points with issuer tracking |
| `cleanup_expired` | Remove expired point batches |

### Admin Functions
| Extrinsic | Description |
|-----------|-------------|
| `authorize_issuer` | Authorize an account to issue points |
| `revoke_issuer` | Revoke issuer authorization |
| `set_admin` | Change the admin account |
| `slash_staker` | Slash a misbehaving staker |
| `distribute_rewards` | Distribute rewards for a period |

### NFT Ticket Functions
| Extrinsic | Description |
|-----------|-------------|
| `mint_ticket` | Mint a new ticket NFT |
| `redeem_ticket` | Redeem/use a ticket |
| `transfer_ticket` | Transfer ticket to another account |

### Staking Functions
| Extrinsic | Description |
|-----------|-------------|
| `stake` | Stake tokens to become a staker |
| `unstake` | Unstake all tokens (legacy, immediate) |
| `increase_stake` | Add more stake to existing stake |
| `request_unbond` | Request unbonding with lock period |
| `withdraw_unbonded` | Withdraw tokens after unbonding period |
| `cancel_unbonding` | Cancel unbonding and re-stake |

### Pool Functions
| Extrinsic | Description |
|-----------|-------------|
| `create_pool` | Create a new staking pool |
| `delegate` | Delegate stake to a pool |
| `undelegate` | Remove delegation from pool |
| `set_pool_commission` | Update pool commission rate |
| `close_pool` | Close a pool (no delegators) |

### Era and Rewards Functions
| Extrinsic | Description |
|-----------|-------------|
| `rotate_era` | Trigger era rotation and verifier selection |
| `claim_rewards` | Claim pending staker/issuer rewards |
| `add_to_reward_pool` | Add tokens to reward pool |

## Configuration

```rust
impl pallet_travel_points::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_travel_points::weights::SubstrateWeight<Runtime>;
    
    // Point Configuration
    type MaxPointBatches = ConstU32<100>;
    type DefaultExpirationPeriod = ConstU32<5256000>; // ~1 year
    
    // Ticket Configuration
    type MaxTicketsPerUser = ConstU32<100>;
    
    // Basic Staking Configuration
    type MaxStakers = ConstU32<1000>;
    type MinStakeAmount = ConstU128<1000>;
    type StakerRewardPercent = ConstU32<3000>; // 30%
    type BlocksPerRewardPeriod = ConstU32<14400>; // ~1 day
    
    // Advanced Staking Configuration
    type UnbondingPeriod = ConstU32<100800>; // ~7 days
    type OfflineSlashPercent = ConstU32<500>; // 5%
    type InvalidVerificationSlashPercent = ConstU32<1000>; // 10%
    type MaliciousSlashPercent = ConstU32<10000>; // 100%
    
    // Pool Configuration
    type MaxPools = ConstU32<100>;
    type MaxDelegatorsPerPool = ConstU32<100>;
    type MinPoolOperatorStake = ConstU128<10000>;
    type MaxPoolCommission = ConstU32<3000>; // 30%
    
    // Era Configuration
    type VerifiersPerEra = ConstU32<21>;
    type BlocksPerEra = ConstU32<14400>; // ~1 day
    
    // Issuer Rewards
    type IssuerRewardPercent = ConstU32<2000>; // 20%
    type MaxUnbondingRequests = ConstU32<32>;
}
```

## Events

### Core Events
| Event | Description |
|-------|-------------|
| `PointsEarned` | Points were awarded to a user |
| `PointsSpent` | Points were spent (with issuer tracking) |
| `PointsExpired` | Points expired for a user |
| `IssuerAuthorized` | An account was authorized to issue points |
| `IssuerRevoked` | An account's authorization was revoked |
| `AdminChanged` | The admin account was changed |

### Staking Events
| Event | Description |
|-------|-------------|
| `Staked` | Tokens were staked |
| `Unstaked` | Tokens were unstaked |
| `StakeIncreased` | Additional stake added |
| `Slashed` | A staker was slashed |
| `UnbondingInitiated` | Unbonding period started |
| `UnbondingWithdrawn` | Unbonded tokens withdrawn |
| `UnbondingCancelled` | Unbonding cancelled, tokens re-staked |

### Pool Events
| Event | Description |
|-------|-------------|
| `PoolCreated` | New staking pool created |
| `Delegated` | Stake delegated to pool |
| `Undelegated` | Delegation withdrawn |
| `PoolCommissionUpdated` | Pool commission changed |
| `PoolClosed` | Pool was closed |

### Era Events
| Event | Description |
|-------|-------------|
| `EraRotated` | New era started, verifiers rotated |
| `VerifierSelected` | Verifier selected for era |
| `RewardsDistributed` | Rewards distributed for period |
| `RewardClaimed` | Rewards claimed by account |

## Example Usage

```rust
// Admin authorizes a booking system contract
TravelPoints::authorize_issuer(admin_origin, booking_contract)?;

// Booking system awards points for a flight
TravelPoints::award_points(
    Origin::signed(booking_contract),
    passenger,
    1000, // points
    TravelType::Airline,
    None, // use default expiration
)?;

// User spends points with an issuer (redeems for service)
TravelPoints::spend_points(
    Origin::signed(passenger),
    500, // points to spend
    booking_contract, // issuer to track for rewards
)?;

// User stakes tokens
TravelPoints::stake(Origin::signed(staker), 10000)?;

// User requests unbonding
TravelPoints::request_unbond(Origin::signed(staker), 5000)?;

// After unbonding period, withdraw
TravelPoints::withdraw_unbonded(Origin::signed(staker))?;

// Create a staking pool
TravelPoints::create_pool(
    Origin::signed(operator),
    10000, // initial stake
    1000,  // 10% commission
)?;

// Delegate to pool
TravelPoints::delegate(Origin::signed(delegator), 0, 5000)?;

// Distribute rewards (admin)
TravelPoints::distribute_rewards(admin_origin, period)?;

// Claim rewards
TravelPoints::claim_rewards(Origin::signed(account))?;
```

## Security Considerations

- **Slashing**: Slash amounts are configurable and applied immediately
- **Unbonding**: Protects against rapid stake withdrawal during attacks
- **Pool Commission**: Maximum commission is capped to protect delegators
- **Era Rotation**: Deterministic verifier selection prevents manipulation
- **Permission Checks**: Admin-only functions protected by origin checks

## References

- Substrate Staking Pallet
- Substrate Nomination Pools
- Cosmos SDK Staking Module
- Travel rewards/loyalty platform best practices

## License

Unlicense
