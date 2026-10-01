export function spawnGroup(cmd: readonly string[], cwd: string) {
  return Bun.spawn([...cmd], { cwd, stdout: 'pipe', stderr: 'pipe', detached: true });
}

export function killGroup(pid: number): void {
  try {
    process.kill(-pid, 'SIGTERM');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error;
  }
}
