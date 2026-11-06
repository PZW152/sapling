import { useEffect, useState } from "react";
import { api } from "./api";
import { useApp } from "./AppContext";

function toDataUrl(bytes: number[]) {
  const text = bytes.reduce((all, byte) => all + String.fromCharCode(byte), "");
  // 類型寫 png 就好，瀏覽器看內容判斷實際格式。
  return `data:image/png;base64,${btoa(text)}`;
}

/** 實例的圖示。沒有設定時用名稱的第一個字。 */
export function InstanceIcon({ id, name }: { id: string; name: string }) {
  const { contentVersion } = useApp();
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    api.instanceIcon(id)
      .then((bytes) => {
        if (current) {
          setUrl(bytes ? toDataUrl(bytes) : null);
        }
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [id, contentVersion]);

  return url ? (
    <img className="instance-icon" src={url} alt="" />
  ) : (
    <b className="instance-icon" aria-hidden="true">
      {[...name][0]}
    </b>
  );
}
