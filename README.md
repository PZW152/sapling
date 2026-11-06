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
- 遊戲異常結束時可查看問題記錄
- 可調整遊戲記憶體、Java 參數和同時下載數

你需要擁有正版 Minecraft Java 版。離線模式只在這台電腦登入過擁有遊戲的微軟帳號後才能使用，不支援盜版。

## 安裝

到 [Releases](https://github.com/PZW152/sapling/releases) 下載最新版本。

下載 `portable.exe`，放進一個資料夾後直接打開，不需要安裝。遊戲和設定會存在同一個資料夾的 `Sapling Data` 裡。

目前只支援 Windows 11 x64。程式尚未數位簽章，Windows 可能會顯示安全警告。

Sapling 目前是早期版本，1.0.0 才是第一個基礎版本，詳見 [版本號規範](VERSIONING.md)、[更新日誌](CHANGELOG.md) 和 [設計與開發原則](DESIGN.md)。

## 從原始碼建置

需要 Rust（MSVC 工具鏈）、Node.js 和 Microsoft C++ Build Tools。

```bash
cd app
npm install
npm run tauri dev
```

## 授權

Copyright (c) 2026 PZW152。本專案代碼公開，但不是標準開源軟體。

歡迎學習和善意參考：少量參考請署名；修改版和大量參考必須遵守 [善意使用指南](USAGE.md)，包括說明是第三方作品、公開原始碼和不收費。不可以冒充官方、倒賣，或用於訓練 AI。

完整條款見 [LICENSE](LICENSE)，AI 工具請參閱 [AGENTS.md](AGENTS.md)。

## 聲明

非 Minecraft 官方產品，與 Mojang 和 Microsoft 無關。
