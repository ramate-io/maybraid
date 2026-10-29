# Labels

[`labels.yml`](labels.yml) keeps the repository issue labels in sync with a
hard-coded list. It does not attach labels to issues; it only creates, updates,
and (sometimes) deletes label definitions.

## When it runs

- **Push** that changes `.github/workflows/labels.yml` (any branch).
- **`workflow_dispatch`** from the Actions tab.

It does not run on every commit.

## What it does

`LABELS_JSON` in the workflow is the source of truth. For each entry the job
tries `issues.createLabel`. If the name already exists, it updates color and
description to match the JSON.

That create-or-update pass always runs.

## Deletion

Labels that exist on the repo but are **not** in `LABELS_JSON` are deleted only
when:

- the run is on **`main`**, or
- the triggering commit message contains `debug(ci:labels:deletion)`.

A feature-branch edit of `labels.yml` therefore adds or updates names without
stripping other labels. Merging that change to `main` (or a dispatch from
`main`) will delete anything extra.

Do not put one-off labels on the GitHub UI if you expect them to survive a
`main` labels run.

## Canonical names

| Name | Role |
|---|---|
| `event` | Dated release, demo, or similar. Other work justifies up to events. |
| `release-candidate` | Intended usable version of software or docs. |
| `feature` | New behavior. |
| `bug` | Behavior that needs to be fixed. |
| `proposal` | Detailed proposal (RFC). |
| `docs` | Documentation. |
| `delivery` | Delivery mechanisms. |
| `validation` | Validation mechanisms. |
| `priority:low` | Do not take over higher-priority work. |
| `priority:medium` | Work sparingly when other issues are available. |
| `priority:high` | Prefer over most other issues. |
| `priority:urgent` | Prefer over everything else. |

Issue process (events → release candidates → features/bugs) is in
[CONTRIBUTING.md](../../CONTRIBUTING.md).
