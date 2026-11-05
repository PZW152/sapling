import { useEffect, useState } from "react";
import { api } from "../../shell/api";
import type { LoginCode, Profile } from "../../shell/types";

export function AccountPanel({
  profile,
  onProfileChange,
  disabled,
  offline,
}: {
  profile: Profile | null;
  onProfileChange: (profile: Profile | null) => void;
  disabled: boolean;
  /** 這次改用離線名稱啟動，不會用到這個帳號。 */
  offline: boolean;
}) {
  const [restoring, setRestoring] = useState(true);
  const [code, setCode] = useState<LoginCode | null>(null);
  const [starting, setStarting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    api.restoreAccount()
      .then(onProfileChange)
      .catch((reason) => setError(String(reason)))
      .finally(() => setRestoring(false));
  }, [onProfileChange]);

  async function signIn() {
    setError("");
    setStarting(true);
    try {
      const loginCode = await api.loginStart();
      setCode(loginCode);
      setCopied(await copyText(loginCode.userCode));
      setStarting(false);
      onProfileChange(await api.loginFinish());
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
      await api.signOut();
      onProfileChange(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  if (code) {
    return (
      <section className="panel account">
        <h2 className="label">在瀏覽器輸入代碼</h2>
        <button
          className="login-code"
          aria-label="複製登入代碼"
          onClick={() => copyText(code.userCode).then(setCopied)}
        >
          {code.userCode}
        </button>
        <p className="hint">{copied ? "代碼已複製，貼到登入頁面。" : "按上面的代碼可以複製。"}</p>
        <p className="hint">沒看到頁面的話，請前往 {code.verificationUri}</p>
        <p className="hint waiting">正在等待你完成登入</p>
      </section>
    );
  }

  if (profile) {
    return (
      <section className="panel account">
        <div className="me">
          <Avatar skinUrl={profile.skinUrl} />
          <div className="me-text">
            <strong>{profile.name}</strong>
            <small>{offline ? "正版帳號，這次不使用" : "正版帳號"}</small>
          </div>
          <button className="text-button" disabled={disabled} onClick={signOut}>
            登出
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  return (
    <section className="panel account">
      <h2 className="label">帳號</h2>
      <p className="hint">{restoring ? "正在恢復登入" : "需要正版微軟帳號才能進入遊戲。"}</p>
      <button className="secondary strong" disabled={restoring || starting} onClick={signIn}>
        {starting ? "正在取得代碼" : "用微軟帳號登入"}
      </button>
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
  if (!skinUrl) {
    return <div className="skin-head" aria-hidden="true" />;
  }
  return (
    <div className="skin-head" aria-hidden="true">
      <span className="skin-face" style={{ backgroundImage: `url(${skinUrl})` }} />
      <span className="skin-hat" style={{ backgroundImage: `url(${skinUrl})` }} />
    </div>
  );
}
