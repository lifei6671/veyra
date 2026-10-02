import { useState, type FormEvent } from "react";
import { LockClosedIcon } from "@heroicons/react/24/outline";
import { api, ApiError } from "../api/client";
import type { AuthStatus } from "../api/types";
import openBoxLogo from "../assets/openbox-logo.png";

export function AuthScreen({ status, onAuthenticated }: { status: AuthStatus; onAuthenticated: (status: AuthStatus) => void }) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");
  const setup = status.enabled && !status.passwordSet;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (setup && password !== confirm) {
      setError("两次输入的密码不一致");
      return;
    }
    setSubmitting(true);
    setError("");
    try {
      const next = setup ? await api.setupPassword(password) : await api.login(password);
      onAuthenticated(next);
    } catch (reason) {
      setError(reason instanceof ApiError && reason.code === "ACCESS_PASSWORD_INVALID" ? "密码不正确" : reason instanceof Error ? reason.message : "登录失败");
    } finally {
      setSubmitting(false);
    }
  };

  return <main className="auth-page">
    <form className="auth-card" onSubmit={submit}>
      <img src={openBoxLogo} alt="Open-Box" />
      <div className="auth-icon"><LockClosedIcon /></div>
      <h1>{setup ? "设置访问密码" : "连接 Open-Box"}</h1>
      <p>{setup ? "首次使用需要创建面板访问密码。" : "请输入后端面板密码，登录状态会保存在当前浏览器会话。"}</p>
      <label><span>密码</span><input autoFocus type="password" autoComplete={setup ? "new-password" : "current-password"} value={password} onChange={event => setPassword(event.target.value)} required minLength={setup ? 8 : 1} /></label>
      {setup && <label><span>确认密码</span><input type="password" autoComplete="new-password" value={confirm} onChange={event => setConfirm(event.target.value)} required minLength={8} /></label>}
      {error && <div className="auth-error" role="alert">{error}</div>}
      <button className="primary-button" type="submit" disabled={submitting || !password}>{submitting ? "正在连接…" : setup ? "保存并进入" : "登录"}</button>
      <small>后端：https://openbox.disign.me</small>
    </form>
  </main>;
}
