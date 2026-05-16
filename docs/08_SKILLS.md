# 第8步：技能系统

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_host/src/tools/fs.rs` 新增 `list_skills()` | 扫描 skills 目录 |
| `agent_plugins/system_core/skill_loader.lua` | 技能加载/列表 |
| `agent_plugins/skills/example/SKILL.md` | 示例技能 |

## 技能目录约定

```
agent_plugins/skills/
├── <skill_name>/
│   └── SKILL.md        ← 必须有此文件才被识别为技能
```

## list_skills 扫描逻辑

```rust
fn list_skills() {
    遍历 skills/ 子目录
    若某子目录下有 SKILL.md → 加入列表
}
```

## skill_loader.lua 功能

- `load_skill(name)` 读取技能 SKILL.md 内容
- `list_skills()` 返回可用技能列表

Agent 可在运行中按需加载技能来扩展能力。
