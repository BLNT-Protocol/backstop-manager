# Changing the owner

The manager needs support for changing the owner. Ownership moves in two
steps: the current owner proposes a new owner, and the new owner accepts.
The owner does not change until the proposal is accepted, which proves the
new address can authorize calls to the manager. This matters most when the
new owner is a contract (e.g. a DAO or governor).

The manager is not affected by an ownership change. The existing manager
keeps its address and scope after the new owner accepts.

## Functions

*  `propose_owner(new_owner: Address)` (Only Owner): proposes `new_owner`
   as the next owner of the manager. The address can be a standard G
   pubkey or a contract address.
   *  Records the pending owner, the current ledger sequence, and the
      current ledger timestamp.
   *  If a proposal is already pending, it is replaced and its expiry
      restarts.
   *  Fails with `InvalidOwner` if `new_owner` is the current owner or the
      manager contract itself (`e.current_contract_address()`), since the
      contract can't authorize calls to itself and would be left without a
      usable owner.

*  `accept_owner()` (Only Pending Owner): called by the pending owner
   (requires `new_owner.require_auth()`). On success, `new_owner` becomes
   the owner and the pending proposal is cleared.
   *  Fails with `NoPendingOwner` if there is no pending proposal.
   *  Fails with `OwnerTransferNotReady` if called in the same ledger as
      `propose_owner` (`e.ledger().sequence()` must be strictly greater
      than the recorded proposal ledger). Since a Soroban transaction runs
      within a single ledger, this also prevents proposing and accepting
      in the same transaction.
   *  Fails with `OwnerProposalExpired` if called after the proposal has
      expired (see Expiry).

*  `cancel_owner_proposal(from: Address)` (Owner or Pending Owner): clears
   the pending proposal. Requires `from.require_auth()`. Can be called at any time by the current owner to cancel the
   transfer, or by the pending owner to decline it.
   *  Fails with `NoPendingOwner` if there is no pending proposal.
   *  Fails with `UnauthorizedError` if `from` is neither the owner nor the
      pending owner.

*  `pending_owner() -> Option<PendingOwner>` (Read-Only): returns the
   pending proposal, if any, so UIs and DAO tooling can see a transfer in
   progress. An expired proposal is still returned until it is replaced
   or cancelled; callers can compare `timestamp` against the expiry.

   ```rust
   #[contracttype]
   pub struct PendingOwner {
       pub id: Address,      // the proposed owner
       pub ledger: u32,      // ledger sequence of the proposal
       pub timestamp: u64,   // ledger timestamp of the proposal
   }
   ```

## Expiry

A proposal expires one week after it is made. `accept_owner` succeeds only
while:

```
e.ledger().timestamp() <= pending.timestamp + OWNER_PROPOSAL_EXPIRY
```

where `OWNER_PROPOSAL_EXPIRY = 7 * 24 * 60 * 60` seconds (604800).

Expiry uses the ledger timestamp rather than a ledger count, so it doesn't
depend on an assumed ledger close time. The same-ledger check in
`accept_owner` uses the ledger sequence, since it is about ledgers rather
than time.

## Storage

The pending proposal is stored in instance storage under a new key
(e.g. `"PendOwner"`), alongside the existing owner. It is removed when the
proposal is accepted or cancelled. All new functions call
`storage::extend_instance`.

## Errors

New error codes in `BackstopManagerErrors`:

| Error                        | Meaning                                                  |
|------------------------------|----------------------------------------------------------|
| `InvalidOwner` (103)         | Proposed owner is the current owner or the contract      |
| `NoPendingOwner` (104)       | Accept or cancel called with no pending proposal         |
| `OwnerTransferNotReady` (105)| Accept called in the same ledger as the proposal         |
| `OwnerProposalExpired` (106) | Accept called after the proposal expired                 |

## Events

| Event              | Topics                        | Data          |
|--------------------|-------------------------------|---------------|
| `owner_proposed`   | `("owner_proposed", owner)`     | `new_owner` |
| `owner_accepted`   | `("owner_accepted", new_owner)` | `old_owner` |
| `owner_cancelled`  | `("owner_cancelled", from)`     | `new_owner` |
