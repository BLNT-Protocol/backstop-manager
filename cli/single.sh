#!/usr/bin/env bash
# Single-sided join: mint the tier's LP tokens from just one of its underlying
# tokens, from the signer's own account, then optionally send them to the
# manager contract. The manager contract can only join proportionally (join.sh),
# so this goes straight to the Comet LP with the signer as the user.
# If the signer is the owner, it can first pull that token out of the contract.
source "$(dirname "$0")/common.sh"
header "Single-sided join"

lp=$(lp_token)
tokens=()
while IFS= read -r token; do
    tokens+=("$token")
done < <(view "$lp" get_tokens | jq -r '.[]')

for i in "${!tokens[@]}"; do
    symbol=$(view "${tokens[i]}" symbol 2>/dev/null | tr -d '"' || true)
    echo "  $((i + 1))) ${symbol:-?} ${tokens[i]}"
    echo "     you hold $(fmt "$(view "${tokens[i]}" balance --id "$FROM" | num)")," \
        "contract holds $(fmt "$(contract_balance "${tokens[i]}")")"
done
echo
read -r -p "Token to join with [1]: " choice
choice=${choice:-1}
if ! [[ "$choice" =~ ^[0-9]+$ ]] || (( choice < 1 || choice > ${#tokens[@]} )); then
    echo "Unknown choice." >&2
    exit 1
fi
token=${tokens[choice - 1]}
is_xlm=false
[[ "$(view "$token" symbol 2>/dev/null | tr -d '"')" == "native" ]] && is_xlm=true

# Tokens sitting in the contract can't be joined single-sided there. The owner
# can pull them out to its own account first.
in_contract=$(contract_balance "$token")
if [[ "$in_contract" != "0" ]]; then
    owner=$(view "$MANAGER_ID" owner | tr -d '"')
    if [[ "$owner" == "$FROM" ]]; then
        pull=$(ask_amount "Pull from the contract to $SOURCE first (0 to skip)" "$in_contract")
        if [[ "$pull" != "0" ]]; then
            if (( $(calc "int(a > b)" "$pull" "$in_contract") )); then
                echo "The contract only holds $(fmt "$in_contract")." >&2
                exit 1
            fi
            confirm "Transfer $(fmt "$pull") of $token from the contract to $SOURCE?" || exit 0
            invoke "$MANAGER_ID" transfer_token --token "$token" --to "$FROM" \
                --amount "$pull" >/dev/null
            echo "Transferred."
        fi
    else
        echo "The contract holds $(fmt "$in_contract") of this token. Only the owner ($owner)"
        echo "can pull it out to join single-sided: the manager's refund_token sends it to"
        echo "the owner, who then runs this script."
    fi
    echo
fi

held=$(view "$token" balance --id "$FROM" | num)
if [[ "$held" == "0" ]]; then
    echo "$SOURCE holds none of $token."
    exit 0
fi
default=$held
if $is_xlm; then
    # Keep XLM for the account's reserve and fees
    default=$(calc "max(a - 50000000, 0)" "$held")
    echo "XLM: the default leaves 5 XLM in $SOURCE for reserve and fees."
fi

pool_balance=$(view "$lp" get_balance --token "$token" | num)
supply=$(view "$lp" get_total_supply | num)
weight=$(view "$lp" get_normalized_weight --token "$token" | num)
echo "Pool holds $(fmt "$pool_balance") of this token; LP supply $(fmt "$supply")."
# Comet's MAX_IN_RATIO: token_amount_in <= balance * (STROOP / 3 + 1) / STROOP
max_in=$(calc "a * 3333334 // 10000000" "$pool_balance")
echo "Comet accepts at most $(fmt "$max_in") (about a third of the pool's balance) in one join."
default=$(calc "min(a, b)" "$default" "$max_in")

amount=$(ask_amount "Amount to join with" "$default")
[[ "$amount" != "0" ]] || exit 0
if (( $(calc "int(a > b)" "$amount" "$held") )); then
    echo "$SOURCE only holds $(fmt "$held")." >&2
    exit 1
fi

# Simulate the join to see how many LP tokens it would mint
if ! expected=$(view "$lp" dep_tokn_amt_in_get_lp_tokns_out --token_in "$token" \
    --token_amount_in "$amount" --min_pool_amount_out 0 --user "$FROM" | num) \
    || [[ -z "$expected" ]]; then
    echo "Simulating the join failed. If the token is a classic asset, open DEX offers" >&2
    echo "may lock part of the balance (selling liabilities): try a smaller amount, or" >&2
    echo "rerun the call without --quiet to see the error." >&2
    exit 1
fi
min_out=$(calc "a * (10000 - b) // 10000" "$expected" "$SLIPPAGE_BPS")
# What the minted LP tokens are worth in this token at the pool's current
# ratio (weights are 7-decimal fractions). The gap is the swap fee and price impact.
worth=$(calc "a * b * 10000000 // (c * d)" "$expected" "$pool_balance" "$supply" "$weight")
cost_bps=$(calc "max(0, (b - a) * 10000 // b)" "$worth" "$amount")
echo
echo "Joining with $(fmt "$amount") would mint about $(fmt "$expected") LP tokens,"
echo "worth about $(fmt "$worth") of this token at the current pool ratio"
echo "(fee and price impact about $(calc "a // 100" "$cost_bps").$(printf '%02d' "$(calc "a % 100" "$cost_bps")")%)."
echo "Minimum accepted (with ${SLIPPAGE_BPS} bps slippage): $(fmt "$min_out")"

confirm "Join single-sided?" || exit 0
minted=$(invoke "$lp" dep_tokn_amt_in_get_lp_tokns_out --token_in "$token" \
    --token_amount_in "$amount" --min_pool_amount_out "$min_out" --user "$FROM" | num)
echo "Minted $(fmt "$minted") LP tokens to $SOURCE."

lp_held=$(view "$lp" balance --id "$FROM" | num)
send=$(ask_amount "LP tokens to send to the contract (0 to keep)" "$minted")
[[ "$send" != "0" ]] || exit 0
if (( $(calc "int(a > b)" "$send" "$lp_held") )); then
    echo "$SOURCE only holds $(fmt "$lp_held") LP tokens." >&2
    exit 1
fi
confirm "Send $(fmt "$send") LP tokens from $SOURCE to $MANAGER_ID?" || exit 0
invoke "$lp" transfer --from "$FROM" --to "$MANAGER_ID" --amount "$send" >/dev/null
echo "Contract now holds $(fmt "$(contract_balance "$lp")") LP tokens."
echo "Next: deposit (2) to put them in the backstop."
