import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LoginCode, Profile } from "./types";

export function AccountPanel({
  profile,
  onProfileChange,
  disabled,
}: {
  profile: Profile | null;
  onProfileChange: (profile: Profile | null) => void;
  disabled: boolean;
}) {
  const [restoring, setRestoring] = useState(true);
  const [code, setCode] = useState<LoginCode | null>(null);
  const [starting, setStarting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Profile | null>("restore_account")
      .then(onProfileChange)
      .catch((reason) => setError(String(reason)))
      .finally(() => setRestoring(false));
  }, [onProfileChange]);

  async function signIn() {
    setError("");
    setStarting(true);
    try {
      const loginCode = await invoke<LoginCode>("login_start");
      setCode(loginCode);
      setCopied(await copyText(loginCode.userCode));
      setStarting(false);
      onProfileChange(await invoke<Profile>("login_finish"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setStarting(false);
      setCode(null);
    }
  }

  async function signOut() {
    setError("");
    try {
      await invoke("sign_out");
      onProfileChange(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  if (code) {
    return (
      <section className="panel account">
        <h2>在瀏覽器輸入代碼</h2>
        <button
          className="login-code"
          aria-label="複製登入代碼"
          onClick={() => copyText(code.userCode).then(setCopied)}
        >
          {code.userCode}
        </button>
        <p>{copied ? "代碼已複製，貼到瀏覽器的登入頁面。" : "按上面的代碼可以複製。"}</p>
        <p>沒看到登入頁面的話，請前往 {code.verificationUri}</p>
        <p className="waiting">正在等待你完成登入</p>
      </section>
    );
  }

  return (
    <section className="panel account">
      <div className="account-identity">
        <Avatar skinUrl={profile?.skinUrl ?? null} />
        <div>
          <h2>{profile?.name ?? (restoring ? "正在恢復登入" : "尚未登入")}</h2>
          <p>{profile ? "正版帳號" : "需要正版微軟帳號"}</p>
        </div>
      </div>
      {profile ? (
        <button className="secondary" disabled={disabled} onClick={signOut}>
          登出
        </button>
      ) : (
        <button className="secondary" disabled={restoring || starting} onClick={signIn}>
          {starting ? "正在取得代碼" : "用微軟帳號登入"}
        </button>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

function Avatar({ skinUrl }: { skinUrl: string | null }) {
  if (skinUrl) {
    return (
      <div className="skin-head" aria-hidden="true">
        <span className="skin-face" style={{ backgroundImage: `url(${skinUrl})` }} />
        <span className="skin-hat" style={{ backgroundImage: `url(${skinUrl})` }} />
      </div>
    );
  }
  return (
    <svg className="avatar" viewBox="0 0 56 56" aria-hidden="true">
      <circle cx="28" cy="22" r="10" />
      <path d="M10 56c0-13 8-20 18-20s18 7 18 20z" />
    </svg>
  );
}
