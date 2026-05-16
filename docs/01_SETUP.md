# 第1步：准备工作与创建项目

## 1.1 安装 Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 按提示选择默认安装 (1)，等待完成
source "$HOME/.cargo/env"
rustc --version   # 应 >= 1.75
cargo --version
```

## 1.2 安装 Lua（仅供本地测试，非必须）

项目内嵌 Lua，无需系统安装。但调试用：
- Ubuntu: `sudo apt install lua5.4`
- macOS: `brew install lua`

## 1.3 安装 Git

```sh
git --version
# 若未找到，从 https://git-scm.com/downloads 下载
```

## 1.4 设置 Anthropic API Key

注册 [Anthropic Console](https://console.anthropic.com/) 获取密钥。

```sh
# ~/.bashrc 或 ~/.zshrc 中添加：
export ANTHROPIC_API_KEY="sk-ant-你的密钥"
# 然后：
source ~/.zshrc
```

## 1.5 创建 Rust 项目

```sh
cd /home/dzj/file/ds-life
cargo new agent_host
cd agent_host
```

## 1.6 写入 Cargo.toml

替换 `agent_host/Cargo.toml` 内容（见 `Cargo.toml` 文件）。

## 1.7 创建 Lua 测试脚本

`scripts/test.lua`：
```lua
print("Hello from Lua! 1+1 = " .. (1+1))
```

## 1.8 验证

```sh
cd agent_host && cargo run
# 应输出：Hello from Lua! 1+1 = 2
```
