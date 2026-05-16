-- 自我迭代功能
--
-- Agent 通过此函数修改自身的 main_loop.lua 代码。
-- 流程：
--   1. 读取当前 main_loop.lua
--   2. 调用 LLM 要求改进
--   3. 用 load() 做语法验证
--   4. 创建编辑会话 → 更新 → commit

local function improve_self(reason)
    local code = host.read_file("agent_plugins/system_core/main_loop.lua")
    local prompt = "Current agent main loop code:\n```lua\n"
        .. code
        .. "\n```\n\nNeed to improve because: "
        .. reason
        .. "\n\nProvide the improved version of main_loop.lua code ONLY, wrapped in ```lua ... ```."

    local resp_str = host.llm_chat({{role = "user", content = prompt}})
    local ok, resp = pcall(json.decode, resp_str)
    if not ok then
        return "Failed to parse LLM response: " .. tostring(resp_str)
    end

    -- 提取文本内容
    local new_code = nil
    for _, block in ipairs(resp.content) do
        if block.type == "text" then
            new_code = block.text
            break
        end
    end
    if not new_code then
        return "No text in self-improvement response"
    end

    -- 尝试从代码块标记中提取
    local extracted = string.match(new_code, "```lua\n(.-)```")
    if extracted then
        new_code = extracted
    end

    -- 安全验证：新代码必须能通过 Lua 语法检查
    local test_fn, load_err = load(new_code)
    if not test_fn then
        return "Self-improvement rejected: syntax error: " .. tostring(load_err)
    end

    local sid = host.edit_session_start()
    host.update_plugin(sid, "system_core/main_loop.lua", new_code)
    host.edit_session_commit(sid, "Self-improvement: " .. reason)
    return "Self-improvement applied: " .. reason
end

return { improve_self = improve_self }
