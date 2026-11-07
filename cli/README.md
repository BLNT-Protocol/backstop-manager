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

To collect earnings: queue (4), wait 17 days, withdraw (5), harvest (6).
