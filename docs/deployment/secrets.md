# Secrets

The secrets are committed to git, encrypted with [SOPS](https://github.com/getsops/sops) and [age](https://github.com/FiloSottile/age). Nobody edits `.env` by hand: it is generated from the encrypted file.

| File                   | Who can decrypt      | Becomes                        |
| ---------------------- | -------------------- | ------------------------------ |
| `secrets/dev.enc.env`  | your laptop          | `.env` on your laptop          |
| `secrets/prod.enc.env` | your laptop, the VPS | `/opt/qafiyah/.env` on the VPS |

This page is for machines that hold an age key: the maintainers' laptops and the VPS. A contributor without a key never runs `secrets:*`. For them, `.env` is a plain file that they write themselves; see `docs/development.md`.

`.sops.yaml` lists the age **public** keys that can decrypt each file. Each machine keeps its own **private** key at `~/.config/sops/age/keys.txt`, with mode `600`. A private key never leaves the machine where it was generated. The one exception is the backup of the laptop key in the password manager.

## Important

- **Back up the laptop's private key.** Save the contents of `~/.config/sops/age/keys.txt` in the password manager. If that file is lost and no other recipient can decrypt, every committed secret is lost for good. This includes the `DUMP_KEY__*` passphrases that unlock `data/db/`.
- **A new VPS needs its key before the first deploy.** `bun run deploy` and `bun run db:reseed` fail on a server that has no `sops`, no age key, or no recipient line in `.sops.yaml`. Follow "Setting up a new production server" below first.
- **Never edit `.env` by hand**, on the laptop or on the VPS. The next `secrets:pull` or deploy overwrites it. Edit with `bun run secrets:edit` instead.
- **Never commit a decrypted file.** Only `secrets/*.enc.env` belongs in git.
- **Add a new variable to `scripts/secrets/schema.ts` first.** The check refuses every key that the schema does not list. This is what catches typos.

## Daily use

```bash
bun run secrets:edit          # edit dev in $EDITOR, then generate .env again
bun run secrets:edit prod     # edit prod, then commit, release, and run bun run deploy
bun run secrets:pull          # generate .env again from secrets/dev.enc.env
bun run secrets:check         # check both files against scripts/secrets/schema.ts
bun run dump:key:set          # write DUMP_KEY__* into secrets/dev.enc.env
```

`bun run deploy` and `bun run db:reseed` run `scripts/secrets/pull.sh prod` on the VPS right after they check out `origin/main`. So a committed change to `secrets/prod.enc.env` reaches production on the next deploy.

## Validation

`scripts/secrets/schema.ts` is the single list of allowed keys. For each key, it says whether dev and prod require, allow, or forbid it, and what format its value must have. `bun run secrets:check` enforces the schema:

- **Always, even without a key (CI):** no key is set twice, there is no unknown key, no key is forbidden in that environment, every required key is present, and every value is encrypted. The key names are plaintext in the encrypted file, so none of these checks needs decryption.
- **With an age key (laptop):** it also checks these things:
  - the value formats: 64-character hex for generated keys, 16 or more characters for production passwords, and `ENVIRONMENT` exactly `production`
  - credential pairs are set together
  - the backup bucket is never the public `qafiyah-assets` bucket
  - no two secrets hold the same value

  A problem names the key and never prints a value.

The check runs at these points:

- after every `secrets:edit` (an invalid save opens the editor again)
- in `dump:key:set`
- before `secrets:pull` writes the dev `.env`
- in `bun run ci`, which the pre-push hook and GitHub Actions run

The VPS does not run it, so the VPS needs no Bun. This is safe, because `bun run deploy` ships only a commit whose GitHub CI run passed.

The apps check again at startup. So a manual `docker compose` run with a broken `.env` still fails closed. In production, the API stops without `API_KEY_INTERNAL` or `API_KEY_FULL`, and the web container stops without `INTERNAL_API_KEY` or `SESSION_STATE_SECRET`.

## Setting up a machine

Install the tools, and generate the machine's key:

```bash
brew install sops age                     # macOS; on the VPS, use the release binaries (see below)
mkdir -p ~/.config/sops/age
(umask 077 && age-keygen -o ~/.config/sops/age/keys.txt)
```

`age-keygen` prints the public key (`age1...`). On a machine that can already decrypt, add the new key to the right rules in `.sops.yaml`. Then wrap the files again for the new recipient, and commit:

```bash
sops updatekeys secrets/prod.enc.env
sops updatekeys secrets/dev.enc.env
```

A machine cannot give itself access. A machine that is already a recipient must run `updatekeys`.

## Setting up a new production server

Production moved to SOPS on 2026-09-24. To give a new or rebuilt VPS access, do these steps in order. Do not run `bun run deploy` until step 4.

1. **On the VPS**, install `age` from apt and `sops` from its release binary, checked against its checksums. Then generate the VPS key:

   ```bash
   apt-get install -y age
   v=3.13.3
   curl -fsSLO "https://github.com/getsops/sops/releases/download/v${v}/sops-v${v}.linux.amd64"
   curl -fsSLO "https://github.com/getsops/sops/releases/download/v${v}/sops-v${v}.checksums.txt"
   grep " sops-v${v}.linux.amd64$" "sops-v${v}.checksums.txt" | sha256sum -c -
   install -m 0755 "sops-v${v}.linux.amd64" /usr/local/bin/sops
   mkdir -p ~/.config/sops/age && (umask 077 && age-keygen -o ~/.config/sops/age/keys.txt)
   ```

   Keep the `age1...` public key that it prints. The private key never leaves the VPS.

2. **On the laptop**, add that public key to the `secrets/prod\.enc\.env` rule in `.sops.yaml`, next to the laptop key, separated by a comma. If it replaces an old VPS, remove the old key. Then run `sops updatekeys secrets/prod.enc.env`.
3. **Check that the VPS can decrypt** before anything depends on it:

   ```bash
   scp secrets/prod.enc.env qafiyah:/tmp/prod.enc.env
   ssh qafiyah 'sops decrypt --input-type dotenv --output-type dotenv /tmp/prod.enc.env >/dev/null && echo ok; rm /tmp/prod.enc.env'
   ```

4. **Commit and push** `.sops.yaml` and `secrets/prod.enc.env`, and release them into `main`. Then run `bun run deploy`. From then on, every deploy generates `/opt/qafiyah/.env`.

## Losing or revoking a key

- **The laptop key is lost:** restore `~/.config/sops/age/keys.txt` from the password manager. Without that backup and without the VPS key, no committed secret can be recovered.
- **A machine is compromised:** remove its public key from `.sops.yaml`, run `sops updatekeys`, and **change the secrets themselves**. A removed recipient keeps what it already decrypted, and the old ciphertext stays in the git history.
