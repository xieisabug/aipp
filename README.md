# AIPP - AI 助手平台

<div align="center">

[![AIPP Logo](https://xieisabug.github.io/aipp/app-icon.png)]

**基于 Tauri 2 构建的强大桌面 AI 助手平台**

_与多个大语言模型对话，执行代码，预览组件，提升生产力_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.7-24C8DB.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB.svg)](https://reactjs.org/)
[![Rust](https://img.shields.io/badge/Rust-Latest-orange.svg)](https://www.rust-lang.org/)

</div>

| 下列描述 60% 由 AI 生成

## 项目概述

AIPP 是一个跨平台桌面应用程序，最初构建这个项目的初衷是想要一个完美贴合我自己需求的“套壳”工具，我想要用 Claude 的 Artifacts 但是又喜欢用 Gemini 模型，我想要数据在本地或者我自己的服务器，我想要快速的用到最新的模型，我想要将 AI 帮我编写的小工具保存起来，我想要编写很多能利用 AI 的小工具……。有太多我自己独特的需求，那就只能自己动手了。

项目基于 Tauri 2 和 React 构建，在保持强大 AI 能力的同时提供原生桌面体验。AIPP 让你专注于 AI 工具的核心逻辑和 Prompt 设计，甚至未来可以让 AI 为你实现需要的工具，而无需担心底层的技术实现。

## 当前功能

AIPP 提供了完整的 AI 助手桌面应用体验，支持连接各种主流大语言模型提供商如 OpenAI、Anthropic、Google 等，通过统一的 API 接口实现无缝模型切换。应用具备丰富的对话管理和历史记录功能，支持文件附件和多模态输入，用户可以定制 AI 助手和专业化 Prompt，通过 Bang 命令快速执行各种操作。

特别值得一提的是，AIPP 不仅是聊天工具，更是强大的内容创作和预览平台。它能够实时预览 HTML、SVG 等网页内容，且支持 React 和 Vue 组件的预览，并可以直接运行 Python、Bash/Powershell、AppleScript 等脚本（如果系统有对应的环境的话）。

为了提升工作效率，AIPP 提供了全局快捷键（Ctrl+Shift+I/O）快速访问，系统托盘常驻运行不占用任务栏，多窗口系统让不同任务可以在独立窗口中进行。目前所有数据都使用 SQLite 本地存储，确保隐私安全。

## 架构设计

AIPP 采用现代化的技术栈，后端使用 Rust 配合 Tauri 2 框架确保性能和跨平台兼容性，前端使用 React 19 和 TypeScript 提供类型安全的开发体验。UI 层面采用 shadcn/ui 组件库和 Radix UI 原语，配合 Tailwind CSS 实现现代化的界面设计。数据存储使用 SQLite 配合 rusqlite，构建工具使用 Vite 确保快速的开发体验。

应用采用多窗口架构设计，Ask 窗口提供全局快捷键快速 AI 查询功能，ChatUI 窗口承载完整的对话界面，Config 窗口负责设置和模型配置，Artifacts 预览窗口专门用于内容渲染和组件预览，未来还将推出插件窗口支持可扩展的插件系统。

## 快速开始

使用 AIPP 需要 Node.js 18+、Rust 1.70+ 以及 Tauri 的平台特定依赖。克隆仓库后，通过 `npm install` 安装依赖，然后使用 `npm run tauri dev` 运行应用开始开发。

开发时推荐使用 `npm run build` 构建前端进行测试，使用 `cargo check --manifest-path src-tauri/Cargo.toml` 检查 Rust 后端代码。

## 未来规划

AIPP 将建设完整的插件生态系统，提供自定义 AI 工具的可扩展架构，且计划插件和 Artifacts 可以直接访问大语言模型提供商和本地数据库。官方插件市场将对所有插件进行安全审查，确保用户使用安全。

在 AI 能力方面，AIPP 将持续集成各大 AI 提供商的最新功能，如 Claude Projects、DALL-E 等，实现实时模型更新和能力同步，提供高级多模态支持。同时开发各种插件工具，例如根据表结构生成全栈代码的代码生成器、自动化测试评估的大模型评测工具、并排响应比较的多模型对比工具等。

## 设计理念

AIPP 坚持本地优先的理念，所有数据本地存储不上传云端，在 Apache 2.0 许可证下保持开源透明。支持 Windows、macOS 和 Linux 跨平台运行，不含任何遥测功能，确保用户对话保持私密。

AIPP 专注于 AI 能力的整合和扩展，通过插件系统让所有 AI 都能使用最新的工程能力，但不会提供跳脱于 AI 之外的功能。

## 许可证

本项目采用[APACHE 许可证](LICENSE)。

[![Star History Chart](https://api.star-history.com/svg?repos=xieisabug/tea&type=Date)](https://star-history.com/#xieisabug/tea&Date)

[image-banner]: https://xieisabug.github.io/tea/tea-logo-with-background.png
