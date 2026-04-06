#!/usr/bin/env bash
# Create a GitHub issue from a small JSON manifest (body markdown + metadata).
# See publish-gh-issue.md for schema and examples.
set -euo pipefail

usage() {
  echo "Usage: $0 [--dry-run] <issue.json>" >&2
  echo "  Requires: gh (authenticated), jq." >&2
  echo "  Org projects: gh auth refresh -s project -s read:project" >&2
  exit 1
}

DRY_RUN=0
while [[ "${1:-}" == -* ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    -h | --help) usage ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      ;;
  esac
done

[[ $# -eq 1 ]] || usage

CONFIG_PATH=$1
if [[ ! -f "$CONFIG_PATH" ]]; then
  echo "Not a file: $CONFIG_PATH" >&2
  exit 1
fi

CONFIG=$(cd "$(dirname "$CONFIG_PATH")" && pwd)/$(basename "$CONFIG_PATH")
CONFIG_DIR=$(dirname "$CONFIG")

command -v jq >/dev/null 2>&1 || {
  echo "publish-gh-issue: jq is required (brew install jq)" >&2
  exit 1
}
command -v gh >/dev/null 2>&1 || {
  echo "publish-gh-issue: gh is required" >&2
  exit 1
}

REPO=$(jq -r '.repo // "ramate-io/maybraid"' "$CONFIG")
TITLE=$(jq -r '.title' "$CONFIG")
BODY_REL=$(jq -r '.body_file' "$CONFIG")

if [[ -z "$TITLE" || "$TITLE" == null ]]; then
  echo "publish-gh-issue: .title is required in $CONFIG" >&2
  exit 1
fi
if [[ -z "$BODY_REL" || "$BODY_REL" == null ]]; then
  echo "publish-gh-issue: .body_file is required in $CONFIG" >&2
  exit 1
fi

if [[ $BODY_REL == /* ]]; then
  BODY_ABS=$BODY_REL
else
  BODY_ABS="$CONFIG_DIR/$BODY_REL"
fi
if [[ ! -f "$BODY_ABS" ]]; then
  echo "publish-gh-issue: body file not found: $BODY_ABS" >&2
  exit 1
fi

create_cmd=(gh issue create -R "$REPO" --title "$TITLE" --body-file "$BODY_ABS")
while IFS= read -r lab; do
  [[ -n "$lab" ]] && create_cmd+=(-l "$lab")
done < <(jq -r '.labels[]? // empty' "$CONFIG")

PARENT=$(jq -r 'if .parent == null or .parent == "" then empty else .parent | tostring end' "$CONFIG")

DEFAULT_PROJECTS='[{"number":2,"owner":"ramate-io"},{"number":17,"owner":"ramate-io"}]'

if [[ $DRY_RUN -eq 1 ]]; then
  echo "Dry run — would run:"
  printf '  %q ' "${create_cmd[@]}"
  echo
  if [[ -n "${PARENT:-}" ]]; then
    echo "  addSubIssue parent=$PARENT (repo $REPO)"
  fi
  echo "  project item-add for:"
  jq -r --argjson d "$DEFAULT_PROJECTS" '(.projects // $d)[] | "    project \(.number) owner \(.owner)"' "$CONFIG"
  exit 0
fi

URL=$("${create_cmd[@]}")
NUM=${URL##*/}

if [[ -n "${PARENT:-}" ]]; then
  PARENT_NODE=$(gh api "repos/$REPO/issues/$PARENT" -q .node_id)
  CHILD_NODE=$(gh api "repos/$REPO/issues/$NUM" -q .node_id)
  gh api graphql \
    -f query='mutation($i: ID!, $s: ID!) { addSubIssue(input: {issueId: $i, subIssueId: $s}) { issue { number } subIssue { number } } }' \
    -f i="$PARENT_NODE" -f s="$CHILD_NODE" >/dev/null 2>&1 || true
fi

while IFS= read -r proj; do
  n=$(echo "$proj" | jq -r '.number')
  o=$(echo "$proj" | jq -r '.owner')
  gh project item-add "$n" --owner "$o" --url "$URL" 2>/dev/null || true
done < <(jq -c --argjson d "$DEFAULT_PROJECTS" '(.projects // $d)[]' "$CONFIG")

echo "$URL"
