# Contributing

## Start with an issue

Every pull request links to an [issue](https://github.com/raaqimorg/qafiyah/issues). For a bug
or an idea you don't plan to work on, the form asks only for what happened and its impact. For
security vulnerabilities, see [SECURITY.md](SECURITY.md) instead.

If you plan to send the fix, open the issue first (or pick an existing one) and fill in the rest
of the form:

- **Impact:** what breaks, and the worst case if it stays unfixed.
- **Root cause:** which code causes the bug or the limitation.
- **Proposed fix:** a snippet or a clear description.
- **Reasoning:** why the fix addresses the underlying problem.
- **Trade-offs:** whether it could introduce new bugs, mask others, or only work around the
  issue, and what else the changed code touches.
- **Demo:** for UI changes, screenshots or a video in the form's Before and After table.

Labels scope an issue or pull request: one type (`🐛 bug`, `✨ enhancement`, ...) plus every
component and topic it touches, from the [label list](https://github.com/raaqimorg/qafiyah/labels).
GitHub lets only people with write or triage access set labels, so list the ones that fit in the
form's Labels field or the pull request template's Labels section, and a maintainer applies them.
The form adds its type label by itself when you submit it in the browser, and a pull request gets
its component and topic labels automatically from the files it changes.

For a large or significant change, wait until the maintainers agree on the approach in the issue
before writing code, so your effort isn't wasted. They mark that agreement with the `✅ accepted`
label. Keep each pull request small and focused on one concern; split unrelated changes into
separate issues and pull requests.

## Making changes

1. Fork the repo and create a branch off the open version branch, not `main`. There is one at a time, named `v` and a number (`v2`, `v3`, and so on); `git ls-remote --heads origin 'v*'` shows it. Changes reach `main`, and the live site, when that version is released ([`pull-requests.md`](../docs/pull-requests.md), "Versions and releases").
2. Follow the conventions in `docs/`: [`code-conventions.md`](../docs/code-conventions.md),
   [`typescript-conventions.md`](../docs/typescript-conventions.md),
   [`rust-conventions.md`](../docs/rust-conventions.md), and
   [`testing.md`](../docs/testing.md).
3. Set up the repo and run it locally as described in [`docs/development.md`](../docs/development.md).
4. Run `bun run ci` before opening a PR. It is the full gate; GitHub Actions runs the same gate on every push and PR, skipping the Docker phases a change doesn't touch.
5. Follow [`pull-requests.md`](../docs/pull-requests.md) for commit message and PR conventions.
6. `README.md`'s "Documentation map" lists where everything is documented; update the doc that describes what you changed.
7. Open the pull request into the open version branch with `Closes #<issue>` in its description, and fill in the template. The issue closes when the version is released, not when your pull request merges.

## Working with an AI agent

Install the [GitHub CLI](https://cli.github.com) (`gh`) and run `gh auth login`. With it, an agent
can read an issue and its discussion (`gh issue view <number> --comments`), open an issue
(`gh issue create`), and open a pull request linked to it (`gh pr create`), with no copying
between the terminal and the browser. The repo's `AGENTS.md` points agents to
[`pull-requests.md`](../docs/pull-requests.md), which holds the same rules as this file.

## Keeping your email private

Every commit carries an author email, and pushing publishes it. To make sure this clone only ever commits with the address you meant, for example a GitHub no-reply address rather than a personal or work one:

1. Set that address for this clone only: `git config --local user.email <address>`.
2. Turn on the guard: `git config --local qafiyah.allowedEmail "$(git config --local user.email)"`. The git hooks then refuse any commit, and any push, whose author, committer, or `Co-authored-by:` line uses a different address, including one an AI agent or a script sets. The address stays in `.git/config` and is never committed.
3. On GitHub, under Settings, then Emails, turn on "Keep my email addresses private" and "Block command line pushes that expose my email", so GitHub also rejects them on push.

Details and limits: [`docs/development.md`](../docs/development.md) ("Committing").

By contributing, you agree that your code and documentation contributions will be licensed under
the project's [MIT license](../LICENSE), and that your data contributions (poems, poets, and
corrections to them) will be dedicated to the public domain under [CC0 1.0](../data/LICENSE).
