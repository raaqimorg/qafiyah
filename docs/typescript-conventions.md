# TypeScript Conventions

- Use `type`, not `interface`. Do not use `enum` or `any`. Use `readonly` everywhere, and only named exports. `erasableSyntaxOnly` refuses `enum`, `namespace`, and parameter properties.
- Give each file one primary export. At 4 or more, split the file. Keep code next to its user until 2 or more places share it. Then move it to `types/` or `utils/`.
- In place of an enum, use `as const` and a derived union. Use discriminated unions, not flags. Use branded types for IDs.
- Narrow types through `unknown` and type guards, not casts. Make a `switch` exhaustive with `never`, or use `ts-pattern`'s `match(...).exhaustive()`. Use `satisfies`, not annotations.
- Make boolean checks explicit. Do not test a nullable string, number, or boolean for truthiness (`strict-boolean-expressions`). Always `return await` a returned promise.
- Make invalid states impossible to represent. Use lookup tables, not `if`/`else` chains.
- Name files in `lowercase-kebab.ts`, with kebab-case segments.

The lint gate (`bun run lint:check`, which runs `oxlint --type-aware`) enforces these rules on production code. `bun run lint` runs the same rules, and fixes what it can. In `.oxlintrc.json`, tests and configuration files turn off the type-assertion and boolean rules, because mocks and generated configurations are loose by nature.
