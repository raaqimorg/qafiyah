---
name: reviewing
description: Use when the user asks for a review of anything in this repo (a diff, branch, commit, PR, plan, spec, or finished task), including "review this", "look it over", "sanity check", or "does this look right".
---

# Reviewing

A review here is one light sanity pass that you do yourself, in the current session, the way Linus Torvalds reads a patch: read the change, spot what is actually wrong, say it plainly, done.

It replaces `superpowers:requesting-code-review`, the `code-review` and `simplify` skills, and any subagent or workflow review. Use those only when the user names them.

## The pass

1. Get the change (`git diff`, `git diff main...HEAD`, `gh pr diff <n>`, or the file the user points at). Read it once, top to bottom, with just enough surrounding code to follow it.
2. Hold it against four questions:
   - Does it work? Wrong logic, a missed edge case, a broken error path, anything that fails in production.
   - Does it break what already works? Callers, API responses, URLs, page output.
   - Is it in good taste? Special cases a better data shape would remove, needless abstraction, clever code, anything "Keep it boring" in `AGENTS.md` rules out.
   - Does it follow the repo rules? No comments, no em-dashes, the conventions in `docs/`.
3. If one finding needs confirming, a quick targeted check (a grep, one test) is fine. Then stop.

## The output

- A one-line verdict first: "Ship it", "Ship after fixing X", or "No, because Y".
- Then the problems that matter, most serious first, each as `file:line` and one sentence on what breaks. Usually zero to five.
- That is the whole review. Report, don't fix, unless the user asked for fixes.

Blunt about the code, never about the person.
