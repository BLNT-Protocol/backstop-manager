# Backstop Manager: BLNT v3 Compatibility Notes

Written 2026-11-05.

## Summary

The backstop manager is **not** tied to the BLND token, but it **is** tied to the
Blend v2 backstop interface. It does not work as-is with the BLNT v3 stack
(YieldBlent pool `CAUTPSQDAJEUV5XEU3EXFVYRGDJLEJGVTFJCFZCSPTDCREAMZ573VF4D`).

The owner, manager-scope and allowlist logic does not depend on Blend and
carries over unchanged.

## What's already generic

- `backstop_token` is a constructor argument (`src/contract.rs:46`). It can be
  any Comet LP token.
- Backstops and pools are owner-managed allowlists, up to 4 each
  (`set_backstops` / `set_pools`).
- The bootstrapper address can be changed by the owner.
- Bootstrap tokens are picked by index into `comet.get_tokens()`.
- "BLND-USDC" appears only in doc comments (`src/contract.rs:34`, `:350`, `:490`).

## v2 dependencies

1. **Blend v2 backstop ABI**, through `blend-contract-sdk = 2.22.0`.
2. **Comet backstop token**: `get_tokens`, `join_pool`, `exit_pool`,
   `get_normalized_weight`.
3. **A bootstrapper bound to the same backstop and LP** (`src/testutils.rs:105`).
   `bb_claim_bootstrap` works out its deposit amount from the manager's stored
   `backstop_token`.
4. **`backstop_token` can't be changed after deployment.** Each manager
   instance is locked to one LP token.

## What breaks on v3

Source: `blnt-contracts-v3/docs/V2_V3_PUBLIC_API.md`

| Manager fn             | v2 call it makes                       | v3 signature                                  |
|------------------------|----------------------------------------|-----------------------------------------------|
| `b_deposit`            | `deposit(from, pool, amount)`          | `deposit(tier, from, pool, amount)`           |
| `b_queue_withdrawal`   | `queue_withdrawal(from, pool, amount)` | `queue_withdrawal(tier, from, pool, amount)`  |
| `b_dequeue_withdrawal` | `dequeue_withdrawal(from, pool, amount)` | `dequeue_withdrawal(tier, from, pool, amount)` |
| `b_withdraw`           | `withdraw(from, pool, amount)`         | `withdraw(tier, from, pool, amount, to)`      |
| `b_claim`              | `claim(from, pools, min_lp_out)`       | `claim(tier, from, pools, min_lp_out)`        |

### Multiple backstop tokens per pool

v3 pools have up to three tiers, each with its own token. YieldBlent has two:

| Tier       | Asset    | Token                                                      |
|------------|----------|------------------------------------------------------------|
| FirstLoss  | BlntXlm  | `CA5HX3D47PQZWC2E7VWXCKH4534P2NOJEOIXAWU47MI4QV2FQUYY3FLC` |
| SecondLoss | BlntUsdc | `CDQMIKIHKA5UY3FQJ2UDTLNX3GOPD3GPNZWSPTOBVOHEUMQJQXOPC7Y3` |

`b_deposit` pre-authorizes `transfer` on the single stored `backstop_token`, so
it would sign for the wrong token on at least one tier. The token has to be
resolved per call with `backstop_token(tier, pool)`. `c_join_pool` and
`c_exit_pool` also assume one Comet pool.

### What still works

- v3 `deposit` still calls `token.transfer(from, backstop, amount)`
  (`blnt-contracts-v3/backstop/src/backstop/deposit.rs:29`), so the existing
  `authorize_as_current_contract` approach is fine once it targets the tier's
  token.
- No bootstrapper was used for YieldBlent, so the `bb_*` functions and the
  bootstrapper dependency can be removed.

## Port plan

1. Replace the v2 SDK `BackstopClient` with a client imported from the v3
   backstop WASM (`contractimport!`).
2. Add a `tier: BackstopTier` argument to all `b_*` functions.
3. Set the new `to` argument of `withdraw` to the manager contract itself, so
   funds stay with the owner and manager.
4. Get the deposit token from `backstop.backstop_token(tier, pool)` instead of
   from storage. Remove or repurpose the stored `backstop_token`.
5. Make `c_join_pool` and `c_exit_pool` take a tier or token. Possibly check
   it against the tokens of an allowlisted backstop and pool.
6. Remove the bootstrapper functions, storage and `backstop_bootstrapper.wasm`.
7. Check that the Comet V1.1 `join_pool` / `exit_pool` / `get_tokens` interface
   hasn't changed. The `comet.wasm` builds differ:
   - v3 repo: `d735c339…2b3e687`
   - this repo: `8abc2891…94d1a36`
8. Rewrite the tests against the v3 contracts and the v3 `comet.wasm`.

## Port status (branch `v3-port`)

| Step | Status |
| --- | --- |
| 1. v3 backstop client | Done. `contractimport!` of `src/dependencies/backstop.wasm`. `blend-contract-sdk` is removed. |
| 2. `tier` on `b_*` | Done. The argument order is `(from, backstop, tier, pool, ...)`. |
| 3. `withdraw` `to` | Done. Always the manager contract. |
| 4. Token per tier | Done. `b_deposit` pre-authorizes `transfer` on `backstop_token(tier, pool)`. The stored `backstop_token` is removed. |
| 5. Comet functions | Done. `c_join_pool` / `c_exit_pool` take `(backstop, tier, pool)`, check them against the allowlists, and look up the LP from the backstop. |
| 6. Bootstrapper removed | Done. Functions, storage, `InvalidTokenIndex` error and the WASM are gone. |
| 7. Comet interface | Checked. `join_pool`, `exit_pool` and `get_tokens` are unchanged. `transfer` now takes a `MuxedAddress` `to`, which encodes the same as an `Address`, so the auth entries still match. `comet.wasm` is replaced with the v3 build. |
| 8. Tests | Done. A new v3 fixture runs the backstop and Comet tests for both FirstLoss (BLNT:XLM) and SecondLoss (BLNT:USDC). 6 tests pass. |

### Changes beyond the plan

- **SDK 22 → 27, Rust 1.81 → 1.91.1, target `wasm32v1-none`.** v3 contracts are
  built for Protocol 27, and an SDK 22 test host can't run them.
- **The manager defines its own `BackstopTier`** (`src/types.rs`) and converts
  it to the backstop's type. `contractimport!` doesn't export imported types
  into the contract spec, so without this the ABI would reference an undefined
  type and the CLI and JS bindings couldn't encode the argument.
- The constructor is now `(owner, manager, admin_scope, backstops, pools)`.

### Open items

- `js-sdk/` still has the v2 bindings. Regenerate it from the new WASM.
- Vendored v3 WASMs are local builds of `5d055ca` (see `src/dependencies/README.md`).
  Swap in the tagged release artifacts once they exist.
- New v3 entry points (`user_balance(tier, ..)`, `force_*`, `donate`) aren't
  exposed. That isn't needed for owner/manager custody.
- Plain USDC and XLM tiers work with `b_*`, but `c_*` will fail for them because
  they aren't Comet pools. That's expected.
