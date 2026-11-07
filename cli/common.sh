# Shared helpers for the backstop manager CLI. Sourced by the other scripts.

set -euo pipefail

CLI_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${CONFIG:-$CLI_DIR/config.env}"

if [[ ! -f "$CONFIG" ]]; then
    echo "Missing $CONFIG. Copy config.env.example to config.env and fill it in." >&2
    exit 1
fi
# shellcheck source=/dev/null
source "$CONFIG"

for var in NETWORK SOURCE MANAGER_ID BACKSTOP_ID POOL_ID TIER SLIPPAGE_BPS; do
    if [[ -z "${!var:-}" ]]; then
        echo "$var is not set in $CONFIG" >&2
        exit 1
    fi
done
case "$TIER" in
    FirstLoss | SecondLoss | ThirdLoss) ;;
    *) echo "TIER must be FirstLoss, SecondLoss or ThirdLoss" >&2; exit 1 ;;
esac

FROM="$(stellar keys address "$SOURCE")"

# Read-only call: simulate only and print the result
view() {
    local id="$1"; shift
    stellar contract invoke --quiet --send=no --network "$NETWORK" --source "$SOURCE" \
        --id "$id" -- "$@"
}

# State-changing call: sign with SOURCE and submit
invoke() {
    local id="$1"; shift
    stellar contract invoke --quiet --send=yes --network "$NETWORK" --source "$SOURCE" \
        --id "$id" -- "$@"
}

# Strip JSON quotes from an i128 result ("123" -> 123)
num() { tr -d '"[:space:]'; }

# Integer math on arbitrarily large numbers. Arguments are bound to a, b, c, ...
# e.g. calc "a * b // c" "$x" "$y" "$z"
calc() {
    python3 - "$@" <<'PY'
import sys
expr, args = sys.argv[1], [int(x) for x in sys.argv[2:]]
print(int(eval(expr, {"min": min, "max": max}, dict(zip("abcdefgh", args)))))
PY
}

# Sum integers read one per line from stdin (0 if none)
sum_lines() { python3 -c "import sys; print(sum(int(l) for l in sys.stdin if l.strip()))"; }

# 7-decimal base units -> human readable (12345678 -> 1.2345678)
fmt() {
    python3 - "$1" <<'PY'
import sys
from decimal import Decimal
print(f"{Decimal(int(sys.argv[1])) / 10**7:,.7f}")
PY
}

# Unix timestamp -> a stable UTC display, without relying on GNU or BSD date.
fmt_timestamp() {
    python3 - "$1" <<'PY'
import sys
from datetime import datetime, timezone
print(datetime.fromtimestamp(int(sys.argv[1]), timezone.utc).strftime("%Y-%m-%d %H:%M UTC"))
PY
}

# Human readable -> 7-decimal base units (1.5 -> 15000000)
to_units() {
    python3 - "$1" <<'PY'
import sys
from decimal import Decimal, InvalidOperation
try:
    v = Decimal(sys.argv[1].replace(",", ""))
except InvalidOperation:
    sys.exit(f"invalid amount: {sys.argv[1]}")
if v < 0:
    sys.exit("amount must not be negative")
print(int(v * 10**7))
PY
}

# Prompt for an amount in tokens, with a default in base units. Prints base units.
ask_amount() {
    local prompt="$1" default="$2" answer
    read -r -p "$prompt [$(fmt "$default")]: " answer
    if [[ -z "$answer" ]]; then echo "$default"; else to_units "$answer"; fi
}

confirm() {
    local answer
    read -r -p "${1:-Proceed?} [y/N]: " answer
    [[ "$answer" == "y" || "$answer" == "Y" ]]
}

# The LP token used by the configured tier
lp_token() { view "$BACKSTOP_ID" backstop_token --tier "$TIER" --pool "$POOL_ID" | tr -d '"'; }

# Token balance of the manager contract
contract_balance() { view "$1" balance --id "$MANAGER_ID" | num; }

harvestable() { view "$MANAGER_ID" harvestable --token "$1" | num; }

principal() {
    view "$MANAGER_ID" principal --backstop "$BACKSTOP_ID" --pool "$POOL_ID" --tier "$TIER" | num
}

# The manager contract's backstop balance as JSON: {"q4w":[{"amount":..,"exp":..}],"shares":..}
user_balance() {
    view "$BACKSTOP_ID" user_balance --tier "$TIER" --pool "$POOL_ID" --user "$MANAGER_ID"
}

# The configured tier's entry from pool_data as JSON: {"shares":..,"tokens":..,...}
tier_data() {
    local index
    case "$TIER" in FirstLoss) index=0 ;; SecondLoss) index=1 ;; ThirdLoss) index=2 ;; esac
    view "$BACKSTOP_ID" pool_data --pool "$POOL_ID" | jq ".tiers[$index]"
}

header() {
    echo
    echo "== $1 =="
    echo "Manager contract: $MANAGER_ID"
    echo "Backstop: $BACKSTOP_ID  Pool: $POOL_ID  Tier: $TIER"
    echo "Signing as: $SOURCE ($FROM)"
    echo
}
