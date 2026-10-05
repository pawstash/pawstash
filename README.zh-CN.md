<p align="center">
  <img src="assets/pawstash.png" alt="Pawstash" width="340" />
</p>

<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

<p align="center">
  面向归档用户的跨平台客户端、下载器和收藏夹管理工具，支持通过服务器进行端到端加密的媒体库同步等功能。
</p>

<p align="center">
  <a href="https://t.me/pawstashapp"><img src="https://img.shields.io/badge/Telegram-Join%20Channel-229ED9?style=flat-square&logo=telegram&logoColor=white" alt="Telegram" /></a>
  <a href="https://discord.gg/ahcx8ub5Ck"><img src="https://img.shields.io/badge/Discord-Join%20Server-5865F2?style=flat-square&logo=discord&logoColor=white" alt="Discord" /></a>
  <a href="https://reddit.com/r/pawstash"><img src="https://img.shields.io/badge/Reddit-r%2Fpawstash-FF4500?style=flat-square&logo=reddit&logoColor=white" alt="Reddit" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg?style=flat-square" alt="许可证：GPL-3.0" /></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2" /></a>
  <a href="https://svelte.dev"><img src="https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white" alt="Svelte 5" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.80+-orange?style=flat-square&logo=rust&logoColor=white" alt="Rust" /></a>
</p>

<p align="center">
  <a href="#截图">截图</a> • <a href="#下载">下载</a> • <a href="#主要功能">主要功能</a> • <a href="#反馈与问题">反馈与问题</a> • <a href="#许可证">许可证</a>
</p>

---

## 截图

### 创作者页面
<img src="assets/creator.png" alt="创作者页面" width="100%" />

### 帖子查看器
<img src="assets/post.png" alt="帖子查看器" width="100%" />

### 设置页面
<img src="assets/settings.png" alt="设置页面" width="100%" />

---

## 主要功能

- **本地优先与离线使用**：收藏、自定义收藏夹、创作者订阅及已保存的媒体均可完全在设备本地使用。
- **零知识同步**：端到端加密的多设备同步（XChaCha20-Poly1305 + Argon2id）。
- **下载与缓存**：下载媒体附件，将浏览过的帖子缓存到本地，以供离线访问。
- **跨平台**：适用于 Windows、Android、macOS 和 Linux 的统一应用。

---

## 下载

[![最新版本](https://img.shields.io/github/v/release/pawstash/pawstash?include_prereleases&label=Latest%20Release&color=2ea44f&style=flat-square)](https://github.com/pawstash/pawstash/releases/latest)
[![预发布版](https://img.shields.io/badge/Pre--releases-All%20Beta%20Builds-f59e0b?style=flat-square)](https://github.com/pawstash/pawstash/releases)

| 平台 | 安装包 | 架构 | 直接下载 |
| :--- | :--- | :--- | :--- |
| **Windows** | 独立便携版 | x64 | [![下载 .exe](https://img.shields.io/badge/Download-Pawstash--portable.exe-2ea44f?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash-portable.exe) |
| **Windows** | 安装程序（`.exe`） | x64 | [![下载安装程序](https://img.shields.io/badge/Download-Pawstash--Setup.exe-0078d4?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash-Setup.exe) |
| **Android** | 已签名 APK（推荐） | ARM64（`arm64-v8a`） | [![下载 APK](https://img.shields.io/badge/Download-Pawstash--arm64.apk-3ddc84?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash-arm64.apk) |
| **Android** | 已签名 APK（旧版架构） | ARMv7（`armeabi-v7a`） | [![下载 APK](https://img.shields.io/badge/Download-Pawstash--v7a.apk-3ddc84?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash-v7a.apk) |
| **Android** | 通用 APK | 所有架构 | [![下载 APK](https://img.shields.io/badge/Download-Pawstash.apk-3ddc84?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash.apk) |
| **Linux** | AppImage（`.AppImage`） | x64 | [![下载 AppImage](https://img.shields.io/badge/Download-Pawstash.AppImage-fcc624?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash.AppImage) |
| **Linux** | Debian（`.deb`） | x64 | [![下载 DEB](https://img.shields.io/badge/Download-Pawstash.deb-e11d48?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash.deb) |
| **Linux** | 独立压缩包（`.tar.gz`） | x64 | [![下载 tar.gz](https://img.shields.io/badge/Download-Pawstash--linux--x64.tar.gz-e5a93c?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash-linux-x64.tar.gz) |
| **macOS** | 通用 DMG（`.dmg`） | Apple Silicon / Intel | [![下载 DMG](https://img.shields.io/badge/Download-Pawstash.dmg-555555?style=flat-square)](https://github.com/pawstash/pawstash/releases/latest/download/Pawstash.dmg) |

---

## 反馈与问题

发现了错误，或有改进 Pawstash 的想法？

[![报告错误](https://img.shields.io/badge/Bug-Report%20an%20Issue-d73a4a?style=flat-square&logo=github)](https://github.com/pawstash/pawstash/issues/new?template=bug_report.yml&labels=bug&title=%5BBug%5D%3A+)
[![功能建议](https://img.shields.io/badge/Feature-Suggest%20an%20Idea-0075ca?style=flat-square&logo=github)](https://github.com/pawstash/pawstash/issues/new?template=feature_request.yml&labels=enhancement&title=%5BFeature%5D%3A+)

---

## 许可证

GNU 通用公共许可证 v3.0（GPL-3.0）——详见 [LICENSE](LICENSE)。
