# Harvesting interest and emissions

The backstop manager needs finer control over harvesting interest and
emissions from the backstop. The manager should be able to take a share
of the earnings on the backstop positions, set by the owner, without being
able to touch the deposited principal.

## How earnings arrive

Earnings reach a backstop position in two ways:

*  **Interest** is added to a tier with `donate`. It raises the number of
   LP tokens each share is worth; the position's share count does not
   change.
*  **Emissions** claimed with `b_claim` are converted to LP tokens and
   deposited back into the backstop, so they add shares to the position.

Because interest does not add shares, all accounting is done in **LP
tokens**, not shares. The contract records how many LP tokens it has
deposited into each position (the principal) and compares that with what
the position is currently worth.

## Example

```
t0: owner deposits 100 LP tokens; principal = 100
t1: interest raises the position's value to 101 LP tokens
t2: emissions are claimed, adding shares; value = 102 LP tokens
t3: manager queues shares worth 10 LP tokens for withdrawal
t4: after the queue period, the shares are withdrawn to the contract,
    returning 10 LP tokens
      earnings before withdrawal = 102 - 100 = 2
      2 of the 10 tokens are earnings, 8 are principal
      principal = 100 - 8 = 92
      with harvest_bps = 10000, 2 tokens become harvestable
t5: manager calls `harvest` and receives 2 LP tokens
```

## Accounting

### Position value

A position is identified by `(backstop, pool, tier)`. Each tier uses its
own LP token, so principal is tracked per position and never summed across
positions.

The position is valued during `b_withdraw` from the tier's exact token and
share totals immediately before withdrawal:

```
user            = backstop.user_balance(tier, pool, contract)   // before withdrawing
position_shares = user.shares + sum(user.q4w[i].amount)
tier_data       = backstop.pool_data(pool).tiers[tier]
value           = position_shares * tier_data.tokens / tier_data.shares
returned        = backstop.withdraw(tier, contract, pool, withdrawn_shares, contract)
```

Queued shares are included because they still belong to the position and
are still exposed to losses. Division rounds down, which can undervalue
the position by less than one LP-token base unit, in the owner's favor.

Using the tier totals avoids extrapolating the rounded result of a small
withdrawal across the full position. Because the v3 backstop exposes those
totals through `pool_data`, `b_withdraw` requires that complete backstop-data
query to succeed before it changes the position.

### Principal

| Action | Effect on principal |
|---|---|
| `b_deposit(amount)` | `principal += amount` |
| `b_claim` | none (emissions count as earnings) |
| `b_queue_withdrawal`, `b_dequeue_withdrawal` | none |
| `b_withdraw` | reduced by the principal part of the withdrawal (below) |
| `set_principal` (owner) | set directly |

Principal starts at 0 for a position the contract has no record of.

### Withdrawals: earnings first

`b_withdraw` applies whether the owner or the manager calls it. Before
withdrawing, the contract values the position; after withdrawing it knows
how many tokens `w` were returned.

```
earnings        = max(0, value - principal)
earnings_part   = min(w, earnings)
principal_part  = w - earnings_part
principal      -= principal_part
manager_part    = earnings_part * harvest_bps / 10000   (rounded down)
harvestable[token] += manager_part
```

Withdrawn tokens are counted as earnings first, so the manager can queue
and withdraw exactly the earnings without touching principal. The owner's
share of the earnings (`earnings_part - manager_part`) stays in the
contract as the owner's funds, like any other withdrawn tokens.

### Losses

Backstop tiers can be drawn on to cover bad debt, the first-loss tier
first. If the position's value falls below principal, earnings are 0 and
nothing becomes harvestable until the value climbs back above principal.
Principal is not written down after a loss, so it acts as a high-water
mark: the manager earns only on value above what was deposited.

### Harvestable balances are not reserved

`harvestable[token]` is a record of what the manager is owed, not a
reserved balance. Other actions can spend the LP tokens that back it:

*  the owner's `transfer_token` and a manager's `refund_token`,
*  `c_exit_pool`, which converts LP tokens to the underlying tokens,
*  `b_deposit`, which puts them back into the backstop. Re-depositing
   harvestable tokens adds them to principal, so the manager can choose
   to reinvest its earnings, but they stop being harvestable.

`harvest` therefore pays the smaller of the harvestable amount and the
contract's balance of that token. The owner can correct the record with
`set_harvestable`.

### Earnings belong to the manager role

Harvestable balances are owed to whoever is manager at the time of
`harvest`, not to a particular address. If the owner changes the manager
with `set_manager`, any unharvested amount goes to the new manager. An
owner who wants to settle with the outgoing manager first should have them
call `harvest` before the change, or reset the balance with
`set_harvestable`.

### Changing `harvest_bps`

`harvest_bps` is applied when earnings are realized by `b_withdraw`, not
when they accrue. A change to `harvest_bps` applies to all earnings
withdrawn after the change, including earnings that accrued before it.
Amounts already added to `harvestable` are unaffected.

### Trust assumptions

The manager realizes earnings itself with `b_withdraw`, which requires
scope High. This is the expected setup for a hands-off owner.

Harvesting guarantees that the manager cannot transfer principal to
itself. It does not guarantee that principal keeps its value: a High
manager can also call `c_join_pool` and `c_exit_pool`, and a join or exit
with loose limits (`max_amounts_in` too high, `min_amounts_out` too low)
can lose value to slippage or a sandwich. This risk exists without
harvesting and is accepted: an owner who gives a manager High scope
trusts them with Comet trades.

## Functions

### Owner

*  `set_harvest_bps(bps: u32)`: sets the manager's share of earnings, in
   basis points (0 to 10000). Defaults to 0, so harvesting is off until
   the owner turns it on. Fails with `InvalidBps` if `bps > 10000`.
*  `set_principal(backstop: Address, pool: Address, tier: BackstopTier, amount: i128)`:
   sets the principal for a position. Used to set up positions the
   contract already holds, and to correct principal after the position
   changes without going through `b_withdraw` (e.g. the backstop's
   `force_withdrawal` or `force_queue_withdrawal`). Fails with
   `NegativeAmountError` if `amount < 0`.
*  `set_harvestable(token: Address, amount: i128)`: sets the harvestable
   balance for a token. Fails with `NegativeAmountError` if `amount < 0`.

### Manager

*  `harvest(from: Address, token: Address) -> i128`: transfers
   `min(harvestable[token], contract balance of token)` from the contract
   to the manager's address, reduces `harvestable[token]` by that amount,
   and returns it. Requires `from.require_auth()`; `from` must be the
   manager or the owner (scope `Low`), and tokens always go to the
   manager. No particular scope is needed beyond `Low`: `harvest_bps` is
   what controls whether the manager earns anything.

### Read-only

*  `harvest_bps() -> u32`
*  `principal(backstop: Address, pool: Address, tier: BackstopTier) -> i128`
*  `harvestable(token: Address) -> i128`

## Storage

| Key | Value | Storage |
|---|---|---|
| `"HarvBps"` | `u32` | instance |
| `DataKey::Principal(PositionKey { backstop, pool, tier })` | `i128` | persistent |
| `DataKey::Harvest(token)` | `i128` | persistent |

Persistent entries have their TTL extended whenever they are read or
written. The number of positions is bounded by the valid backstop and pool
lists (at most 4 × 4 × 3), but entries stay after a backstop or pool is
removed from those lists, so instance storage is not used for them.

## Errors

| Error | Meaning |
|---|---|
| `InvalidBps` (107) | `harvest_bps` greater than 10000 |

`NegativeAmountError` (8) already exists.

## Events

| Event | Topics | Data |
|---|---|---|
| `harvest_bps_set` | `("harvest_bps_set")` | `bps` |
| `principal_set` | `("principal_set", backstop, pool)` | `(tier, amount)` |
| `harvestable_set` | `("harvestable_set", token)` | `amount` |
| `harvest` | `("harvest", token, manager)` | `amount` |
| `earnings_realized` | `("earnings_realized", backstop, pool)` | `(tier, principal_part, earnings_part, manager_part)` |

`earnings_realized` is emitted by `b_withdraw` so the split of each
withdrawal can be followed off-chain.
