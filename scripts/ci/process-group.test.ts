import { describe, expect, test } from 'bun:test';

import { killGroup, spawnGroup } from './process-group';

const alive = (pid: number): boolean => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

async function until(condition: () => boolean): Promise<boolean> {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    if (condition()) return true;
    await Bun.sleep(20);
  }
  return condition();
}

describe('killGroup', () => {
  test('stops the children the task started, not only the task itself', async () => {
    const task = spawnGroup(['sh', '-c', 'sleep 300 & echo $!; wait'], import.meta.dir);
    const reader = task.stdout.getReader();
    const { value } = await reader.read();
    const child = Number(new TextDecoder().decode(value).trim());
    expect(alive(child)).toBe(true);
    killGroup(task.pid);
    await task.exited;
    expect(await until(() => !alive(child))).toBe(true);
  });

  test('leaves a process outside the group running', async () => {
    const outsider = Bun.spawn(['sleep', '300']);
    const task = spawnGroup(['sleep', '300'], import.meta.dir);
    killGroup(task.pid);
    await task.exited;
    expect(alive(outsider.pid)).toBe(true);
    outsider.kill();
    await outsider.exited;
  });

  test('does nothing once the whole group has exited', async () => {
    const task = spawnGroup(['true'], import.meta.dir);
    await task.exited;
    expect(() => killGroup(task.pid)).not.toThrow();
  });
});
