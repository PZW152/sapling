<p align="center"><img src="assets/logo.svg" width="96" alt="Sapling Launcher logo"></p>

<h1 align="center">Sapling Launcher</h1>

<p align="center">樹苗：一個簡單好用的 Minecraft Java 版啟動器（簡稱 SPL）</p>

<p align="center">
  <a href="https://github.com/PZW152/sapling/releases/latest"><img src="https://img.shields.io/github/v/release/PZW152/sapling?color=2F9E5B&label=%E6%9C%80%E6%96%B0%E7%89%88%E6%9C%AC" alt="最新版本"></a>
  <a href="https://github.com/PZW152/sapling/actions/workflows/ci.yml"><img src="https://github.com/PZW152/sapling/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center"><img src="assets/screenshot.png" width="760" alt="Sapling Launcher 首頁"></p>

## 功能

- 使用正版微軟帳號登入，重開後保持登入
- 自動下載遊戲檔案和所需的 Java，並用 SHA1 校驗
- 支援 1.19 及以後的原版正式版
- 實時顯示遊戲日誌
- 可調整遊戲記憶體、Java 參數和同時下載數

本啟動器不提供離線登入，你需要擁有正版 Minecraft Java 版。

## 安裝

到 [Releases](https://github.com/PZW152/sapling/releases) 下載最新的安裝檔。

目前只支援 Windows 11 x64。安裝檔尚未數位簽章，Windows 可能會顯示安全警告。

## 從原始碼建置

需要 Rust（MSVC 工具鏈）、Node.js 和 Microsoft C++ Build Tools。

```bash
cd app
npm install
npm run tauri dev
```

## 授權

Copyright (c) 2026 PZW152，保留一切權利。本專案不是開源軟體。

你可以下載官方版本供個人使用，也可以在 GitHub 上閱讀原始碼。未經書面許可，不可以複製、修改、重構、散佈、仿製本作品，或用於訓練 AI。完整條款見 [LICENSE](LICENSE)，AI 工具請參閱 [AGENTS.md](AGENTS.md)。

## 聲明

非 Minecraft 官方產品，與 Mojang 和 Microsoft 無關。
