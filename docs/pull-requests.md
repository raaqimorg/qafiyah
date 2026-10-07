# Pull Requests

- Run lint, format, and tests through the task runner, not package by package.
- Write issues, pull requests, and commit messages by the "Writing" rules in `docs/code-conventions.md`.
- A commit message is a subject line only, at most 200 characters, and nothing else:
  - no body, no bulleted explanation, and no paragraph after a blank line
  - no `Co-authored-by` line, and no other AI-attribution line or signoff
  - If the message does not fit on one line, it is too long. The fix is a shorter message, not a different subject.
- Commit and push only as the identity that this clone uses.
  - Never pass `--author`, `git -c user.email=...`, or `GIT_AUTHOR_*` or `GIT_COMMITTER_*` variables.
  - Never use `--no-verify` or `HUSKY=0` to get past the commit identity guard (`docs/development.md`, "Committing").
  - If the guard refuses, stop and ask.

## Versions and releases

Versions are whole numbers: 1, 2, 3, and so on. Version 1 is the release of 2026-10-07. There are no tags and no GitHub Releases. While a version is open, it is a branch. After its release, it is a file in `docs/changelog/`.

- Only one version is open at a time. Its branch is named `v` and its number (`v2`). It is cut from `main` right after the previous release. To find it, run `git ls-remote --heads origin 'refs/heads/v[0-9]*'`.
- Every open version has a draft release pull request from `vN` into `main`.
  - Its title is `release: version N`, and its label is `🚀 release`.
  - Open it with the version's first change, because GitHub refuses a pull request between two identical branches: `gh pr create --base main --head vN --draft --title "release: version N" --label "🚀 release"`.
  - When a pull request merges into `vN`, add its `Closes #<number>` line and a one-line summary to the description of the release pull request.
- Branch off the open version branch, and open pull requests into it (`gh pr create --base v2`). They are squash-merged as usual. Only release pull requests (below), urgent fixes, and security advisories go into `main`.
- `Closes #<number>` in a pull request into a version branch does not close the issue. GitHub closes issues only on merges into the default branch, so the issue closes when the version is released.
- Dependabot opens its pull requests against `main`. Before you merge one, move it to the open version branch: `gh pr edit <number> --base v2`.
- An urgent fix is the exception. Use this path only when production is broken and the fix cannot wait for the next release:
  1. Branch off `main`, and open the pull request into `main`.
  2. When it merges, deploy `main`.
  3. List it as a hotfix, with its date, in the next version's changelog.
- A fix through a GitHub security advisory can only merge into `main`.
- After an urgent fix or a security fix lands on `main`, merge `main` into the open version branch, so the next release carries it. Run `git checkout v2 && git merge origin/main`, then push. Merge, do not rebase: the version branch is shared, and a rebase would rewrite the commits that every open pull request is built on.

To release version N, do these steps:

1. On `vN`, write `docs/changelog/vN.md` and commit it. Start from `git log --oneline main..vN`.
2. Check that the description of the release pull request has `Closes #<number>` for every issue that the version fixed. Then mark it ready: `gh pr ready <number>`.
3. When CI passes, merge it with a merge commit, not a squash, so each change keeps its own commit on `main`: `gh pr merge <number> --merge --subject "release: version N (#<number>)" --body ""`.
4. Deploy `main` with the deploy runbook (`.claude/skills/deploy/SKILL.md`).
5. Delete `vN` (`git push origin --delete vN`). Open the next version from `main` (`git push origin main:refs/heads/v<N+1>`). Its draft release pull request opens with its first change.

`docs/changelog/vN.md` holds these parts, in this order:

1. The title `# Version N`.
2. The release date and the commit that was deployed.
3. `## Changes`: one line for each merged pull request. Give what changed, then its pull request and issue numbers.
4. `## Release steps`: anything that the release needed beyond a plain deploy (a dump restore, a reindex). If it needed nothing more, write "A plain deploy."
5. `## Follow-ups`: the issues that the release opened.

## Issue first

These rules apply to every change that becomes a pull request. `.github/CONTRIBUTING.md` explains them for people. This section is the checklist for agents.

- Every pull request links an issue. First read the issue and its comments: `gh issue view <number> --comments`.
- If there is no issue, create one with `gh issue create` before you write code.
  - Use the headings of the matching form in `.github/ISSUE_TEMPLATE/` as the body.
  - Fill in the impact, the root cause, the proposed fix, the reasoning, and the trade-offs.
  - The trade-offs say what new bugs the fix can add and what bugs it can hide. They also say whether it only works around the issue, and what else the changed code touches.
- Give the issue its scope with labels. Run `gh label list`, and pick every label that fits: one `Type:` label, and each `Component:` and `Topic:` label that the change touches.
  - Pass them with `--label`, one flag for each label, only if you have write or triage access. To check, run `gh api repos/raaqimorg/qafiyah --jq .permissions`.
  - Without that access, GitHub drops the labels without a message. So write them under the `Labels` heading of the body instead, and a maintainer applies them.
  - The forms add their type label only when someone submits them in the browser, never through `gh`.
- For a large or significant change, stop after you create or comment on the issue. Wait until the maintainers agree on the approach there. They show their agreement with the `✅ accepted` label (`gh issue view <number> --json labels`).
- Keep one concern in each pull request. If the work finds an unrelated problem, open a separate issue for it. Do not add it to the current pull request.
- Open the pull request with `gh pr create`.
  - Fill in `.github/PULL_REQUEST_TEMPLATE.md`, with `Closes #<number>` as its first line.
  - Label it the same way as the issue: `--label` with write or triage access, or else the `Labels` section of the template.
  - Every pull request also gets component and topic labels automatically from the files that it changes (`.github/labeler.yml`). So the type label is the one that most needs you.
  - A UI change needs screenshots from before and after. `gh` cannot upload images, so ask the person you work with to add them.
