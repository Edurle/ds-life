# 自然语言 → 自动化流水线

## 概念

用自然语言描述业务流程，Agent 生成并注册为**定时 Lua 任务**，持续运行，失败则自我修复。

## 示例

```
用户: "每天凌晨 3 点拉取 GitHub 最新代码，跑测试，
       如果通过就部署到 staging，失败就通知我"

Agent:
  1. 理解意图
  2. 生成 pipeline.lua（cron + git pull + cargo test + deploy + notify）
  3. 加载到 system_core/tasks/ 作为定时任务
  4. 持续监控执行，失败时自我修复
```

## 技术实现

```lua
-- system_core/tasks/daily_deploy.lua (Agent 生成)
function cron_task()
    local ok = host.bash("git pull")
    if not ok then return "git pull failed" end

    ok = host.bash("cargo test")
    if not ok then
        host.bash("notify-send 'Tests failed'")
        return "tests failed"
    end

    host.bash("scp target/release/agent_host staging:/app/")
    return "deployed"
end

host.register_cron("daily_deploy", "0 3 * * *", cron_task)
```

## 影响

- NL → 自动化代码 → 运行 → 自我维护 → 形成闭环
- Agent 从"回答问题"升级为"管理业务流程"
- 工业场景：CI/CD 辅助、监控响应、数据流水线
