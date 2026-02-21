# ZeroSpider 🕷️

**协议驱动的自主 AI Agent 运行时，支持智能模型选择和多模型协商。**

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

[English](README.md) · [简体中文](README.zh-CN.md)

---

## 概述

ZeroSpider 是一个 Rust 优先的自主 AI Agent 运行时，集成了 [ai-protocol](https://github.com/hiddenpath/ai-protocol) 生态系统，实现智能的、协议驱动的 AI 操作。

### 核心特性

| 特性 | 描述 |
|------|------|
| **协议驱动 Provider** | 通过 YAML 文件配置 AI Provider，无需修改代码 |
| **智能模型选择** | 根据成本、速度、质量和可靠性自动选择最适合的模型 |
| **多模型协商** | 获取多个 AI 意见并综合出最佳响应 |
| **并行任务执行** | 同时执行独立任务，提高效率 |
| **远程部署** | 将 Agent 部署到远程服务器，支持受控访问 |
| **硬件集成** | 支持 GPIO、STM32 和其他外设 |

---

## 快速开始

### 前置条件

- Rust 1.70+（2021 版本）
- AI Protocol 目录（可选，用于协议驱动的 Provider）

### 构建

```bash
# 克隆仓库
git clone https://github.com/hiddenpath/zerospider.git
cd zerospider

# 启用所有功能构建
cargo build --features ai-protocol,smart-routing,multi-model,remote-deploy

# 或仅使用默认功能构建
cargo build
```

### 树莓派交叉编译 (aarch64)

```bash
# 安装目标平台
rustup target add aarch64-unknown-linux-gnu

# 构建发布版本
cargo build --release --target aarch64-unknown-linux-gnu

# 二进制文件位置
ls target/aarch64-unknown-linux-gnu/release/zerospider
```

### 运行

```bash
# 启用智能模型选择
cargo run --features smart-routing -- --smart

# 启用多模型协商
cargo run --features multi-model -- --negotiate

# 启用所有功能
cargo run --features ai-protocol,smart-routing,multi-model,remote-deploy
```

---

## 配置

### 环境变量

```bash
# AI Protocol 目录（用于协议驱动的 Provider）
export AI_PROTOCOL_DIR=/path/to/ai-protocol

# Provider API 密钥
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
```

### 配置文件

```yaml
# ~/.zerospider/config.yaml
routing:
  smart_selection: true
  
negotiation:
  enabled: true
  min_providers: 2
  consensus_threshold: 0.7

deployment:
  remote_enabled: true
  allowed_hosts:
    - server1.example.com
    - server2.example.com
```

---

## 文档

- [用户指南](docs/user-guide.zh-CN.md) - 完整的用户文档
- [集成指南](docs/ai-protocol-integration-guide.zh-CN.md) - 开发者集成指南
- [上游需求](docs/upstream-requirements.zh-CN.md) - 对 ai-lib-rust 和 ai-protocol 的需求

### 用户指南章节

1. [入门指南](docs/user-guide/01-getting-started.md)
2. [基本概念](docs/user-guide/02-basic-concepts.md)
3. [与 AI 聊天](docs/user-guide/03-chat-with-ai.md)
4. [Provider 配置](docs/user-guide/04-providers.md)
5. [智能路由](docs/user-guide/05-smart-routing.md)
6. [通道配置](docs/user-guide/06-channels.md)
7. [Telegram 集成](docs/user-guide/07-telegram.md)
8. [工具使用](docs/user-guide/10-tools.md)
9. [多模型协商](docs/user-guide/13-negotiation.md)
10. [自动化任务](docs/user-guide/14-automation.md)
11. [远程部署](docs/user-guide/15-deployment.md)
12. [安全设置](docs/user-guide/17-security.md)
13. [命令参考](docs/user-guide/18-commands.md)
14. [配置参考](docs/user-guide/19-config.md)
15. [常见问题](docs/user-guide/20-faq.md)

---

## 功能标志

| 标志 | 描述 |
|------|------|
| `ai-protocol` | 启用 ai-lib-rust 集成 |
| `smart-routing` | 启用 Provider 评分和自适应模型选择 |
| `multi-model` | 启用多模型协商和并行任务 |
| `remote-deploy` | 启用受控远程部署 |
| `hardware` | 启用硬件外设支持 |
| `channel-matrix` | 启用带 E2EE 的 Matrix 通道 |

---

## 架构

ZeroSpider 使用 trait 驱动的模块化架构：

- **Providers**：AI 模型后端（OpenAI、Anthropic、本地模型等）
- **Channels**：通信平台（Telegram、Discord、Matrix 等）
- **Tools**：可扩展的工具执行（shell、文件、浏览器等）
- **Memory**：持久化存储后端（SQLite、PostgreSQL 等）
- **Security**：策略执行和密钥管理

---

## 上游依赖

ZeroSpider 集成了：

- [ai-lib-rust](https://github.com/hiddenpath/ai-lib-rust) - 协议驱动的 AI API 客户端库
- [ai-protocol](https://github.com/hiddenpath/ai-protocol) - Provider 配置和协议定义

### 同步上游更新

ZeroSpider 追踪 [zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) 的更新：

```bash
# 列出上游变更
./sync-upstream.sh --list

# 预览合并
./sync-upstream.sh --dry-run

# 执行合并
./sync-upstream.sh

# 选择性合并特定提交
./sync-upstream.sh --cherry-pick <commit-hash>
```

---

## 许可证

可选择以下任一许可证：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

---

## 作者

**Luqiang Wang**

---

## 致谢

ZeroSpider 是 [ZeroClaw](https://github.com/ZeroClaw-Labs/zeroclaw) 的分支，增加了以下功能：
- ai-protocol 集成
- Provider 评分和智能路由
- 多模型协商
- 并行任务执行
- 远程部署
