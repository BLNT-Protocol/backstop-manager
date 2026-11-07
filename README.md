# backstop-manager

Contract that allows an owner to delegate management for a backstop
deposit to another key. This is useful for DAO and/or similar
governered owners where actions often require some voting period or
delay to take place. The manager can take actions on behalf of the
owner, like queue deposits for withdraw and deposit new LP tokens.

There can only be one owner and one manager.

This version targets the BLNT v3 backstop. Every backstop and Comet
action takes a `BackstopTier` (`FirstLoss`, `SecondLoss`,
`ThirdLoss`), and the token for that tier is looked up from the
backstop with `backstop_token(tier, pool)`. The manager is not bound
to a single backstop token.

The owner can:
* Transfer ownership to a new address in two steps: the owner calls `propose_owner`, and the new owner calls `accept_owner` in a later ledger, within one week. The owner can cancel a pending proposal, and the proposed owner can decline it, with `cancel_owner_proposal`. The manager is unchanged by an ownership transfer.
* Manage the manager key, including reducing the scope in which they can act
* Set the manager's share of backstop earnings (`set_harvest_bps`, in basis points, default 0), and correct the earnings records with `set_principal` and `set_harvestable` (see `interest.md`)
* Add/remove pools that the contract can move funds between
* Add/remove backstops that the contract can interact with
* Transfer tokens from held by the manager contract
* Perform all manager actions

Manager actions are limited by scope. Their scope can be:
* Low - they can only perform basic actions
* Medium - they can perform some management actions
* High - they can perform all management actions

Manager actions operate on funds held by the backstop manager contract
on behalf of the owner. Tokens sent out by manager actions go to the
owner, except for the manager's accounted share of earnings paid by
`harvest`.

The manager can:
* Refund tokens held by the manager contract to the owner (scope req. `Low`)
* Claim emissions for a backstop tier (scope req. `Low`)
* Deposit a tier's backstop token into a backstop (scope req. `Medium`)
* Queue for withdraw funds from backstop (scope req. `Medium`)
* Cancel queue funds for withdraw from backstop (scope req. `Medium`)
* Withdraw funds from backstop to the manager contract (scope req. `High`)
* Harvest its share of backstop earnings to its own address (scope req. `Low`). Earnings are the value of a backstop position above the LP tokens deposited into it, and become harvestable when withdrawn with `b_withdraw`. The manager can never harvest principal.
* Join and/or Exit the Comet LP used by a backstop tier (scope req. `High`)

## CLI

`cli/manager.sh` is a menu-driven interface for joining the LP, depositing, claiming, queueing, withdrawing and harvesting. See `cli/README.md`.
