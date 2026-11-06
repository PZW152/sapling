import { useSyncExternalStore } from "react";

export type DownloadTask = {
  id: number;
  name: string;
  /** 空字串代表還沒結束；`done` 成功；其他文字是失敗原因。 */
  result: string;
};

const DONE = "done";

let tasks: DownloadTask[] = [];
let nextId = 1;
const listeners = new Set<() => void>();

function publish(next: DownloadTask[]) {
  tasks = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** 記下一項下載，並在它結束時更新結果。錯誤會原樣往外丟，呼叫方照常處理。 */
export async function trackDownload<T>(name: string, work: () => Promise<T>) {
  const id = nextId;
  nextId += 1;
  publish([{ id, name, result: "" }, ...tasks]);
  const finish = (result: string) =>
    publish(tasks.map((task) => (task.id === id ? { ...task, result } : task)));
  try {
    const value = await work();
    finish(DONE);
    return value;
  } catch (reason) {
    finish(String(reason));
    throw reason;
  }
}

export function isRunning(task: DownloadTask) {
  return task.result === "";
}

export function isDone(task: DownloadTask) {
  return task.result === DONE;
}

/** 清掉已結束的項目，進行中的留著。 */
export function clearFinished() {
  publish(tasks.filter(isRunning));
}

export function useDownloadTasks() {
  return useSyncExternalStore(subscribe, () => tasks);
}

export function hasRunningDownloads() {
  return tasks.some(isRunning);
}
