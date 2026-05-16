use mlua::{Lua, Function, Value as LuaValue};
use serde_json::{Value as JsonValue, Map};
use std::process::Command;

mod tools;
mod recovery;
mod tui;

// ── JSON ↔ Lua 值转换 ──────────────────────────────────────────

fn value_to_lua(lua: &Lua, val: &JsonValue) -> mlua::Result<LuaValue> {
    match val {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i as i32))
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

fn lua_to_value(lua: &Lua, val: LuaValue) -> mlua::Result<JsonValue> {
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
            let keys: Vec<LuaValue> = tbl
                .clone()
                .pairs::<LuaValue, LuaValue>()
                .filter_map(|p| p.ok().map(|(k, _)| k))
                .collect();
            let is_array = !keys.is_empty()
                && keys.iter().all(|k| matches!(k, LuaValue::Integer(_)));

            if is_array {
                let mut arr = Vec::new();
                for i in 1i32.. {
                    match tbl.get::<i32, LuaValue>(i) {
                        Ok(v) => arr.push(lua_to_value(lua, v)?),
                        Err(_) => break,
                    }
                }
                Ok(JsonValue::Array(arr))
            } else {
                let mut map = Map::new();
                for pair in tbl.pairs::<String, LuaValue>() {
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
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable not set");

    let host_llm = lua.create_async_function(move |lua, messages: mlua::Value| {
        // 同步部分完成 Lua → JSON 转换，async 只捕获 owned 类型
        let rust_val = lua_to_value(&lua, messages)
            .map_err(|e| mlua::Error::external(e))?;
        let api_key = api_key.clone();
        async move {
            let response = tools::llm::llm_chat(
                &api_key,
                "claude-3-5-haiku-20241022",
                rust_val,
            )
            .await
            .map_err(|e| mlua::Error::external(e))?;
            Ok(response)
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

    let json_encode = lua.create_function(|lua, val: LuaValue| {
        let rust_val = lua_to_value(lua, val)
            .map_err(|e| mlua::Error::external(e))?;
        serde_json::to_string(&rust_val).map_err(|e| mlua::Error::external(e))
    })?;
    json_tbl.set("encode", json_encode)?;

    lua.globals().set("json", json_tbl)?;
    lua.globals().set("host", host)?;

    Ok(lua)
}

// ── 入口 ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 模式选择：`cargo run tui` → TUI，默认 → CLI
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "tui" {
        return tui::run_tui();
    }

    // ── CLI 模式 ──
    let task = "What is the current working directory? Use bash to run pwd.";

    let mut detector = recovery::CrashDetector::new(5);

    let result: Result<String, anyhow::Error> = async {
        let lua = create_lua()?;
        let main_loop_code =
            std::fs::read_to_string("agent_plugins/system_core/main_loop.lua")?;
        let main_loop: mlua::Table = lua.load(&main_loop_code).eval()?;
        let run_agent: mlua::Function = main_loop.get("run_agent")?;
        run_agent
            .call_async::<String>(task)
            .await
            .map_err(|e| anyhow::anyhow!("Agent error: {}", e))
    }
    .await;

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
