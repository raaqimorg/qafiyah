export function spawnGroup(cmd: readonly string[], cwd: string) {
  return Bun.spawn([...cmd], { cwd, stdout: 'pipe', stderr: 'pipe', detached: true });
}

export function killGroup(pid: number): void {
  try {
    process.kill(-pid, 'SIGTERM');
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code !== 'ESRCH' && code !== 'EPERM') throw error;
  }
}
