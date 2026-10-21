import { parseEnv } from 'node:util';

export async function ensureEnvFileFrom(
  targetPath: string,
  sourcePath: string
): Promise<'created' | 'skipped'> {
  if (await Bun.file(targetPath).exists()) return 'skipped';
  if (!(await Bun.file(sourcePath).exists())) return 'skipped';
  await Bun.write(targetPath, Bun.file(sourcePath));
  return 'created';
}

export async function loadEnvFile(
  path: string,
  env: Record<string, string | undefined>
): Promise<void> {
  const file = Bun.file(path);
  if (!(await file.exists())) return;
  for (const [key, value] of Object.entries(parseEnv(await file.text()))) {
    env[key] ??= value;
  }
}
