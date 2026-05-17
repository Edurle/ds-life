use mlua::{Lua, Value as LuaValue};
use serde_json::{Value as JsonValue, Map};
use std::process::Command;

mod tools;
mod recovery;
mod tui;
mod config;
mod persistence;

// ── JSON ↔ Lua 值转换 ──────────────────────────────────────────

fn value_to_lua<'lua>(lua: &'lua Lua, val: &JsonValue) -> mlua::Result<LuaValue<'lua>> {
    match val {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Ok(LuaValue::Nil)
            }
        }
        JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        JsonValue::Array(arr) => {
            let tbl = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                tbl.set(i + 1, value_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(tbl))
        }
        JsonValue::Object(obj) => {
            let tbl = lua.create_table()?;
            for (k, v) in obj.iter() {
                tbl.set(k.as_str(), value_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(tbl))
        }
    }
}

fn lua_to_value<'lua>(
    lua: &'lua Lua,
    val: LuaValue<'lua>,
) -> mlua::Result<JsonValue> {
    match val {
        LuaValue::Nil => Ok(JsonValue::Null),
        LuaValue::Boolean(b) => Ok(JsonValue::Bool(b)),
        LuaValue::Integer(i) => Ok(JsonValue::Number(i.into())),
        LuaValue::Number(n) => serde_json::Number::from_f64(n)
            .map(JsonValue::Number)
            .ok_or_else(|| mlua::Error::external("invalid float")),
        LuaValue::String(s) => Ok(JsonValue::String(s.to_str()?.to_owned())),
        LuaValue::Table(tbl) => {
            // 判断：所有键都是整数 → 数组，否则 → 字典
            let keys: Vec<LuaValue<'_>> = tbl
                .clone()
                .pairs::<LuaValue<'_>, LuaValue<'_>>()
                .filter_map(|p| p.ok().map(|(k, _)| k))
                .collect();
            let is_array = !keys.is_empty()
                && keys.iter().all(|k| matches!(k, LuaValue::Integer(_)));

            if is_array {
                let mut arr = Vec::new();
                for i in 1i32.. {
                    match tbl.get::<i32, LuaValue<'_>>(i) {
                        Ok(LuaValue::Nil) => break,
                        Ok(v) => arr.push(lua_to_value(lua, v)?),
                        Err(_) => break,
                    }
                }
                Ok(JsonValue::Array(arr))
            } else {
                let mut map = Map::new();
                for pair in tbl.pairs::<String, LuaValue<'_>>() {
                    let (k, v) = pair?;
                    map.insert(k, lua_to_value(lua, v)?);
                }
                Ok(JsonValue::Object(map))
            }
        }
        _ => Err(mlua::Error::external("unsupported Lua type")),
    }
}

// ── 创建 Lua 环境并注入工具 ──────────────────────────────────

fn create_lua() -> anyhow::Result<Lua> {
    let lua = Lua::new();

    // 沙箱：清空 OS 和 IO 模块
    lua.globals().set("os", lua.create_table()?)?;
    lua.globals().set("io", lua.create_table()?)?;

    let host = lua.create_table()?;

    // --- 同步工具 ---

    // read_file(path, [limit])
    let host_read = lua.create_function(|_, (path, limit): (String, Option<usize>)| {
        tools::fs::read_file(&path, limit).map_err(|e| mlua::Error::external(e))
    })?;
    host.set("read_file", host_read)?;

    // write_file(path, content)
    let host_write = lua.create_function(|_, (path, content): (String, String)| {
        tools::fs::write_file(&path, &content).map_err(|e| mlua::Error::external(e))
    })?;
    host.set("write_file", host_write)?;

    // bash(command)
    let host_bash = lua.create_function(|_, cmd: String| {
        tools::bash::run_bash(&cmd).map_err(|e| mlua::Error::external(e))
    })?;
    host.set("bash", host_bash)?;

    // list_skills()
    let host_list_skills = lua.create_function(|_, ()| {
        tools::fs::list_skills().map_err(|e| mlua::Error::external(e))
    })?;
    host.set("list_skills", host_list_skills)?;

    // edit_session_start()
    let host_edit_start = lua.create_function(|_, ()| {
        tools::self_update::start_session().map_err(|e| mlua::Error::external(e))
    })?;
    host.set("edit_session_start", host_edit_start)?;

    // update_plugin(session_id, file_name, content)
    let host_update = lua.create_function(
        |_, (sid, name, content): (String, String, String)| {
            tools::self_update::update_plugin(&sid, &name, &content)
                .map_err(|e| mlua::Error::external(e))
        },
    )?;
    host.set("update_plugin", host_update)?;

    // edit_session_commit(session_id, message)
    let host_commit = lua.create_function(|_, (sid, msg): (String, String)| {
        tools::self_update::commit_session(&sid, &msg)
            .map_err(|e| mlua::Error::external(e))
    })?;
    host.set("edit_session_commit", host_commit)?;

    // edit_session_rollback(session_id)
    let host_rollback = lua.create_function(|_, sid: String| {
        tools::self_update::rollback_session(&sid)
            .map_err(|e| mlua::Error::external(e))
    })?;
    host.set("edit_session_rollback", host_rollback)?;

    // --- 异步工具：llm_chat ---
    // 从配置文件 / 环境变量加载配置
    let app_config = config::AppConfig::load()
        .expect("Failed to load config. Set ANTHROPIC_API_KEY env var or api_key in ~/.deepseek/config.toml");

    tools::llm::set_context_size(app_config.context_size);

    let model = app_config.model.clone();
    let base_url = app_config.base_url.clone();

    let host_llm = lua.create_async_function(move |lua, messages: LuaValue<'_>| {
        let api_key = app_config.api_key.clone();
        let model = model.clone();
        let base_url = base_url.clone();
        // 同步转换，结果被 move 进 async block
        let convert_result = lua_to_value(&lua, messages);
        async move {
            let rust_val = convert_result
                .map_err(|e| mlua::Error::external(e))?;

            // 检查是否有流式钩子，有则使用流式 API
            let hook = tools::llm::take_streaming_hook();
            let response = if let Some(hook) = hook {
                let result = tools::llm::llm_chat_streaming(
                    &api_key,
                    &model,
                    &base_url,
                    rust_val,
                    |t| hook(t),
                )
                .await;
                // 放回钩子（多步 tool_use 循环可能再次需要）
                tools::llm::restore_streaming_hook(hook);
                result
            } else {
                tools::llm::llm_chat(&api_key, &model, &base_url, rust_val).await
            };
            Ok(response.map_err(|e| mlua::Error::external(e))?)
        }
    })?;
    host.set("llm_chat", host_llm)?;

    // --- JSON 工具 ---
    let json_tbl = lua.create_table()?;

    let json_decode = lua.create_function(|lua, s: String| {
        let val: JsonValue =
            serde_json::from_str(&s).map_err(|e| mlua::Error::external(e))?;
        value_to_lua(lua, &val)
    })?;
    json_tbl.set("decode", json_decode)?;

    let json_encode = lua.create_function(|lua, val: LuaValue<'_>| {
        let rust_val = lua_to_value(lua, val)
            .map_err(|e| mlua::Error::external(e))?;
        serde_json::to_string(&rust_val).map_err(|e| mlua::Error::external(e))
    })?;
    json_tbl.set("encode", json_encode)?;

    lua.globals().set("json", json_tbl)?;
    lua.globals().set("host", host)?;

    Ok(lua)
}

// ── 公共函数：运行一次 Agent 任务 ─────────────────────────────

/// 创建 Lua 环境，加载 main_loop.lua，执行任务，返回结果。
/// `history`：可选的历史消息（Anthropic API 格式），作为本轮上下文前缀。
pub async fn run_agent_once(task: &str, history: Option<&[serde_json::Value]>) -> anyhow::Result<String> {
    let lua = create_lua()?;
    let main_loop_code = std::fs::read_to_string(
        config::workspace_root().join("agent_plugins/system_core/main_loop.lua"),
    )?;
    let main_loop: mlua::Table<'_> = lua.load(&main_loop_code).eval()?;
    let run_agent: mlua::Function<'_> = main_loop.get("run_agent")?;

    // 将 history 转为 Lua 值
    let history_lua = if let Some(h) = history {
        let json_str = serde_json::to_string(h)?;
        let history_val: serde_json::Value = serde_json::from_str(&json_str)?;
        Some(value_to_lua(&lua, &history_val).map_err(|e| anyhow::anyhow!("{}", e))?)
    } else {
        None
    };

    match history_lua {
        Some(hl) => run_agent
            .call_async::<_, String>((task, 15, hl))
            .await
            .map_err(|e| anyhow::anyhow!("Agent error: {}", e)),
        None => run_agent
            .call_async::<&str, String>(task)
            .await
            .map_err(|e| anyhow::anyhow!("Agent error: {}", e)),
    }
}

// ── 入口 ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 模式选择：`cargo run tui` → TUI，默认 → CLI
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "tui" {
        let demo = args.iter().any(|a| a == "--demo" || a == "-d");
        if args.len() > 2 && !args[2].starts_with('-') {
            // cargo run tui -- "task": 单次任务后退出
            let result = run_agent_once(&args[2], None).await?;
            println!("Agent response:\n{}", result);
            return Ok(());
        }
        let db = persistence::init_db().ok();
        return tui::run_tui(demo, db).await;
    }

    // ── CLI 模式 ──
    let task = "List the files in the current directory, then read Cargo.toml and summarize what dependencies this project uses.";

    let mut detector = recovery::CrashDetector::new(5);

    let result = run_agent_once(task, None).await;

    match &result {
        Ok(resp) => {
            detector.record(true);
            println!("Agent response:\n{}", resp);
        }
        Err(e) => {
            detector.record(false);
            eprintln!("Agent error: {}", e);
        }
    }

    if detector.should_rollback(0.6, 3) {
        eprintln!("Crash threshold exceeded, rolling back last commit...");
        Command::new("git")
            .args(["reset", "--hard", "HEAD~1"])
            .status()?;
        eprintln!("Rollback complete. Restart the agent to continue.");
    }

    Ok(())
}
