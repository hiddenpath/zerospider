# 第十八章：命令参考

本章提供 ZeroClaw 所有命令的快速参考。

---

## 全局命令

### 查看帮助

```bash
zeroclaw --help
zeroclaw [command] --help
```

### 查看版本

```bash
zeroclaw --version
```

### 查看状态

```bash
zeroclaw status
```

---

## 初始化命令

### onboard - 初始化配置

```bash
# 快速配置
zeroclaw onboard

# 交互式配置
zeroclaw onboard --interactive

# 指定参数
zeroclaw onboard --api-key sk-xxx --provider openrouter --model auto
```

---

## 对话命令

### agent - 与 AI 对话

```bash
# 交互模式
zeroclaw agent

# 单次对话
zeroclaw agent --message "你好"

# 指定模型
zeroclaw agent --provider openai --model gpt-4o

# 设置温度
zeroclaw agent --temperature 0.7

# 启用智能选择
zeroclaw agent --smart

# 启用多模型协商
zeroclaw agent --negotiate voting
```

---

## Provider 命令

### providers - 列出支持的 Provider

```bash
zeroclaw providers
```

### models - 管理模型

```bash
# 刷新模型列表
zeroclaw models refresh

# 刷新特定 Provider
zeroclaw models refresh --provider openai

# 强制刷新
zeroclaw models refresh --force
```

---

## 认证命令

### auth - 管理 API 认证

```bash
# 查看认证状态
zeroclaw auth status

# 列出认证配置
zeroclaw auth list

# 粘贴 Token
zeroclaw auth paste-token --provider anthropic

# OAuth 登录
zeroclaw auth login --provider openai-codex --device-code

# 切换配置
zeroclaw auth use --provider openai --profile work

# 删除认证
zeroclaw auth logout --provider openai
```

---

## 渠道命令

### channel - 管理通信渠道

```bash
# 列出渠道
zeroclaw channel list

# 启动渠道
zeroclaw channel start

# 健康检查
zeroclaw channel doctor

# 添加渠道
zeroclaw channel add telegram '{"token": "YOUR_TOKEN"}'

# 删除渠道
zeroclaw channel remove telegram

# 绑定 Telegram 用户
zeroclaw channel bind-telegram username_or_id
```

---

## 定时任务命令

### cron - 管理定时任务

```bash
# 列出任务
zeroclaw cron list

# 添加任务（cron 表达式）
zeroclaw cron add "0 9 * * *" "提醒内容"

# 添加任务（指定时间）
zeroclaw cron add-at "2024-12-31T23:59:00" "新年快乐"

# 添加任务（固定间隔）
zeroclaw cron add-every 3600000 "每小时提醒"  # 毫秒

# 添加任务（延迟执行）
zeroclaw cron once "30m" "30分钟后提醒"

# 删除任务
zeroclaw cron remove task_id

# 更新任务
zeroclaw cron update task_id --expression "0 10 * * *"

# 暂停/恢复
zeroclaw cron pause task_id
zeroclaw cron resume task_id

# 立即执行
zeroclaw cron run task_id

# 查看执行历史
zeroclaw cron runs
```

---

## 服务命令

### daemon - 启动守护进程

```bash
# 启动守护进程
zeroclaw daemon

# 指定端口
zeroclaw daemon --port 8080

# 指定主机
zeroclaw daemon --host 0.0.0.0
```

### gateway - 启动网关

```bash
zeroclaw gateway --port 8080
```

### service - 系统服务管理

```bash
# 安装服务
zeroclaw service install

# 启动服务
zeroclaw service start

# 停止服务
zeroclaw service stop

# 查看状态
zeroclaw service status

# 卸载服务
zeroclaw service uninstall
```

---

## 诊断命令

### doctor - 运行诊断

```bash
# 全面诊断
zeroclaw doctor

# 检查模型
zeroclaw doctor models

# 检查特定 Provider
zeroclaw doctor models --provider openai
```

---

## 技能命令

### skills - 管理技能

```bash
# 列出技能
zeroclaw skills list

# 安装技能
zeroclaw skills install https://github.com/user/skill

# 删除技能
zeroclaw skills remove skill_name
```

---

## 配置命令

### config - 管理配置

```bash
# 查看配置 Schema
zeroclaw config schema
```

---

## 集成命令

### integrations - 查看集成

```bash
# 查看集成详情
zeroclaw integrations info composio
```

---

## 硬件命令

### hardware - 硬件发现

```bash
# 发现 USB 设备
zeroclaw hardware discover

# 检查设备
zeroclaw hardware introspect --path /dev/ttyACM0

# 获取芯片信息
zeroclaw hardware info --chip STM32F401RETx
```

### peripheral - 外设管理

```bash
# 列出外设
zeroclaw peripheral list

# 添加外设
zeroclaw peripheral add nucleo-f401re /dev/ttyACM0

# 刷写固件
zeroclaw peripheral flash --port /dev/ttyACM0
```

---

## 迁移命令

### migrate - 数据迁移

```bash
# 从 OpenClaw 迁移
zeroclaw migrate openclaw

# 指定源目录
zeroclaw migrate openclaw --source /path/to/openclaw

# 预览（不写入）
zeroclaw migrate openclaw --dry-run
```

---

## 部署命令

### deploy - 远程部署

```bash
# 部署到服务器
zeroclaw deploy server_name

# 查看状态
zeroclaw deploy status server_name

# 健康检查
zeroclaw deploy health-check server_name

# 更新
zeroclaw deploy update server_name

# 回滚
zeroclaw deploy rollback server_name

# 查看日志
zeroclaw deploy logs server_name --follow

# 重启
zeroclaw deploy restart server_name
```

---

## 常用选项

| 选项 | 说明 |
|------|------|
| `--help, -h` | 显示帮助 |
| `--version, -V` | 显示版本 |
| `--verbose, -v` | 详细输出 |
| `--quiet, -q` | 静默模式 |
| `--config` | 指定配置文件 |
| `--log-level` | 日志级别 |

---

## 下一步

1. [配置参考](./19-config.md)
2. [常见问题](./20-faq.md)

---

[← 上一章：安全设置](./17-security.md) | [返回目录](./README.md) | [下一章：配置参考 →](./19-config.md)
