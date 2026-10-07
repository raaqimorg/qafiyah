# Data

This directory holds versioned binary snapshots, with one subdirectory for each kind:

- `db/`: PostgreSQL dumps. Scripts handle them (`bun run db:reset`, `bun run db:reseed`). They are encrypted, and `bun run dev` seeds itself from them. See `db/README.md` and `db/MAINTAINERS_GUIDE.md`.
- `avatars/`: zips of the poet avatar images. R2 serves a copy at `cdn.qafiyah.com`. They are manual today, with the same directory names and encryption as `db/`. See `avatars/README.md`.

Both use the same layout: `{category}/{sequence}_{DD}_{MM}_{YYYY}/`. The encrypted `.enc` files are committed, and git ignores the plaintext.

Everything here is public, and dedicated to the public domain under [CC0 1.0](LICENSE). This is different from the code in the rest of the repo, which is under MIT. The `.enc` files are encrypted, not restricted.

Why encrypt something that is open? Because nobody can edit a public git history after it is pushed.

- After a plaintext copy is pushed, every fork, clone, and mirror keeps it for good. Nothing that the maintainers do after that can take it back.
- If a record must come out, for any reason, it must be possible to remove it everywhere that it went.
- Encryption keeps that possible. The plaintext copies are the ones given out on request. So there is always a way to reach each person who holds one.

A passphrase is quick to get. Email dumps@qafiyah.com for a `db/` snapshot, or avatars@qafiyah.com for an `avatars/` snapshot. Say what you need it for: your own use, or a contribution that you plan. You get the passphrase at once. There is no vetting, and nobody has to qualify. The README of each subdirectory says the same, in full.

## Moving or renaming anything here

Push that commit with `git -c pack.useSparse=false push`.

Git's sparse pack algorithm is on by default. It packs every blob under a renamed path again. So a directory rename here builds a pack of several GB, even when the real diff is a few KB. GitHub refuses it with `pack exceeds maximum allowed size (2.00 GiB)`. For example, the move from `dumps/` to `data/db/` made a 2.6 GB pack with the sparse algorithm on, and a 14 KB pack with it off.

Only the push that carries the rename has this problem.
