# backstop-manager

Contract that allows an owner to delegate management for a backstop deposit to another key. This is useful for DAO and/or similar governered owners where actions often require some voting period or delay to take place. The admin can take actions on behalf of the owner, like queue deposits for withdraw and deposit new LP tokens.

This version targets the BLNT v3 backstop. Every backstop and Comet action takes a `BackstopTier` (`FirstLoss`, `SecondLoss`, `ThirdLoss`), and the token for that tier is looked up from the backstop with `backstop_token(tier, pool)`. The manager is not bound to a single backstop token.

The owner can:
* Manage the manager key, including reducing the scope in which they can act
* Add/remove pools that the contract can move funds between
* Add/remove backstops that the contract can interact with
* Transfer tokens from held by the manager contract
* Perform all manager actions

Manager actions are limited by scope. Their scope can be:
* Low - they can only perform basic actions
* Medium - they can perform some management actions
* High - they can perform all management actions

All manager actions only allow funds to be held by the owner. They act on behalf of the owner with the funds held by the backstop manager contract.

The admin can:
* Refund tokens held by the manager contract to the owner (scope req. `Low`)
* Claim emissions for a backstop tier (scope req. `Low`)
* Deposit a tier's backstop token into a backstop (scope req. `Medium`)
* Queue for withdraw funds from backstop (scope req. `Medium`)
* Cancel queue funds for withdraw from backstop (scope req. `Medium`)
* Withdraw funds from backstop to the manager contract (scope req. `High`)
* Join and/or Exit the Comet LP used by a backstop tier (scope req. `High`)
