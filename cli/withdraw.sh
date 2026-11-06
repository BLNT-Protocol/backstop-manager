#!/usr/bin/env bash
# 5) Withdraw queued shares whose queue period has ended. The LP tokens go to
# the manager contract; earnings among them become harvestable.
# Requires a High scope manager (or the owner).
source "$(dirname "$0")/common.sh"
header "Withdraw queued shares"

ub=$(user_balance)
now=$(date +%s)
entries=$(jq -r '.q4w | length' <<<"$ub")
if [[ "$entries" == "0" ]]; then
    echo "No shares are queued for withdrawal."
    exit 0
fi

echo "Queued withdrawals:"
while read -r amount exp; do
    if (( exp <= now )); then status="ready"; else status="ready $(date -d "@$exp" '+%Y-%m-%d %H:%M')"; fi
    echo "  $(fmt "$amount") shares: $status"
done < <(jq -r '.q4w[] | "\(.amount) \(.exp)"' <<<"$ub")
matured=$(jq -r --argjson now "$now" '.q4w[] | select(.exp <= $now) | .amount' <<<"$ub" | sum_lines)
echo
if [[ "$matured" == "0" ]]; then
    echo "Nothing is ready to withdraw yet."
    exit 0
fi
amount=$(ask_amount "Shares to withdraw" "$matured")
[[ "$amount" != "0" ]] || exit 0

lp=$(lp_token)
owed_before=$(harvestable "$lp")
confirm "Withdraw $(fmt "$amount") shares?" || exit 0
returned=$(invoke "$MANAGER_ID" b_withdraw --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --amount "$amount" | num)
owed_after=$(harvestable "$lp")
echo "Withdrew $(fmt "$returned") LP tokens to the manager contract."
echo "Newly harvestable: $(fmt "$(calc "a - b" "$owed_after" "$owed_before")")  Total harvestable: $(fmt "$owed_after")"
echo "Principal is now $(fmt "$(principal)")."
