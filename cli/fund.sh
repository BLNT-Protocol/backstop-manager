#!/usr/bin/env bash
# 0) Fund the manager contract: transfer the tier's LP tokens, or its underlying
# tokens (for join), from the signer's account into the contract.
# Any account can fund; this is a plain token transfer, not a contract call.
source "$(dirname "$0")/common.sh"
header "Fund the manager contract"

lp=$(lp_token)
tokens=("$lp")
labels=("LP token")
while IFS= read -r token; do
    tokens+=("$token")
    labels+=("underlying")
done < <(view "$lp" get_tokens | jq -r '.[]')

for i in "${!tokens[@]}"; do
    symbol=$(view "${tokens[i]}" symbol 2>/dev/null | tr -d '"' || true)
    echo "  $((i + 1))) ${labels[i]} ${symbol:-?} ${tokens[i]}"
    echo "     you hold $(fmt "$(view "${tokens[i]}" balance --id "$FROM" | num)")," \
        "contract holds $(fmt "$(contract_balance "${tokens[i]}")")"
done
echo
read -r -p "Token to send [1]: " choice
choice=${choice:-1}
if ! [[ "$choice" =~ ^[0-9]+$ ]] || (( choice < 1 || choice > ${#tokens[@]} )); then
    echo "Unknown choice." >&2
    exit 1
fi
token=${tokens[choice - 1]}

held=$(view "$token" balance --id "$FROM" | num)
if [[ "$held" == "0" ]]; then
    echo "$SOURCE holds none of $token."
    exit 0
fi
default=$held
if [[ "$(view "$token" symbol 2>/dev/null | tr -d '"')" == "native" ]]; then
    # Keep XLM for the account's reserve and fees
    default=$(calc "max(a - 50000000, 0)" "$held")
    echo "XLM: the default leaves 5 XLM in $SOURCE for reserve and fees."
fi
amount=$(ask_amount "Amount to send" "$default")
[[ "$amount" != "0" ]] || exit 0
if (( $(calc "int(a > b)" "$amount" "$held") )); then
    echo "$SOURCE only holds $(fmt "$held")." >&2
    exit 1
fi

confirm "Send $(fmt "$amount") of $token from $SOURCE to $MANAGER_ID?" || exit 0
invoke "$token" transfer --from "$FROM" --to "$MANAGER_ID" --amount "$amount" >/dev/null
echo "Contract now holds $(fmt "$(contract_balance "$token")") of $token."
if [[ "$token" == "$lp" ]]; then
    echo "Next: deposit (2) to put them in the backstop."
else
    echo "Next: join (1) to turn the underlying tokens into LP tokens."
fi
