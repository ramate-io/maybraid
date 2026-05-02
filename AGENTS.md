# Project Instructions

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages.

1. Store the proposed commit message in the `.gitignored` file `commits/{working-branch}`.
2. When asked to summarize changes since the last commit, read that file first and create it if it does not exist.
3. Keep the commit-ready summary at the top concise and focused on the reasoning behind the change. For most small changes, use a topical summary line followed by one to three explanatory sentences.
4. Use Markdown formatting freely when it improves readability. MathJax is also welcome when mathematical notation clarifies the discussion; reviewers can open the file somewhere that supports rich Markdown rendering.
5. Append an ongoing dialogue log at the bottom of the same file as the agent answers questions or makes any changes. Keep this log append-only, even when the commit-ready summary above is rewritten or condensed.
6. Reference issues, pull requests, and other source control links when they add useful narrative context. Do not rely on those references as the substance of the commit message; the text should still explain the change if those platforms or links change later.
7. The developer creates the commit with `git commit -F "commits/$(git branch --show-current)"`.