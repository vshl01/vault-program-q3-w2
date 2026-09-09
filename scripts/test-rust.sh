#!/usr/bin/env bash
# Runs every Rust LiteSVM suite, one program at a time, with per-program totals.
set -uo pipefail

cd "$(dirname "$0")/.."

PROGRAMS=("vault_program:VAULT PROGRAM" "escrow:ESCROW" "nc_vault:NON-CUSTODIAL VAULT")

if [ -t 1 ]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; GREEN=$'\033[32m'; RED=$'\033[31m'
  CYAN=$'\033[36m'; RESET=$'\033[0m'
else
  BOLD=""; DIM=""; GREEN=""; RED=""; CYAN=""; RESET=""
fi

WIDTH=43
RULE=$(printf '─%.0s' $(seq 1 $WIDTH))
BAR=$(printf '═%.0s' $(seq 1 $((WIDTH + 2))))

TOTAL_PASS=0
TOTAL_FAIL=0
FAILED_PROGRAMS=()

for entry in "${PROGRAMS[@]}"; do
  pkg="${entry%%:*}"
  label="${entry##*:}"

  printf '\n%s%s╭%s╮%s\n' "$BOLD" "$CYAN" "$RULE" "$RESET"
  printf '%s%s│  %-*s│%s\n' "$BOLD" "$CYAN" "$((WIDTH - 2))" "$label" "$RESET"
  printf '%s%s╰%s╯%s\n' "$BOLD" "$CYAN" "$RULE" "$RESET"

  pass=0
  fail=0

  while IFS= read -r line; do
    case "$line" in
      *"Running tests/"*)
        file="${line#*Running }"
        file="${file%% (*}"
        printf '\n  %s%s%s\n' "$DIM" "$file" "$RESET"
        ;;
      "test "*" ... ok")
        name="${line#test }"; name="${name% ... ok}"
        printf '    %s✔%s %s\n' "$GREEN" "$RESET" "$name"
        pass=$((pass + 1))
        ;;
      "test "*" ... FAILED")
        name="${line#test }"; name="${name% ... FAILED}"
        printf '    %s✘ %s%s\n' "$RED" "$name" "$RESET"
        fail=$((fail + 1))
        ;;
      *panicked*|*"assertion "*|*"Error:"*)
        printf '      %s%s%s\n' "$DIM" "$line" "$RESET"
        ;;
    esac
  done < <(cargo test -p "$pkg" --no-fail-fast -- --test-threads=1 2>&1)

  TOTAL_PASS=$((TOTAL_PASS + pass))
  TOTAL_FAIL=$((TOTAL_FAIL + fail))

  if [ "$fail" -eq 0 ]; then
    printf '\n  %s%s%s: %s%d passed%s\n' "$BOLD" "$label" "$RESET" "$GREEN" "$pass" "$RESET"
  else
    FAILED_PROGRAMS+=("$label")
    printf '\n  %s%s%s: %s%d passed%s, %s%d failed%s\n' \
      "$BOLD" "$label" "$RESET" "$GREEN" "$pass" "$RESET" "$RED" "$fail" "$RESET"
  fi
done

printf '\n%s%s%s\n' "$BOLD" "$BAR" "$RESET"
if [ "$TOTAL_FAIL" -eq 0 ]; then
  printf '%s  TOTAL: %s%d passed%s%s, 0 failed%s\n' "$BOLD" "$GREEN" "$TOTAL_PASS" "$RESET" "$BOLD" "$RESET"
else
  printf '%s  TOTAL: %s%d passed%s%s, %s%d failed%s%s  (%s)%s\n' \
    "$BOLD" "$GREEN" "$TOTAL_PASS" "$RESET" "$BOLD" "$RED" "$TOTAL_FAIL" "$RESET" \
    "$BOLD" "${FAILED_PROGRAMS[*]}" "$RESET"
fi
printf '%s%s%s\n\n' "$BOLD" "$BAR" "$RESET"

[ "$TOTAL_FAIL" -eq 0 ]
