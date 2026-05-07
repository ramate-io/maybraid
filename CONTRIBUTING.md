# Contributing

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

Each issue should have a priority. All Releases Candidates should eventually link up to Events. All Features and Bugs should eventually link up to Release Candidates. 

Proposals issue labels are generally used to mark requests for research.

All issues should tag the following projects:

- `Ramate`: https://github.com/orgs/ramate-io/projects/2
- `Maybraid`: https://github.com/orgs/ramate-io/projects/17

## Events

Events should be titled as follows:

```
Event: <name of the event>
```

Events should be formatted in Markdown as follows:

```
# Summary
- **Target date:** <insert date>

Two to three line description of the event.

- Bulleted
- Line Item
- Callouts

Any other description as the writer sees fit...
```

## Release Candidates

Release Candidates should be titled as follows:

```
Release Candidate: <name of the release candidate>
```

Release Candidates should be formatted as follows:

```
# Summary

This Release Candidate is intended to enable the following events:

- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)

Two to three line description of the release candidate. 

- Bulleted
- Line Item
- Callout

Any other description as the writer sees fit...
```

## Features & Bugs

A Feature or Bug should be titled as follows:

```
<One sentence description of what the bug or feature does.>
```

Features or Bugs should be formatted as follows:

```
# Summary

<One sentence description of what the bug or feature does.>

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

## Proposals 

A Proposal should be titled as follows:

```
<One sentence description of the problem which should be proposed against.>
```

Proposals should be formatted as follows:

```
# Summary

<One sentence description of the problem which should be proposed against.>.

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

Proposals will generally be satisfied by additions to repository documentation or inclusion of a written proposal in another knowledge base.

## Rust tests

Do **not** use [`.unwrap()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap), [`.expect(...)`](https://doc.rust-lang.org/std/option/enum.Option.html#method.expect), or [`panic!(...)`](https://doc.rust-lang.org/std/macro.panic.html) in test bodies—those snippets are often copied into production code and keep failing habits.

Prefer **`Result`** propagation instead: write helpers that return something like **`anyhow::Result`** (or your crate’s error type), use **`?`**, and declare **`#[test] fn case() -> anyhow::Result<()>`**, so harness failures surface structured errors. [`assert!`](https://doc.rust-lang.org/std/macro.assert.html) / [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html) remain appropriate for expectations.

Common external knowledge bases include:

- [OAC](https://github.com/ramate-io/oac)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Ramate](https://github.com/ramate-io/ramate)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Robles](https://github.com/ramate-io/robles)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.

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