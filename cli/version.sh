#!/usr/bin/env bash
# Show which build of the backstop manager contract is deployed: its WASM
# SHA-256, embedded metadata, the release build that produced it (from its
# GitHub attestation) and the Cargo.toml version at that commit. Compares it
# with the local checkout and build. Read-only.
source "$(dirname "$0")/common.sh"
header "Version"

REPO_DIR="$(cd "$CLI_DIR/.." && pwd)"
LOCAL_WASM="$REPO_DIR/target/wasm32v1-none/optimized/backstop_manager.wasm"

# Cargo.toml's package version, read from stdin
cargo_version() { sed -n 's/^version *= *"\(.*\)".*/\1/p' | head -n 1; }

deployed=$(stellar contract info hash --quiet --network "$NETWORK" --id "$MANAGER_ID")
echo "Deployed contract"
echo "  WASM SHA-256: $deployed"
stellar contract info meta --quiet --network "$NETWORK" --id "$MANAGER_ID" 2>/dev/null \
    | sed -n 's/^ *• \([^:]*\): \([^ ]*\).*/  \1: \2/p'

# The release workflow attests each WASM it builds, so the hash leads back to
# the tag and commit. This looks the hash up on GitHub.
build=$(stellar contract info build --network "$NETWORK" --id "$MANAGER_ID" 2>&1 \
    | sed 's/\x1b\[[0-9;]*m//g' || true)
ref=$(sed -n 's/^ *Ref: *refs\/tags\/\(.*\)/\1/p' <<<"$build")
commit=$(sed -n 's/^ *Git Commit: *\([0-9a-f]*\).*/\1/p' <<<"$build")
if [[ -n "$commit" ]]; then
    echo "  release: ${ref:-?} (commit ${commit:0:7})"
    if version=$(git -C "$REPO_DIR" show "$commit:Cargo.toml" 2>/dev/null | cargo_version) \
        && [[ -n "$version" ]]; then
        echo "  Cargo.toml version: $version"
    else
        echo "  Cargo.toml version: unknown (commit not in the local repo; try git fetch)"
    fi
else
    echo "  release: no build attestation found, so this is not a release build"
    echo "  Cargo.toml version: unknown"
fi

echo
echo "Local checkout"
echo "  commit: $(git -C "$REPO_DIR" describe --tags --always --dirty 2>/dev/null || echo '?')"
echo "  Cargo.toml version: $(cargo_version <"$REPO_DIR/Cargo.toml")"
if [[ -f "$LOCAL_WASM" ]]; then
    local_hash=$(python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$LOCAL_WASM")
    echo "  built WASM SHA-256: $local_hash"
    if [[ "$local_hash" == "$deployed" ]]; then
        echo "  The local build matches the deployed contract."
    else
        echo "  The local build differs from the deployed contract. Local builds don't"
        echo "  reproduce the release build exactly, so compare the commit and version."
    fi
else
    echo "  built WASM: none (run make build)"
fi
