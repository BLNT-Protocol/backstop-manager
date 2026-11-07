#!/usr/bin/env bash
# 2) Deposit the manager contract's LP tokens into the backstop.
# Requires a Medium scope manager (or the owner).
source "$(dirname "$0")/common.sh"
header "Deposit LP tokens into the backstop"

lp=$(lp_token)
balance=$(contract_balance "$lp")
owed=$(harvestable "$lp")
default=$(calc "max(a - b, 0)" "$balance" "$owed")

echo "Contract holds $(fmt "$balance") LP tokens ($lp)"
echo "Owed to the manager (harvestable): $(fmt "$owed")"
echo "Depositing harvestable tokens turns them into principal; the default leaves them out."
echo
if [[ "$balance" == "0" ]]; then
    echo "Nothing to deposit."
    exit 0
fi
amount=$(ask_amount "LP tokens to deposit" "$default")
[[ "$amount" != "0" ]] || exit 0

confirm "Deposit $(fmt "$amount") LP tokens?" || exit 0
shares=$(invoke "$MANAGER_ID" b_deposit --from "$FROM" --backstop "$BACKSTOP_ID" --tier "$TIER" \
    --pool_address "$POOL_ID" --amount "$amount" | num)
echo "Minted $(fmt "$shares") backstop shares. Principal is now $(fmt "$(principal)")."
