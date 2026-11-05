type Pending = { timer: ReturnType<typeof setTimeout>; save: () => Promise<unknown> };

/**
 * 延遲保存：同一個 `key` 只保存最後一次，不同 `key` 互不影響。
 * `onResult` 收到錯誤訊息，保存成功時收到空字串。
 */
export function createSaveQueue(delayMs: number, onResult: (error: string) => void) {
  const pending = new Map<string, Pending>();

  async function run(key: string) {
    const item = pending.get(key);
    if (!item) {
      return;
    }
    clearTimeout(item.timer);
    pending.delete(key);
    try {
      await item.save();
      onResult("");
    } catch (reason) {
      onResult(String(reason));
    }
  }

  return {
    schedule(key: string, save: () => Promise<unknown>) {
      clearTimeout(pending.get(key)?.timer);
      pending.set(key, { save, timer: setTimeout(() => run(key), delayMs) });
    },
    /** 立刻保存所有等待中的改動，例如關閉窗口前。 */
    async flush() {
      await Promise.all([...pending.keys()].map(run));
    },
  };
}
