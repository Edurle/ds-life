# 记忆演化

## 概念

不光是向量存储，而是基于 `self_improve.lua` 的机制：Agent 将重要经验**编译为新的 Lua 工具函数**沉淀到 `system_core/` 中。

## 工作流

```
Agent 反思 ← "我今天学会了处理 OOM 问题"
  ↓
生成新工具函数: solve_oom(context)
  ↓
通过 load() 语法检查
  ↓
写入 system_core/learned/oom_handler.lua
  ↓
下次遇到 OOM 时自动加载此工具
```

## 记忆分类

```
system_core/learned/
├── bash_patterns/      ← 经过验证的 shell 命令模式
│   └── find_large_files.lua
├── code_reviews/       ← 常见代码问题检查
│   └── check_null_ptr.lua
├── error_handlers/     ← 已知错误的自动修复
│   └── oom_handler.lua
└── index.lua           ← 所有学习到的工具索引
```

## 演进方向

1. 初期：Agent 手动触发 `improve_self` 沉淀经验
2. 中期：自动检测重复性成功/失败模式，触发沉淀
3. 长期：Agent 间的经验共享（技能市场 + 记忆交换）
