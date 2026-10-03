# TypeScript Conventions

- `type` > `interface`; no `enum`/`any`; `readonly` everywhere; named exports only. `erasableSyntaxOnly` bans `enum`, `namespace`, and parameter properties.
- One primary export per file (4+ → split). Co-locate until shared 2+, then `types/`|`utils/`.
- Enums → `as const` + derived union. Discriminated unions > flags. Branded types for IDs.
- Narrow via `unknown` + guards, no casts. Exhaustive `switch` + `never`, or `ts-pattern`'s `match(...).exhaustive()`. `satisfies` > annotations.
- Boolean checks are explicit: no truthiness on a nullable string, number, or boolean (`strict-boolean-expressions`). Returned promises are always `return await`'d.
- Invalid states unrepresentable. Lookup tables > `if/else`.
- `lowercase-kebab.ts` filenames, kebab-case segments.

The lint gate (`bun run lint`, `oxlint --type-aware`) enforces this on production code. Tests and config/scripts relax the type-assertion and boolean rules in `.oxlintrc.json`, because mocks and generated configs are inherently loose.
