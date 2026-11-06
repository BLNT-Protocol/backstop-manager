#!/usr/bin/env bash
# 1) Join the tier's Comet LP with the underlying tokens the manager contract holds.
# Requires a High scope manager (or the owner).
source "$(dirname "$0")/common.sh"
header "Create LP tokens"

lp=$(lp_token)
supply=$(view "$lp" get_total_supply | num)
mapfile -t tokens < <(view "$lp" get_tokens | jq -r '.[]')

# The most LP tokens the contract's balances can mint: Comet joins are
# proportional, so the scarcest token sets the limit.
max_out=""
declare -a balances pool_balances
for i in "${!tokens[@]}"; do
    balances[i]=$(contract_balance "${tokens[i]}")
    pool_balances[i]=$(view "$lp" get_balance --token "${tokens[i]}" | num)
    echo "Contract holds $(fmt "${balances[i]}") of ${tokens[i]}"
    limit=$(calc "a * b // c" "${balances[i]}" "$supply" "${pool_balances[i]}")
    if [[ -z "$max_out" ]]; then max_out=$limit; else max_out=$(calc "min(a, b)" "$max_out" "$limit"); fi
done

default_out=$(calc "a * (10000 - b) // 10000" "$max_out" "$SLIPPAGE_BPS")
echo
echo "LP token: $lp"
echo "Most LP tokens these balances can mint: $(fmt "$max_out")"
if [[ "$default_out" == "0" ]]; then
    echo "Nothing to join."
    exit 0
fi
out=$(ask_amount "LP tokens to mint" "$default_out")

# Allow each token amount to be up to SLIPPAGE_BPS above the current pool ratio,
# but never more than the contract holds.
max_in="["
for i in "${!tokens[@]}"; do
    expected=$(calc "-(-a * b // c)" "$out" "${pool_balances[i]}" "$supply")
    limit=$(calc "min(a * (10000 + b) // 10000, c)" "$expected" "$SLIPPAGE_BPS" "${balances[i]}")
    echo "Max in: $(fmt "$limit") of ${tokens[i]}"
    max_in+="\"$limit\","
done
max_in="${max_in%,}]"

confirm "Mint $(fmt "$out") LP tokens?" || exit 0
invoke "$MANAGER_ID" c_join_pool --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --pool_amount_out "$out" --max_amounts_in "$max_in" >/dev/null
echo "Contract now holds $(fmt "$(contract_balance "$lp")") LP tokens."
