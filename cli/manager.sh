#!/usr/bin/env bash
# Backstop manager CLI. Prompts for an operation and runs it.
set -uo pipefail
CLI_DIR="$(cd "$(dirname "$0")" && pwd)"

ops=(
    "join.sh|Create LP tokens from the contract's underlying tokens (High)"
    "deposit.sh|Deposit LP tokens into the backstop (Medium)"
    "claim.sh|Claim emissions (Low)"
    "queue.sh|Queue shares for withdrawal (Medium)"
    "withdraw.sh|Withdraw queued shares (High)"
    "harvest.sh|Harvest earnings to the manager (Low)"
    "fund.sh|Send LP or underlying tokens from the signer to the contract (anyone)"
    "single.sh|Single-sided join from the signer's account, then send the LP to the contract (anyone)"
    "balances.sh|Show balances, earnings and claimable emissions (read-only)"
)

while true; do
    echo
    echo "Backstop manager"
    for i in "${!ops[@]}"; do
        echo "  $((i + 1))) ${ops[i]#*|}"
    done
    echo "  q) Quit"
    read -r -p "Choose an operation: " choice || exit 0
    case "$choice" in
        q | Q) exit 0 ;;
        [1-9])
            if (( choice <= ${#ops[@]} )); then
                bash "$CLI_DIR/${ops[choice - 1]%%|*}" || echo "Operation failed."
            else
                echo "Unknown choice."
            fi
            ;;
        *) echo "Unknown choice." ;;
    esac
done
