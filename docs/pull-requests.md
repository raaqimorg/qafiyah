# Pull Requests

- Run lint/format/test through the task runner, not per-package.
- Commit messages: subject line only, max 200 characters, nothing else. No body, no bulleted explanation, no blank-line-separated paragraph, no `Co-authored-by` or any other AI-attribution line/signoff. If it doesn't fit in one line, the message is too long, not the subject wrong.
- Commit and push only as the identity this clone is configured with. Never pass `--author`, `git -c user.email=...`, or `GIT_AUTHOR_*`/`GIT_COMMITTER_*` variables, and never use `--no-verify` or `HUSKY=0` to get past the commit identity guard (`docs/development.md`, "Committing"). If it refuses, stop and ask.

## Versions and releases

Versions are whole numbers: 1, 2, 3, and so on. Version 1 is the release of 2026-10-07. There are no tags and no GitHub Releases; a version is a branch while it is open and a file in `docs/changelog/` once it is released.

- One version is open at a time, on a branch named `v` and its number (`v2`). It is cut from `main` right after the previous release. `git ls-remote --heads origin 'v*'` shows it.
- Branch off the open version branch and open pull requests into it (`gh pr create --base v2`). They are squash-merged as usual. Only release pull requests (below), urgent fixes, and security advisories go into `main`.
- `Closes #<number>` in a pull request into a version branch does not close the issue, because GitHub closes issues only on merges into the default branch. The issue closes when the version is released.
- Dependabot opens its pull requests against `main`. Retarget each one to the open version branch before merging: `gh pr edit <number> --base v2`.
- An urgent fix that cannot wait for the next release, because production is broken, is the exception: branch off `main`, open the pull request into `main`, and deploy `main` once it merges. List it as a hotfix, with its date, in the next version's changelog.
- A fix made through a GitHub security advisory can only merge into `main`.
- After an urgent fix or a security fix lands on `main`, merge `main` into the open version branch (`git checkout v2 && git merge origin/main`, then push) so the next release carries it. Merge rather than rebase: the version branch is shared, and rebasing it would rewrite the commits every open pull request is built on.

Releasing version N:

1. On `vN`, write `docs/changelog/vN.md` and commit it. Start from `git log --oneline main..vN`.
2. Open a pull request from `vN` into `main` titled `release: version N`, whose description has `Closes #<number>` for every issue the version fixed.
3. When CI passes, merge it with a merge commit, not a squash, so each change keeps its own commit on `main`: `gh pr merge <number> --merge --subject "release: version N (#<number>)" --body ""`.
4. Deploy `main` with the deploy runbook (`.claude/skills/deploy/SKILL.md`).
5. Delete `vN` (`git push origin --delete vN`) and open the next version from `main` (`git push origin main:refs/heads/v<N+1>`).

`docs/changelog/vN.md` holds, in this order: the title `# Version N`, the release date and the commit that was deployed, `## Changes` with one line per merged pull request (what changed, then its pull request and issue numbers), `## Release steps` with anything the release needed beyond a plain deploy (a dump restore, a reindex) or "A plain deploy.", and `## Follow-ups` with the issues the release opened.

## Issue first

These rules apply to any change that will be opened as a pull request. `.github/CONTRIBUTING.md` explains them for people; this section is the checklist for agents.

- Every pull request links an issue. Read it and its comments first: `gh issue view <number> --comments`.
- If there is no issue, create one with `gh issue create` before writing code. Use the headings of the matching form in `.github/ISSUE_TEMPLATE/` as the body, and fill in impact, root cause, proposed fix, reasoning, and trade-offs (new bugs it could introduce, bugs it could mask, whether it only works around the issue, and what else the changed code touches).
- Scope the issue with labels: run `gh label list` and pick every label that fits, one `Type:` label plus each `Component:` and `Topic:` label the change touches. Pass them with `--label` (one flag per label) only if you have write or triage access (`gh api repos/raaqimorg/qafiyah --jq .permissions`); without it GitHub silently drops them, so fill in the body's `Labels` heading with them instead and a maintainer applies them. The forms add their type label only when submitted in the browser, never through `gh`.
- For a large or significant change, stop after creating or commenting on the issue and wait until the maintainers agree on the approach there, which they mark with the `✅ accepted` label (`gh issue view <number> --json labels`).
- One concern per pull request. If the work turns up an unrelated problem, open a separate issue for it instead of folding it in.
- Open the pull request with `gh pr create`, filling in `.github/PULL_REQUEST_TEMPLATE.md` with `Closes #<number>` as its first line, and label it the same way as the issue (`--label` with write or triage access, the template's `Labels` section otherwise). Component and topic labels are also added to every pull request automatically from the files it changes (`.github/labeler.yml`), so the type label is the one that most needs you. UI changes need before and after screenshots; `gh` cannot upload images, so ask the person you are working with to add them.
