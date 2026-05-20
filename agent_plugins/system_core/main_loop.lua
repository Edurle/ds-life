-- Agent 主循环
--
-- 流程：用户输入 → LLM (Anthropic API) → tool_use → Lua 调用 Rust 工具
--      → tool_result → LLM 分析 → 最终回答 → 用户
--
-- llm_chat 返回的是原始 JSON 字符串（来自 Anthropic API），
-- 需要用 json.decode 解析后获取 content / stop_reason 等字段。

local function run_agent(task, max_steps, initial_messages)
    max_steps = max_steps or 15
    local messages = {}
    if initial_messages and #initial_messages > 0 then
        for _, msg in ipairs(initial_messages) do
            table.insert(messages, msg)
        end
        table.insert(messages, {role = "user", content = task})
    else
        messages = { {role = "user", content = task} }
    end

    for step = 1, max_steps do
        local response_str = host.llm_chat(messages)
        local ok, response = pcall(json.decode, response_str)
        if not ok then
            return "Failed to parse LLM response: " .. tostring(response_str)
        end

        local stop_reason = response.stop_reason
        local content = response.content

        -- 构建 assistant 消息
        local assistant_msg = {role = "assistant", content = content}
        table.insert(messages, assistant_msg)

        -- 若 LLM 结束对话，提取最终文本
        if stop_reason == "end_turn" then
            for _, block in ipairs(content) do
                if block.type == "text" then
                    return block.text
                end
            end
            return "No text in final response"
        end

        -- 处理工具调用
        local tool_results = {}
        for _, block in ipairs(content) do
            if block.type == "tool_use" then
                local tool_name = block.name
                local tool_input = block.input or {}
                local tool_id = block.id

                local handler = host[tool_name]
                local tool_result_content
                local is_error = false

                if not handler then
                    tool_result_content = "Error: Unknown tool '" .. tool_name .. "'"
                    is_error = true
                else
                    local ok2, result = pcall(function()
                        -- 根据工具名调整参数传递方式
                        if tool_name == "read_file" then
                            return handler(tool_input.path, tool_input.limit)
                        elseif tool_name == "write_file" then
                            return handler(tool_input.path, tool_input.content)
                        elseif tool_name == "bash" then
                            return handler(tool_input.command)
                        elseif tool_name == "edit_session_start" then
                            return handler()
                        elseif tool_name == "update_plugin" then
                            return handler(tool_input.session_id, tool_input.file_name, tool_input.content)
                        elseif tool_name == "edit_session_commit" then
                            return handler(tool_input.session_id, tool_input.message)
                        elseif tool_name == "edit_session_rollback" then
                            return handler(tool_input.session_id)
                        elseif tool_name == "compile_with_feedback" then
                            return handler(tool_input.crate_dir or ".", tool_input.check_only or true)
                        elseif tool_name == "grep_files" then
                            return handler(tool_input.pattern, tool_input.search_path or ".",
                                tool_input.include or "", tool_input.context_lines or 2, tool_input.max_results or 50)
                        elseif tool_name == "file_search" then
                            return handler(tool_input.query, tool_input.search_path or ".",
                                tool_input.extensions or "", tool_input.max_results or 20)
                        elseif tool_name == "apply_patch" then
                            return handler(tool_input.path, json.encode(tool_input.operations))
                        else
                            -- 通用方式：将 input 的字段按顺序展开
                            local args = {}
                            for _, v in pairs(tool_input) do
                                table.insert(args, v)
                            end
                            return handler(table.unpack(args))
                        end
                    end)

                    if not ok2 then
                        tool_result_content = "Error: " .. tostring(result)
                        is_error = true
                    else
                        tool_result_content = tostring(result)
                    end
                end

                table.insert(tool_results, {
                    type = "tool_result",
                    tool_use_id = tool_id,
                    content = tool_result_content,
                    is_error = is_error,
                })
            end
        end

        -- 将 tool_result 作为 user 消息追加
        table.insert(messages, {role = "user", content = tool_results})
    end

    return "Task step limit exceeded after " .. max_steps .. " steps"
end

return { run_agent = run_agent }
