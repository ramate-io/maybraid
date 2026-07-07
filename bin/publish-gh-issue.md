# `publish-gh-issue.sh`

Script: [`publish-gh-issue.sh`](publish-gh-issue.sh). It creates a GitHub issue from a **JSON manifest** plus a **Markdown body file** so agents (or humans) can commit both and run one command to publish—matching [maybraid#108 — Standardize the GH issue creation agent workflow](https://github.com/ramate-io/maybraid/issues/108).

## Requirements

- [GitHub CLI](https://cli.github.com/) `gh`, authenticated for the target org/repo.
- [`jq`](https://jqlang.org/) for parsing the manifest.
- **Org projects:** if `gh project item-add` fails, refresh scopes:  
  `gh auth refresh -s project -s read:project`

## Usage

```bash
./bin/publish-gh-issue.sh path/to/issue.json
./bin/publish-gh-issue.sh --dry-run path/to/issue.json
```

Paths are usually next to draft bodies under `issues/`—which is not tracked by Git.

## JSON schema

| Field | Required | Default | Description |
|--------|----------|---------|-------------|
| `title` | yes | — | Issue title (use an **imperative** title per [CONTRIBUTING.md](../CONTRIBUTING.md)). |
| `body_file` | yes | — | Path to the Markdown body. Relative paths are resolved from the **directory containing the JSON file**. |
| `repo` | no | `ramate-io/maybraid` | `owner/name` passed to `gh issue create -R`. |
| `labels` | no | none | JSON array of label names, e.g. `["feature", "priority:medium"]`. |
| `parent` | no | none | Parent issue **number** for the GitHub **sub-issue** link (child = newly created issue). Omit for top-level issues. |
| `projects` | no | Ramate `2` and Maybraid `17` under `ramate-io` | Array of `{ "number": <org project #>, "owner": "<org or user>" }`. |

## Example manifest

`issues/example/issue.json` (hypothetical):

```json
{
  "repo": "ramate-io/maybraid",
  "title": "Implement 4.2 alpha noise and stamping API",
  "body_file": "body.md",
  "labels": ["feature", "priority:medium"],
  "parent": 72,
  "projects": [
    { "number": 2, "owner": "ramate-io" },
    { "number": 17, "owner": "ramate-io" }
  ]
}
```

If you omit `projects`, the script still adds **2** and **17** for `ramate-io` (see [CONTRIBUTING.md](../CONTRIBUTING.md) — Ramate / Maybraid boards).

## Behavior

1. **`gh issue create`** with title, body file, and labels.
2. If **`parent`** is set, runs the **`addSubIssue`** GraphQL mutation (parent = that issue, child = new issue). Conflicts (already linked, duplicate) are ignored, so the script is re-runnable for project steps only in a pinch.
3. For each configured project, **`gh project item-add`** with the new issue URL. **“Content already exists”** is ignored.

The script prints the **new issue URL** on success.

## Checking in

Keep **`issue.json` + `body.md`** (or your chosen names) in the repo as the source of truth; re-run the script only when you intend to **create** a new issue. To update an existing issue, edit on GitHub or use `gh issue edit`; do not re-run publish for the same content unless you want a duplicate.
