#!/usr/bin/env bash
set -euo pipefail

terms_file="$(git rev-parse --git-common-dir)/info/forbidden-terms"
[[ -s "${terms_file}" ]] || exit 0

patterns="$(mktemp)"
trap 'rm -f "${patterns}"' EXIT
grep -v -E '^[[:space:]]*(#|$)' "${terms_file}" >"${patterns}" || true
[[ -s "${patterns}" ]] || exit 0

found=0

report() {
  echo "[forbidden-terms] $1" >&2
  found=1
}

matches() {
  grep -i -E -f "${patterns}" >/dev/null
}

check_staged() {
  local path line
  while IFS= read -r path; do
    if matches <<<"${path}"; then report "a staged file name matches a forbidden term: ${path}"; fi
  done < <(git diff --cached --name-only --diff-filter=ACMR)
  while IFS=$'\t' read -r added _ path; do
    [[ "${added}" == "-" ]] && continue
    while IFS= read -r line; do
      report "${path}:${line%%:*} holds a forbidden term"
    done < <(git show ":${path}" | grep -n -i -E -f "${patterns}" || true)
  done < <(git diff --cached --numstat --diff-filter=ACMR)
}

check_message() {
  local line
  while IFS= read -r line; do
    report "the commit message holds a forbidden term (line ${line%%:*})"
  done < <(grep -v '^#' "$1" | grep -n -i -E -f "${patterns}" || true)
}

check_push() {
  local zero local_sha remote_sha sha
  local -a range
  zero="$(git hash-object --stdin </dev/null | tr '0-9a-f' '0')"
  while read -r _ local_sha _ remote_sha; do
    [[ "${local_sha}" == "${zero}" ]] && continue
    if [[ "${remote_sha}" == "${zero}" ]]; then
      range=("${local_sha}" --not --remotes)
    else
      range=("${remote_sha}..${local_sha}")
    fi
    while IFS= read -r sha; do
      if git log -1 --format=%B "${sha}" | matches; then report "commit ${sha:0:12} has a forbidden term in its message"; fi
      if git show --format= --no-color --diff-filter=ACMR "${sha}" | grep -E '^\+' | grep -v -E '^\+\+\+ ' | matches; then
        report "commit ${sha:0:12} adds a line with a forbidden term"
      fi
    done < <(git rev-list "${range[@]}")
  done
}

check_text() {
  if matches; then report "the text holds a forbidden term"; fi
}

case "${1:-}" in
  pre-commit) check_staged ;;
  commit-msg) check_message "$2" ;;
  pre-push) check_push ;;
  text) check_text ;;
  *)
    echo "usage: forbidden-terms.sh pre-commit | commit-msg <file> | pre-push | text" >&2
    exit 2
    ;;
esac

if ((found)); then
  echo "[forbidden-terms] the local list in ${terms_file} names terms that never leave this machine" >&2
  exit 1
fi
