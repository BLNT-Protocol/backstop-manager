#!/usr/bin/env bash
# 4) Queue backstop shares for withdrawal. They can be withdrawn after the
# backstop's queue period (17 days).
# Requires a Medium scope manager (or the owner).
source "$(dirname "$0")/common.sh"
header "Queue shares for withdrawal"

ub=$(user_balance)
td=$(tier_data)
shares=$(jq -r '.shares' <<<"$ub")
queued=$(jq -r '.q4w[].amount' <<<"$ub" | sum_lines)
tier_tokens=$(jq -r '.tokens' <<<"$td")
tier_shares=$(jq -r '.shares' <<<"$td")
principal=$(principal)

value=0
if [[ "$tier_shares" != "0" ]]; then
    value=$(calc "(a + b) * c // d" "$shares" "$queued" "$tier_tokens" "$tier_shares")
fi
earnings=$(calc "max(a - b, 0)" "$value" "$principal")

# Shares worth the earnings, less what is already queued (withdrawals count
# as earnings first)
default=0
if [[ "$tier_tokens" != "0" ]]; then
    default=$(calc "max(min(a * b // c - d, e), 0)" "$earnings" "$tier_shares" "$tier_tokens" "$queued" "$shares")
fi

echo "Shares: $(fmt "$shares") (plus $(fmt "$queued") already queued)"
echo "Position value: $(fmt "$value") LP tokens"
echo "Principal: $(fmt "$principal") LP tokens"
echo "Earnings: $(fmt "$earnings") LP tokens"
echo "Default is the shares worth the earnings, less shares already queued."
echo
amount=$(ask_amount "Shares to queue" "$default")
[[ "$amount" != "0" ]] || exit 0

confirm "Queue $(fmt "$amount") shares for withdrawal?" || exit 0
invoke "$MANAGER_ID" b_queue_withdrawal --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --amount "$amount" >/dev/null
echo "Queued. Withdraw them after the queue period with the withdraw operation."
