# Pull Requests

- Run lint/format/test through the task runner, not per-package.
- Commit messages: subject line only, max 200 characters, nothing else. No body, no bulleted explanation, no blank-line-separated paragraph, no `Co-authored-by` or any other AI-attribution line/signoff. If it doesn't fit in one line, the message is too long, not the subject wrong.
- Commit and push only as the identity this clone is configured with. Never pass `--author`, `git -c user.email=...`, or `GIT_AUTHOR_*`/`GIT_COMMITTER_*` variables, and never use `--no-verify` or `HUSKY=0` to get past the commit identity guard (`docs/development.md`, "Committing"). If it refuses, stop and ask.

## Issue first

These rules apply to any change that will be opened as a pull request. `.github/CONTRIBUTING.md` explains them for people; this section is the checklist for agents.

- Every pull request links an issue. Read it and its comments first: `gh issue view <number> --comments`.
- If there is no issue, create one with `gh issue create` before writing code. Use the headings of the matching form in `.github/ISSUE_TEMPLATE/` as the body, and fill in impact, root cause, proposed fix, reasoning, and trade-offs (new bugs it could introduce, bugs it could mask, whether it only works around the issue, and what else the changed code touches).
- For a large or significant change, stop after creating or commenting on the issue and wait until the maintainers agree on the approach there, which they mark with the `✅ accepted` label (`gh issue view <number> --json labels`).
- One concern per pull request. If the work turns up an unrelated problem, open a separate issue for it instead of folding it in.
- Open the pull request with `gh pr create`, filling in `.github/PULL_REQUEST_TEMPLATE.md` with `Closes #<number>` as its first line. UI changes need before and after screenshots; `gh` cannot upload images, so ask the person you are working with to add them.
