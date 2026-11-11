#!/usr/bin/env bash
# 8) Show the manager contract's balances: tokens it holds, its backstop
# position (principal and accrued earnings) and claimable emissions.
# Read-only; nothing is submitted.
source "$(dirname "$0")/common.sh"
header "Balances"

lp=$(lp_token)
tokens=("$lp")
labels=("LP token")
while IFS= read -r token; do
    tokens+=("$token")
    labels+=("underlying")
done < <(view "$lp" get_tokens | jq -r '.[]')

echo "Held by the contract (harvestable = owed to the manager):"
for i in "${!tokens[@]}"; do
    symbol=$(view "${tokens[i]}" symbol 2>/dev/null | tr -d '"' || true)
    echo "  ${labels[i]} ${symbol:-?} ${tokens[i]}"
    echo "    balance $(fmt "$(contract_balance "${tokens[i]}")")," \
        "harvestable $(fmt "$(harvestable "${tokens[i]}")")"
done

ub=$(user_balance)
td=$(tier_data)
shares=$(jq -r '.shares' <<<"$ub")
queued=$(jq -r '.q4w[].amount' <<<"$ub" | sum_lines)
tier_tokens=$(jq -r '.tokens' <<<"$td")
tier_shares=$(jq -r '.shares' <<<"$td")
principal=$(principal)
bps=$(view "$MANAGER_ID" harvest_bps | num)

value=0
if [[ "$tier_shares" != "0" ]]; then
    value=$(calc "(a + b) * c // d" "$shares" "$queued" "$tier_tokens" "$tier_shares")
fi
earnings=$(calc "max(a - b, 0)" "$value" "$principal")

echo
echo "Backstop position (in LP tokens):"
echo "  Shares: $(fmt "$shares") active, $(fmt "$queued") queued for withdrawal"
now=$(date +%s)
while read -r amount exp; do
    if (( exp <= now )); then status="ready"; else status="ready at $(fmt_timestamp "$exp")"; fi
    echo "    $(fmt "$amount") shares: $status"
done < <(jq -r '.q4w[] | "\(.amount) \(.exp)"' <<<"$ub")
echo "  Value: $(fmt "$value")"
echo "  Principal: $(fmt "$principal")"
if (( $(calc "int(a < b)" "$value" "$principal") )); then
    echo "  Accrued earnings: 0 (value is $(fmt "$(calc "b - a" "$value" "$principal")") below principal)"
else
    echo "  Accrued earnings: $(fmt "$earnings")"
fi
echo "  Manager's share at $bps bps: $(fmt "$(calc "a * b // 10000" "$earnings" "$bps")")"

# Emissions have no view function; simulate the claim to see what it would mint
echo
if claimable=$(view "$MANAGER_ID" b_claim --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --min_lp_tokens_out 0 2>/dev/null | num); then
    echo "Claimable emissions: about $(fmt "$claimable") LP tokens (from a simulated claim)"
else
    echo "Claimable emissions: unavailable (the simulated claim failed; $SOURCE may lack"
    echo "  manager or owner rights, or the tier may not earn emissions)"
fi
