import { useState } from "react";
import { clearFinished, isDone, isRunning, useDownloadTasks } from "../../shell/downloads";

/** 下載頁右上角的清單：不用待在原來的分頁，也能看到每項下載的狀況。 */
export function DownloadQueue() {
  const tasks = useDownloadTasks();
  const [open, setOpen] = useState(false);

  if (tasks.length === 0) {
    return null;
  }

  const running = tasks.filter(isRunning).length;

  return (
    <div className="queue">
      <button className="secondary" aria-expanded={open} onClick={() => setOpen(!open)}>
        {running > 0 ? `下載中 ${running}` : "下載完成"}
      </button>
      {open && (
        <div className="queue-panel">
          <ul>
            {tasks.map((task) => (
              <li key={task.id}>
                <span>{task.name}</span>
                {isRunning(task) && <small>進行中</small>}
                {isDone(task) && <small>完成</small>}
                {!isRunning(task) && !isDone(task) && <p className="error">{task.result}</p>}
              </li>
            ))}
          </ul>
          {running < tasks.length && (
            <button className="text-button" onClick={clearFinished}>
              清除已結束的項目
            </button>
          )}
        </div>
      )}
    </div>
  );
}
