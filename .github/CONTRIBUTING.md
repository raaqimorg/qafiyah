# Contributing

## Start with an issue

Every pull request links to an [issue](https://github.com/raaqimorg/qafiyah/issues). To report a bug or an idea that you do not plan to work on, the form asks only for what happened and its impact. For a security vulnerability, see [SECURITY.md](SECURITY.md) instead.

If you plan to send the fix, open the issue first, or pick an existing one. Then fill in the rest of the form:

- **Impact:** what breaks, and the worst case if nobody fixes it.
- **Root cause:** the code that causes the bug or the limitation.
- **Proposed fix:** a snippet or a clear description.
- **Reasoning:** why the fix solves the underlying problem.
- **Trade-offs:** whether the fix can add new bugs, hide other bugs, or only work around the issue. Also say what else the changed code touches.
- **Screenshots or video:** for a UI change, put what you see in the Before column. Put a mockup of the fix in the After column.

Labels give the scope of an issue or a pull request. Use one type (`🐛 bug`, `✨ enhancement`, and so on), and every component and topic that it touches. Pick them from the [label list](https://github.com/raaqimorg/qafiyah/labels). On GitHub, only people with write or triage access can set labels. So write the labels that fit in the form's Labels field, or in the template's Labels section. A maintainer then applies them. When you submit the form in the browser, it adds its type label itself. A pull request gets its component and topic labels automatically from the files that it changes.

For a large or significant change, wait until the maintainers agree on the approach in the issue before you write code. This way, your work is not wasted. The maintainers show their agreement with the `✅ accepted` label. Keep each pull request small, with one concern. Put unrelated changes in separate issues and pull requests.

## Making changes

1. Fork the repo. Create a branch off the open version branch, not off `main`.
   - Only one version branch is open at a time. Its name is `v` and a number (`v2`, `v3`, and so on).
   - To find it, run `git ls-remote --heads origin 'refs/heads/v[0-9]*'`.
   - Your change reaches `main`, and the live site, when that version is released. See [`pull-requests.md`](../docs/pull-requests.md) ("Versions and releases").
2. Follow the conventions in `docs/`: [`code-conventions.md`](../docs/code-conventions.md), [`typescript-conventions.md`](../docs/typescript-conventions.md), [`rust-conventions.md`](../docs/rust-conventions.md), and [`testing.md`](../docs/testing.md).
3. Set up the repo and run it locally. See [`docs/development.md`](../docs/development.md).
4. Run `bun run ci` before you open a pull request. It is the full gate. GitHub Actions runs the same gate on every push and pull request, and skips the Docker phases that a change does not touch.
5. Follow [`pull-requests.md`](../docs/pull-requests.md) for commit messages and pull requests.
6. Update the doc that describes what you changed. The "Documentation map" in `README.md` lists where each topic is documented.
7. Open the pull request into the open version branch. Put `Closes #<issue>` in its description, and fill in the template. The issue closes when the version is released, not when your pull request merges.

## Working with an AI agent

Install the [GitHub CLI](https://cli.github.com) (`gh`), and run `gh auth login`. With it, an agent can do these tasks without copying text between the terminal and the browser:

- Read an issue and its discussion: `gh issue view <number> --comments`.
- Open an issue: `gh issue create`.
- Open a pull request that links to the issue: `gh pr create`.

The repo's `AGENTS.md` points agents to [`pull-requests.md`](../docs/pull-requests.md), which holds the same rules as this file.

## Keeping your email private

Every commit carries an author email, and a push publishes it. Use these steps to make this clone commit only with the address that you choose. For example, use a GitHub no-reply address, not a personal or work address.

1. Set that address for this clone only: `git config --local user.email <address>`.
2. Turn on the guard: `git config --local qafiyah.allowedEmail "$(git config --local user.email)"`.
   - The git hooks then refuse every commit and every push that uses a different address as author, committer, or `Co-authored-by:` line. This includes an address that an AI agent or a script sets.
   - The address stays in `.git/config`. It is never committed.
3. On GitHub, go to Settings, then Emails. Turn on "Keep my email addresses private" and "Block command line pushes that expose my email". GitHub then also refuses such pushes.

For details and limits, see [`docs/development.md`](../docs/development.md) ("Committing").

## License

By contributing, you agree to these terms:

- Your code and documentation contributions are licensed under the project's [MIT license](../LICENSE).
- Your data contributions (poems, poets, and corrections to them) are dedicated to the public domain under [CC0 1.0](../data/LICENSE).
