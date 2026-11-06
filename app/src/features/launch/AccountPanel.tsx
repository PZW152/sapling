import { useEffect, useRef, useState } from "react";
import { api } from "../../shell/api";
import type { LoginCode, Profile } from "../../shell/types";
import { open } from "@tauri-apps/plugin-dialog";
import { ErrorText } from "../../shell/ErrorText";
import { useEscape } from "../../shell/keyboard";

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
  const [accounts, setAccounts] = useState<Profile[]>([]);
  const [choosing, setChoosing] = useState(false);
  const [slim, setSlim] = useState(false);
  const [changingSkin, setChangingSkin] = useState(false);
  const attempt = useRef(0);
  const panel = useRef<HTMLElement>(null);

  useEscape(choosing ? () => setChoosing(false) : undefined, panel);

  // 登入、登出或切換後，其他帳號的清單都會變。
  useEffect(() => {
    api.listAccounts().then(setAccounts).catch(() => setAccounts([]));
  }, [profile]);

  useEffect(() => {
    api.restoreAccount()
      .then(onProfileChange)
      .catch((reason) => setError(String(reason)))
      .finally(() => setRestoring(false));
  }, [onProfileChange]);

  async function signIn() {
    const mine = ++attempt.current;
    setError("");
    setChoosing(false);
    setStarting(true);
    try {
      const loginCode = await api.loginStart();
      setCode(loginCode);
      setCopied(await copyText(loginCode.userCode));
      setStarting(false);
      const signedIn = await api.loginFinish();
      if (signedIn) {
        onProfileChange(signedIn);
      }
    } catch (reason) {
      if (mine === attempt.current) {
        setError(String(reason));
      }
    }
    // 取消後又重新登入時，舊的等待結束不能蓋掉新的代碼。
    if (mine === attempt.current) {
      setStarting(false);
      setCode(null);
    }
  }

  async function changeSkin() {
    const path = await open({
      title: "選擇皮膚圖片",
      filters: [{ name: "PNG 圖片", extensions: ["png"] }],
    });
    if (!path) {
      return;
    }
    setError("");
    setChangingSkin(true);
    try {
      onProfileChange(await api.changeSkin(path, slim));
    } catch (reason) {
      setError(String(reason));
    }
    setChangingSkin(false);
  }

  function cancelSignIn() {
    attempt.current += 1;
    setStarting(false);
    setCode(null);
    api.loginCancel().catch((reason) => setError(String(reason)));
  }

  async function switchTo(id: string) {
    setError("");
    setChoosing(false);
    try {
      onProfileChange(await api.switchAccount(id));
    } catch (reason) {
      setError(String(reason));
    }
  }

  const others = accounts.filter((account) => account.id !== profile?.id);
  const accountList = (
    <ul className="account-list">
      {others.map((account) => (
        <li key={account.id}>
          <button className="row" disabled={disabled} onClick={() => switchTo(account.id)}>
            <span>{account.name}</span>
          </button>
        </li>
      ))}
    </ul>
  );

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
        <p className="hint waiting" role="status">
          正在等待你完成登入
        </p>
        <div className="account-actions">
          <button className="text-button" onClick={() => api.openLink(code.verificationUri)}>
            打開登入頁面
          </button>
          <button className="text-button" onClick={cancelSignIn}>
            取消
          </button>
        </div>
      </section>
    );
  }

  if (profile) {
    return (
      <section className="panel account" ref={panel}>
        <div className="me">
          <Avatar skinUrl={profile.skinUrl} />
          <div className="me-text">
            <strong>{profile.name}</strong>
            <small>{offline ? "這次不使用" : "正版帳號"}</small>
          </div>
          <button
            className="text-button"
            disabled={disabled}
            aria-expanded={choosing}
            onClick={() => setChoosing(!choosing)}
          >
            帳號
          </button>
        </div>
        {choosing && (
          <>
            {accountList}
            <div className="skin-row">
              <div className="chips" role="group" aria-label="皮膚的手臂粗細">
                <button className="chip" aria-pressed={!slim} onClick={() => setSlim(false)}>
                  粗手臂
                </button>
                <button className="chip" aria-pressed={slim} onClick={() => setSlim(true)}>
                  細手臂
                </button>
              </div>
              <button
                className="text-button"
                disabled={disabled || changingSkin}
                onClick={changeSkin}
              >
                {changingSkin ? "正在上傳" : "更換皮膚"}
              </button>
            </div>
            <div className="account-actions">
              <button className="text-button" disabled={disabled || starting} onClick={signIn}>
                新增帳號
              </button>
              <button className="text-button" disabled={disabled} onClick={signOut}>
                登出
              </button>
            </div>
          </>
        )}
        <ErrorText message={error} />
      </section>
    );
  }

  return (
    <section className="panel account">
      <h2 className="label">帳號</h2>
      <p className="hint">{restoring ? "正在恢復登入" : "需要正版微軟帳號才能進入遊戲。"}</p>
      {!restoring && others.length > 0 && accountList}
      <button className="secondary strong" disabled={restoring || starting} onClick={signIn}>
        {starting ? "正在取得代碼" : "用微軟帳號登入"}
      </button>
      <ErrorText message={error} />
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
