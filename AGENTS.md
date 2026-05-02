# Project Instructions

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages.

1. Store the proposed commit message in the `.gitignored` file `commits/{working-branch}/{head}`, where `{head}` is the full current `HEAD` hash from `git rev-parse HEAD`.
2. When asked to summarize changes since the last commit, read that branch/head file first and create it if it does not exist.
3. Keep the commit-ready summary at the top concise and focused on the reasoning behind the change. For most small changes, use a topical summary line followed by one to three explanatory sentences.
4. Use Markdown formatting freely when it improves readability. MathJax is also welcome when mathematical notation clarifies the discussion; reviewers can open the file somewhere that supports rich Markdown rendering.
5. Append an ongoing dialogue log at the bottom of the same file as the agent answers questions or makes any changes. Keep the log concise: summarize the developer/agent back-and-forth rather than transcribing it.
6. Reference issues, pull requests, and other source control links when they add useful narrative context. Do not rely on those references as the substance of the commit message; the text should still explain the change if those platforms or links change later.
7. The Nix shell configures the repository's helper aliases with `git config --local include.path ../.gitconfig`.
8. The developer creates the commit with `git commit-std` and removes obsolete ignored commit-message files with `git commit-prune-std`.

## Understanding Changes

Because commit files are expected to preserve useful dialogue context, `git blame` can be especially helpful when trying to understand why code is shaped the way it is. Use it when history would clarify intent before changing nearby code.