import { isAtLeast, parseVersion, type Version } from './doctor-version';

type Group = 'run' | 'commit' | 'optional';
type Status = 'ok' | 'warn' | 'fail' | 'skip';

export type CheckOutcome = {
  readonly group: Group;
  readonly name: string;
  readonly status: Status;
  readonly detail: string;
  readonly fix?: string;
};

export type DoctorEnv = {
  readonly platform: NodeJS.Platform;
  readonly bunVersion: string;
  readonly bunMinimum: string | undefined;
  readonly rustChannel: string | undefined;
  readonly hasNodeModules: boolean;
  readonly which: (name: string) => boolean;
  readonly probe: (
    command: readonly string[]
  ) => Promise<{ readonly code: number; readonly output: string } | undefined>;
};

type Fix = { readonly darwin: string; readonly linux: string };
type Requirement = { readonly version: Version; readonly label: string };

const GIB = 1024 ** 3;
const BASH_WITH_MAPFILE: Requirement = { version: [4, 0, 0], label: '4' };
const COMPOSE_WITH_OVERRIDE_TAG: Requirement = { version: [2, 24, 4], label: '2.24.4' };
const DOCKER_MEMORY_FOR_ELASTICSEARCH_BYTES = 3.5 * GIB;

const FIX = {
  git: { darwin: 'xcode-select --install', linux: 'sudo apt install git' },
  bash: { darwin: 'brew install bash, then open a new terminal', linux: 'sudo apt install bash' },
  dockerInstall: {
    darwin: 'install OrbStack (https://orbstack.dev) or Docker Desktop',
    linux: 'install Docker Engine (https://docs.docker.com/engine/install/)',
  },
  dockerStart: { darwin: 'start OrbStack or Docker Desktop', linux: 'sudo systemctl start docker' },
  compose: {
    darwin: 'update OrbStack or Docker Desktop',
    linux: 'sudo apt install docker-compose-plugin',
  },
  dockerMemory: {
    darwin: 'give Docker at least 4 GB of memory in the OrbStack or Docker Desktop settings',
    linux: 'give the machine at least 4 GB of memory',
  },
  rustup: {
    darwin: 'install rustup from https://rustup.rs',
    linux: 'install rustup from https://rustup.rs',
  },
  shellcheck: { darwin: 'brew install shellcheck', linux: 'sudo apt install shellcheck' },
  actionlint: {
    darwin: 'brew install actionlint',
    linux: 'go install github.com/rhysd/actionlint/cmd/actionlint@latest',
  },
  hadolint: {
    darwin: 'brew install hadolint',
    linux: 'download it from https://github.com/hadolint/hadolint/releases',
  },
  gh: { darwin: 'brew install gh', linux: 'see https://github.com/cli/cli#installation' },
  secrets: { darwin: 'brew install sops age', linux: 'see docs/deployment/secrets.md' },
} as const satisfies Record<string, Fix>;

function outcome(
  group: Group,
  name: string,
  status: Status,
  detail: string,
  fix?: string
): CheckOutcome {
  return fix === undefined ? { group, name, status, detail } : { group, name, status, detail, fix };
}

const fixFor = (env: DoctorEnv, fix: Fix): string =>
  env.platform === 'darwin' ? fix.darwin : fix.linux;

const shownVersion = (output: string): string => parseVersion(output)?.join('.') ?? 'installed';

type ToolSpec = {
  readonly group: Group;
  readonly name: string;
  readonly command: readonly string[];
  readonly fix: Fix;
  readonly minimum?: Requirement;
};

async function checkTool(env: DoctorEnv, spec: ToolSpec): Promise<CheckOutcome> {
  const result = await env.probe(spec.command);
  if (result?.code !== 0) {
    return outcome(spec.group, spec.name, 'fail', 'not installed', fixFor(env, spec.fix));
  }
  const found = parseVersion(result.output);
  const shown = found?.join('.') ?? 'installed';
  if (spec.minimum === undefined) return outcome(spec.group, spec.name, 'ok', shown);
  const meets = found !== undefined && isAtLeast(found, spec.minimum.version);
  const detail = `${shown} (needs ${spec.minimum.label})`;
  return meets
    ? outcome(spec.group, spec.name, 'ok', detail)
    : outcome(spec.group, spec.name, 'fail', detail, fixFor(env, spec.fix));
}

function checkBun(env: DoctorEnv): CheckOutcome {
  const label = env.bunMinimum?.replace(/^\D*/, '');
  const minimum = label === undefined ? undefined : parseVersion(label);
  if (minimum === undefined) return outcome('run', 'Bun', 'ok', env.bunVersion);
  const found = parseVersion(env.bunVersion);
  const detail = `${env.bunVersion} (needs ${label})`;
  return found !== undefined && isAtLeast(found, minimum)
    ? outcome('run', 'Bun', 'ok', detail)
    : outcome('run', 'Bun', 'fail', detail, 'bun upgrade');
}

function checkDockerMemory(env: DoctorEnv, output: string): CheckOutcome {
  const bytes = Number(output.trim());
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return outcome('run', 'Docker memory', 'skip', 'unknown');
  }
  const detail = `${(bytes / GIB).toFixed(1)} GB (needs 4 GB)`;
  return bytes >= DOCKER_MEMORY_FOR_ELASTICSEARCH_BYTES
    ? outcome('run', 'Docker memory', 'ok', detail)
    : outcome('run', 'Docker memory', 'fail', detail, fixFor(env, FIX.dockerMemory));
}

async function checkDocker(env: DoctorEnv): Promise<readonly CheckOutcome[]> {
  const installed = await env.probe(['docker', '--version']);
  if (installed?.code !== 0) {
    return [
      outcome('run', 'Docker', 'fail', 'not installed', fixFor(env, FIX.dockerInstall)),
      outcome('run', 'Docker Compose', 'skip', 'needs Docker'),
      outcome('run', 'Docker memory', 'skip', 'needs Docker'),
    ];
  }
  const [info, compose] = await Promise.all([
    env.probe(['docker', 'info', '--format', '{{.MemTotal}}']),
    checkTool(env, {
      group: 'run',
      name: 'Docker Compose',
      command: ['docker', 'compose', 'version', '--short'],
      fix: FIX.compose,
      minimum: COMPOSE_WITH_OVERRIDE_TAG,
    }),
  ]);
  if (info?.code !== 0) {
    return [
      outcome('run', 'Docker', 'fail', 'installed, not running', fixFor(env, FIX.dockerStart)),
      compose,
      outcome('run', 'Docker memory', 'skip', 'Docker is not running'),
    ];
  }
  return [
    outcome('run', 'Docker', 'ok', shownVersion(installed.output)),
    compose,
    checkDockerMemory(env, info.output),
  ];
}

async function checkToolchain(env: DoctorEnv): Promise<CheckOutcome> {
  const channel = env.rustChannel;
  if (channel === undefined) return outcome('run', 'Rust toolchain', 'skip', 'no pinned channel');
  const list = await env.probe(['rustup', 'toolchain', 'list']);
  const installed =
    list?.output.split('\n').some((line) => line.startsWith(`${channel}-`) || line === channel) ===
    true;
  return installed
    ? outcome('run', 'Rust toolchain', 'ok', channel)
    : outcome(
        'run',
        'Rust toolchain',
        'warn',
        `${channel} not installed yet`,
        'the first cargo build downloads it'
      );
}

async function checkRust(env: DoctorEnv): Promise<readonly CheckOutcome[]> {
  const rustup = await env.probe(['rustup', '--version']);
  if (rustup?.code !== 0) {
    const detail = env.which('cargo')
      ? 'not installed, and cargo alone ignores rust-toolchain.toml'
      : 'not installed';
    return [
      outcome('run', 'rustup', 'fail', detail, fixFor(env, FIX.rustup)),
      outcome('run', 'Rust toolchain', 'skip', 'needs rustup'),
    ];
  }
  return [outcome('run', 'rustup', 'ok', shownVersion(rustup.output)), await checkToolchain(env)];
}

async function runGroup(env: DoctorEnv): Promise<readonly CheckOutcome[]> {
  const [git, bash, docker, rust] = await Promise.all([
    checkTool(env, { group: 'run', name: 'Git', command: ['git', '--version'], fix: FIX.git }),
    checkTool(env, {
      group: 'run',
      name: 'Bash',
      command: ['bash', '--version'],
      fix: FIX.bash,
      minimum: BASH_WITH_MAPFILE,
    }),
    checkDocker(env),
    checkRust(env),
  ]);
  const install = env.hasNodeModules
    ? outcome('run', 'bun install', 'ok', 'done')
    : outcome('run', 'bun install', 'fail', 'not run yet', 'bun install');
  return [checkBun(env), git, bash, ...docker, ...rust, install];
}

async function commitGroup(env: DoctorEnv): Promise<readonly CheckOutcome[]> {
  return await Promise.all([
    checkTool(env, {
      group: 'commit',
      name: 'ShellCheck',
      command: ['shellcheck', '--version'],
      fix: FIX.shellcheck,
    }),
    checkTool(env, {
      group: 'commit',
      name: 'actionlint',
      command: ['actionlint', '--version'],
      fix: FIX.actionlint,
    }),
    checkTool(env, {
      group: 'commit',
      name: 'hadolint',
      command: ['hadolint', '--version'],
      fix: FIX.hadolint,
    }),
  ]);
}

async function checkGitHubCli(env: DoctorEnv): Promise<CheckOutcome> {
  const version = await env.probe(['gh', '--version']);
  if (version?.code !== 0) {
    return outcome('optional', 'GitHub CLI', 'skip', 'not installed', fixFor(env, FIX.gh));
  }
  const shown = shownVersion(version.output);
  const auth = await env.probe(['gh', 'auth', 'status']);
  return auth?.code === 0
    ? outcome('optional', 'GitHub CLI', 'ok', shown)
    : outcome('optional', 'GitHub CLI', 'warn', `${shown}, not logged in`, 'gh auth login');
}

function checkMaintainerTool(env: DoctorEnv, name: string): CheckOutcome {
  return env.which(name)
    ? outcome('optional', name, 'ok', 'installed')
    : outcome(
        'optional',
        name,
        'skip',
        'not installed (maintainers only)',
        fixFor(env, FIX.secrets)
      );
}

async function optionalGroup(env: DoctorEnv): Promise<readonly CheckOutcome[]> {
  return [
    await checkGitHubCli(env),
    checkMaintainerTool(env, 'sops'),
    checkMaintainerTool(env, 'age'),
  ];
}

const GROUPS: Record<Group, (env: DoctorEnv) => Promise<readonly CheckOutcome[]>> = {
  run: runGroup,
  commit: commitGroup,
  optional: optionalGroup,
};

export async function runChecks(
  env: DoctorEnv,
  groups: readonly Group[]
): Promise<readonly CheckOutcome[]> {
  const results = await Promise.all(groups.map((group) => GROUPS[group](env)));
  return results.flat();
}
