# Contributing

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/roadline/issues?q=is%3Aissue%20state%3Aopen%20label%3Apriority%3Ahigh%2Cpriority%3Amedium%20label%3Aevent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/roadline/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/roadline/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%20label%3Apriority%3Aurgent%2Cpriority%3Ahigh) | High-priority `feature` and `bug` issues. |

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
> After `gh issue create`, **attach the release candidate as the parent** (sub-issue in the UI is not the same as a Markdown link) and add **`Ramate`**, **`Maybraid`** via Projects (new). `gh issue create -p …` often fails for org projects; use `gh project item-add` (needs `gh auth refresh -s project -s read:project`).
>
> ```bash
> # 1) Create issue (title + body file; labels as needed)
> gh issue create -R ramate-io/gwrdfa --title "…" --body-file body.md -l proposal -l priority:low
>
> # 2) Link new issue as sub-issue of the RC (parent = RC, child = new)
> PARENT_NODE=$(gh api repos/ramate-io/maybraid/issues/<RC#> -q .node_id)
> CHILD_NODE=$(gh api repos/ramate-io/maybraid/issues/<NEW#> -q .node_id)
> gh api graphql -f query='mutation($i: ID!, $s: ID!) { addSubIssue(input: {issueId: $i, subIssueId: $s}) { issue { number } subIssue { number } } }' -f i="$PARENT_NODE" -f s="$CHILD_NODE"
>
> # 3) Add to each org project (numbers match the list under “All issues should tag…” above)
> for n in 1 2 3 4; do gh project item-add "$n" --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>; done
> ```