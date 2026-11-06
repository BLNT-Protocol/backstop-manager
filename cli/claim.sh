#!/usr/bin/env bash
# 3) Claim backstop emissions. They are converted to LP tokens and deposited
# back into the backstop, adding shares (but not principal).
# Requires a Low scope manager (or the owner).
source "$(dirname "$0")/common.sh"
header "Claim emissions"

# Simulate the claim to see how many LP tokens it would mint
expected=$(view "$MANAGER_ID" b_claim --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --min_lp_tokens_out 0 | num)
echo "Claiming now would mint about $(fmt "$expected") LP tokens."
if [[ "$expected" == "0" ]]; then
    echo "Nothing to claim."
    exit 0
fi
min_out=$(calc "a * (10000 - b) // 10000" "$expected" "$SLIPPAGE_BPS")
echo "Minimum accepted (with ${SLIPPAGE_BPS} bps slippage): $(fmt "$min_out")"

confirm "Claim emissions?" || exit 0
minted=$(invoke "$MANAGER_ID" b_claim --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --min_lp_tokens_out "$min_out" | num)
echo "Minted $(fmt "$minted") LP tokens into the backstop."
