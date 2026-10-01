#!/usr/bin/env bash
set -euo pipefail

allowed="$(git config --local --get qafiyah.allowedEmail || true)"
[[ -n "${allowed}" ]] || exit 0

refuse() {
  echo "[commit-identity] $1" >&2
  echo "[commit-identity] only the address in this clone's qafiyah.allowedEmail may author, commit, or co-author here" >&2
  exit 1
}

email_of() {
  sed -E 's/^.*<([^>]*)>.*$/\1/'
}

check_staged() {
  [[ "$(git var GIT_AUTHOR_IDENT | email_of)" == "${allowed}" ]] || refuse "the author email is not the allowed one"
  [[ "$(git var GIT_COMMITTER_IDENT | email_of)" == "${allowed}" ]] || refuse "the committer email is not the allowed one"
}

check_commit() {
  local sha="$1"
  [[ "$(git log -1 --format=%ae "${sha}")" == "${allowed}" ]] || refuse "commit ${sha:0:12} has a different author email"
  [[ "$(git log -1 --format=%ce "${sha}")" == "${allowed}" ]] || refuse "commit ${sha:0:12} has a different committer email"
  local trailer
  while IFS= read -r trailer; do
    [[ "$(email_of <<<"${trailer}")" == "${allowed}" ]] || refuse "commit ${sha:0:12} has a trailer with a different email"
  done < <(git log -1 --format=%B "${sha}" | grep -E '^[A-Za-z-]+-[Bb]y:.*<[^>]*@[^>]*>' || true)
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
      range=("${remote_sha}..${local_sha}" --not --remotes)
    fi
    while read -r sha; do
      check_commit "${sha}"
    done < <(git rev-list "${range[@]}")
  done
}

case "${1:-}" in
  pre-commit) check_staged ;;
  pre-push) check_push ;;
  *)
    echo "usage: $0 pre-commit|pre-push" >&2
    exit 2
    ;;
esac
