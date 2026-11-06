import { useState } from "react";
import { api } from "../../shell/api";
import type { Feature } from "../../shell/feature";
import { toMegabytes } from "../../shell/progress";
import type { CleanupSummary } from "../../shell/types";

function StorageCleanup() {
  const [summary, setSummary] = useState<CleanupSummary | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [working, setWorking] = useState(false);
  const [message, setMessage] = useState("");

  async function check() {
    setWorking(true);
    setMessage("");
    setConfirming(false);
    try {
      const found = await api.cleanupPreview();
      setSummary(found);
      if (found.items === 0) {
        setMessage("沒有可以清理的檔案。");
      }
    } catch (reason) {
      setMessage(String(reason));
    }
    setWorking(false);
  }

  async function clean() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    setWorking(true);
    setConfirming(false);
    try {
      const freed = await api.cleanupRun();
      setSummary(null);
      setMessage(`已清出 ${toMegabytes(freed)} MB。`);
    } catch (reason) {
      setMessage(String(reason));
    }
    setWorking(false);
  }

  const canClean = summary !== null && summary.items > 0;

  return (
    <section className="panel setting">
      <div className="setting-head">
        <span>清理空間</span>
        {canClean && <output>{toMegabytes(summary.bytes)} MB</output>}
      </div>
      {canClean ? (
        <button
          className={confirming ? "secondary danger" : "secondary"}
          disabled={working}
          onBlur={() => setConfirming(false)}
          onClick={clean}
        >
          {confirming ? "再按一次清理" : "清理"}
        </button>
      ) : (
        <button className="secondary" disabled={working} onClick={check}>
          {working ? "正在檢查" : "檢查可以清理的檔案"}
        </button>
      )}
      <p>
        {message || "刪除沒有實例在用的遊戲版本、資源和 Java。存檔和模組不受影響，需要時會重新下載。"}
      </p>
    </section>
  );
}

export const storageCleanup: Feature = {
  id: "storage-cleanup",
  name: "清理空間",
  description: "刪除沒有實例在用的共用檔案",
  optional: true,
  AdvancedSettings: StorageCleanup,
};
