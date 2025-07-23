# Gemini 项目上下文

本文全面介绍 "AIPP" AI助手项目技术相关和开发需要遵循的内容。

## 1. 项目概述

"AIPP" 是一个面向AI工具的桌面客户端/平台

**核心功能 (当前与规划中):**

*   **多模型支持:** 通过标准的API方式，支持接入各种主流的大语言模型供应商。
*   **功能丰富的聊天界面:** 提供对话、历史记录、文件管理、助手管理等标准聊天客户端功能。
*   **Bang命令:** 在输入框中通过 `!` 符号快速执行命令或提供附加上下文。
*   **内容预览:** 在应用内直接渲染和预览HTML、SVG，甚至是由React等框架编写的前端组件。
*   **脚本执行:** 在配置好的环境中（如Python, Bash）直接运行由AI生成的代码脚本。
*   **插件化架构 (未来):** 通过一个安全的官方插件市场来扩展新功能。
*   **数据本地化:** 所有用户数据均存储在本地，没有将数据上传到云端的计划。
*   **专注桌面端:** 应用为桌面环境（Windows, macOS, Linux）构建，没有推出Web版本的计划。

## 2. 技术栈

本项目是一个Tauri 2.0应用，它整合了Rust作为后端和React作为前端。

*   **后端:**
    *   **框架:** [Tauri](https://tauri.app/)
    *   **语言:** Rust
*   **前端:**
    *   **框架:** [React](https://react.dev/) (使用 Vite)
    *   **语言:** TypeScript
    *   **UI 组件库:** [shadcn/ui](https://ui.shadcn.com/), Radix UI
    *   **样式方案:** [Tailwind CSS](https://tailwindcss.com/)
*   **包管理器:** `pnpm` 用于管理前端依赖。

## 3. 项目结构

仓库主要分为两个核心部分：前端源代码 (`src`) 和 Tauri 后端源代码 (`src-tauri`)。

*   `src/`: 包含React前端应用的代码。
    *   `src/components/`: 可复用的React组件，包括基础UI元素 (`ui/`)、对话框和特定功能的组件。
    *   `src/hooks/`: 用于管理应用状态和逻辑的自定义React钩子。
    *   `src/data/`: 定义核心概念（如助手、对话）的数据模型和类型。
    *   `src/lib/`: 通用工具函数库。
    *   `src/styles/`: 自定义CSS样式文件。
    *   `src/main.tsx`: React应用的入口文件。
*   `src-tauri/`: 包含Rust后端代码。
    *   `src-tauri/src/main.rs`: Rust应用的入口文件。
    *   `src-tauri/src/api/`: 前端与rust交互的接口暴漏在这个文件夹的文件中。
    *   `src-tauri/src/db/`: 数据库相关的逻辑。
    *   `src-tauri/tauri.conf.json`: Tauri应用的核心配置文件，定义了应用标识、版本、窗口设置和构建命令等。
    *   `src-tauri/Cargo.toml`: Rust项目的包管理配置文件。
*   `dist/`: 用于存放前端构建后生成的静态资源。
*   `package.json`: 定义了前端项目的元数据、依赖和可执行脚本。
*   `vite.config.ts`: 前端构建工具Vite的配置文件。

## 4. 构建和运行命令

以下是在用于应用的开发和构建中，可以进行程序是否编译正确的命令，前端与后端文件有更改时，需要使用下列命令来进行验证修改是否成功。

*   **构建前端:**
    *   `npm run build`
    *   此命令会首先进行TypeScript的类型检查 (`tsc`)，然后使用Vite将前端代码打包成生产环境的静态资源。
    *   **注意：** 这个前端项目不用启动 `npm run dev` 来试图获取调试信息，这里的调试信息都在浏览器你看不到

*   **构建Rust端:**
    *   `cargo check`
    *   此命令要进入 src-tauri 文件夹执行，如果是 mac 直接执行 `cd src-tauri && cargo check`，如果是 Windows 执行 `cd src-tarui; cargo check`
    *   **注意:** 不要 cd src-tauri 来执行 cargo check, 直接执行 `cargo check --manifest-path src-tauri/Cargo.toml`

## 5. 编码注意事项

- 优化资源加载，前端优先展示界面后异步进行数据加载
- 多使用缓存，遵循react的最佳实践，减少页面的重绘次数
- 在 Rust 代码中注意内存使用和异步操作，不要进行阻塞操作，复杂的逻辑放在 Rust 中处理，减少 JS 代码的复杂度
- 编码时考虑在 Windows、macOS 和 Linux 上的表现
- 使用平台特定 API 时，提供跨平台的替代方案

## 6. 提交规范

使用语义化提交信息：

- feat: 新功能
- fix: 修复 bug
- docs: 文档更新
- style: 代码风格更改（不影响代码运行的变动）
- refactor: 重构（既不是新增功能，也不是修复 bug 的代码变动）
- perf: 性能优化
- test: 增加测试
- chore: 构建过程或辅助工具的变动