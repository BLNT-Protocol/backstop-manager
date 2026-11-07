#!/usr/bin/env bash
# 6) Pay the manager its harvestable LP tokens.
# Requires a Low scope manager (or the owner); tokens always go to the manager.
source "$(dirname "$0")/common.sh"
header "Harvest earnings"

lp=$(lp_token)
owed=$(harvestable "$lp")
balance=$(contract_balance "$lp")
pay=$(calc "min(a, b)" "$owed" "$balance")
manager=$(view "$MANAGER_ID" manager | jq -r '.id')

echo "Harvestable: $(fmt "$owed") LP tokens ($lp)"
echo "Contract holds: $(fmt "$balance")"
echo "Pays to the manager: $manager"
echo
if [[ "$pay" == "0" ]]; then
    echo "Nothing to harvest."
    exit 0
fi

confirm "Harvest $(fmt "$pay") LP tokens?" || exit 0
paid=$(invoke "$MANAGER_ID" harvest --from "$FROM" --token "$lp" | num)
echo "Paid $(fmt "$paid") LP tokens to $manager."
