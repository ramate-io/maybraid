# Project Instructions

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages.

Every time the Agent applies this commit-file workflow, it must first refresh the current branch and `HEAD`:

```bash
git branch --show-current
git rev-parse HEAD
```

Use those fresh values for all subsequent commit-file reads and writes. Do not reuse a path from earlier in the conversation; the developer may have committed since then.

1. Store the proposed commit message in the `.gitignored` file `commits/{working-branch}/{head}`, where `{head}` is the full current `HEAD` hash.
2. When asked to summarize changes since the last commit, read that branch/head file first and create it if it does not exist.
3. Keep the commit-ready summary at the top concise and focused on the reasoning behind the change. For most small changes, use a topical summary line followed by one to three explanatory sentences.
4. Use Markdown formatting freely when it improves readability. MathJax is also welcome when mathematical notation clarifies the discussion; reviewers can open the file somewhere that supports rich Markdown rendering.
5. Append an ongoing `## Agent Dialogue` section at the bottom of the same file as the agent answers questions or makes any changes. This section is scoped to the work since the current `HEAD` commit only; do not carry dialogue forward from older commit-message files after the developer commits. Keep it concise: summarize the developer/agent back-and-forth rather than transcribing it. Use script notation, so the exchange is easy to scan: `- **Developer:** ...` and `- **Agent:** ...`.
6. Reference issues, pull requests, RFC sections, and other source control links when they add useful narrative context, and write those references as Markdown links whenever a target is known. For example, prefer `[#178](https://github.com/ramate-io/maybraid/issues/178)` and `[RFC-170 4.7](rfc/rfc-000-000-170-terrain-detail/README.md#47-world-space-ground-color-noise)` over bare `#178` or `RFC-170.4.7`. Do not rely on those references as the substance of the commit message; the text should still explain the change if those platforms or links change later.
7. The Nix shell configures the repository's helper aliases with `git config --local include.path ../.gitconfig`.
8. The developer creates the commit with `git commit-std` and removes obsolete ignored commit-message files with `git commit-prune-std`.

## Understanding Changes

Because commit files are expected to preserve useful dialogue context, `git blame` can be especially helpful when trying to understand why code is shaped the way it is. Use it when history would clarify intent before changing nearby code.

## Rust tests

Avoid `.unwrap()` and `.expect(...)` in Rust test code (copied snippets tend to land in production). See [CONTRIBUTING.md § Rust tests](CONTRIBUTING.md#rust-tests).

## Prompting GitHub Actions

We currently use GitHub for source control. When work relates to a clear GitHub thread, such as an issue, pull request, or discussion, the Agent should consider whether a written GitHub update would help preserve context.

For comments:

1. Draft the proposed comment in `comments/{agent-chosen-name}`.
2. Tell the developer what the comment is for and provide the appropriate `gh` command to publish it.
3. Do not publish the comment unless the developer explicitly asks the Agent to do so.

For new issues, suggest one when it would clarify follow-up work, capture a discovered problem, or preserve a distinct implementation thread. Prefer the repository helper script:

> [!TIP]
> **Preferred:** use [`bin/publish-gh-issue.sh`](bin/publish-gh-issue.sh) with a small JSON manifest next to your Markdown body—see [`bin/publish-gh-issue.md`](bin/publish-gh-issue.md). That covers **`gh issue create`**, **sub-issue parent** (UI relationship—not the same as linking in the body), and **`Ramate` / `Maybraid` org projects** (defaults: project **2** and **17** on `ramate-io`). `gh issue create -p …` often fails for org projects; the script uses `gh project item-add` instead (needs `gh auth refresh -s project -s read:project`).
>
> ```bash
> ./bin/publish-gh-issue.sh issues/your-scope/issue.json
> ```
>
> **Manual sequence** (same end state as the script):
>
> ```bash
> # 1) Create issue (repo, title, body, labels—maybraid unless you override in JSON)
> gh issue create -R ramate-io/maybraid --title '…' --body-file body.md -l feature -l priority:medium
>
> # 2) Link new issue as sub-issue of the parent (parent = RC or Jersey epic, child = new)
> PARENT_NODE=$(gh api repos/ramate-io/maybraid/issues/<PARENT#> -q .node_id)
> CHILD_NODE=$(gh api repos/ramate-io/maybraid/issues/<NEW#> -q .node_id)
> gh api graphql -f query='mutation($i: ID!, $s: ID!) { addSubIssue(input: {issueId: $i, subIssueId: $s}) { issue { number } subIssue { number } } }' -f i="$PARENT_NODE" -f s="$CHILD_NODE"
>
> # 3) Org projects (numbers match “All issues should tag…” above: Ramate 2, Maybraid 17)
> gh project item-add 2 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> gh project item-add 17 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> ```