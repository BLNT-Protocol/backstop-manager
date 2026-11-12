# Backstop manager CLI

Shell scripts for the day-to-day operations of a backstop manager contract,
using the [Stellar CLI](https://developers.stellar.org/docs/tools/cli).

## Setup

Requires `stellar`, `jq` and `python3`.

```sh
cp cli/config.env.example cli/config.env   # then fill it in
stellar keys add manager --secret-key       # or use an existing identity
```

`SOURCE` in `config.env` is the Stellar CLI identity that signs. It must be
the contract's manager (with enough scope for the operation) or the owner.
Set `CONFIG=/path/to/other.env` to use a different config file.

## Usage

```sh
cli/manager.sh
```

Choose an operation from the menu. Each script can also be run on its own.
Every operation shows the relevant balances, suggests a default, and asks
for confirmation before sending a transaction. Amounts are entered and shown
in tokens (7 decimals).

| # | Script | Operation | Scope |
|---|---|---|---|
| 1 | `join.sh` | Mint the tier's LP tokens from the underlying tokens the contract holds. Defaults to the most the balances allow, less `SLIPPAGE_BPS`. | High |
| 2 | `deposit.sh` | Deposit the contract's LP tokens into the backstop. Defaults to the balance less the harvestable amount. | Medium |
| 3 | `claim.sh` | Claim emissions. Simulates the claim first and sets the minimum LP out to that, less `SLIPPAGE_BPS`. | Low |
| 4 | `queue.sh` | Queue shares for withdrawal. Defaults to the shares worth the position's earnings, less shares already queued. | Medium |
| 5 | `withdraw.sh` | Withdraw queued shares whose 17-day queue has ended. Earnings among them become harvestable. | High |
| 6 | `harvest.sh` | Pay the manager its harvestable LP tokens. | Low |
| 7 | `fund.sh` | Send the tier's LP tokens, or its underlying tokens, from the signer's account to the contract. Defaults to the signer's whole balance. A plain token transfer, so any account can fund. | Any |
| 8 | `single.sh` | Single-sided join: mint the tier's LP tokens from just one underlying token (e.g. only BLNT), from the signer's own account, then optionally send them to the contract. Simulates first, shows the fee and price impact, and sets the minimum LP out less `SLIPPAGE_BPS`. Comet caps one join at about a third of the pool's balance of the token. If the signer is the owner, it can first pull that token out of the contract (`transfer_token`). | Any (pulling from the contract: owner) |
| 9 | `balances.sh` | Show the tokens the contract holds and what is harvestable, the backstop position (shares, queued withdrawals, value, principal, accrued earnings and the manager's share), and claimable emissions from a simulated claim. Read-only. | Any |
| 10 | `version.sh` | Show the deployed contract's WASM SHA-256 and metadata, the release tag and commit its GitHub build attestation points to, and the Cargo.toml version at that commit. Also shows the local checkout's version and build hash, and whether they match. Read-only. | Any |

To start a position: fund (7) with LP tokens, then deposit (2); or fund with
the underlying tokens, join (1), then deposit (2). With only one of the
underlying tokens (e.g. only BLNT), use single-sided join (8), which sends
the LP tokens to the contract, then deposit (2). Join (1) can't do this:
the contract only supports proportional joins.

To collect earnings: queue (4), wait 17 days, withdraw (5), harvest (6). Only
the earnings become harvestable; the principal stays in the contract. A
manager with High scope can run all four steps; with a lower scope the owner
has to queue and withdraw before the manager can harvest.
