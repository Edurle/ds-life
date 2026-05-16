# 技能市场

## 概念

当前 `agent_plugins/skills/` 是静态目录。升级为可搜索、下载、加载的动态技能包系统。

## 工作流

```
Agent 遇到陌生领域
  → host.search_skills("kubernetes")    # 查询可用技能
  → host.install_skill("k8s-deploy")    # 下载到 skills/
  → skill_loader.load("k8s-deploy")     # 加载到上下文
  → Agent 获得新能力
```

## 技能格式

```toml
# SKILL.toml
name = "k8s-deploy"
version = "1.0.0"
author = "community"
description = "Kubernetes deployment helper"
prompt = "You are a K8s deployment expert..."
tools = ["bash", "read_file", "write_file"]
```

## 技能仓库

- 本地：`agent_plugins/skills/`（已有）
- Git：`https://github.com/user/agent-skill-k8s`
- Registry：`https://skills.deepseek.com/`（未来）

## Agent 自主安装

Agent 在 `main_loop.lua` 中遇到无法处理的任务时，可主动调用 `host.search_skills` 查找并安装相关技能。
