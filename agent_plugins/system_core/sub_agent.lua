-- 子智能体
--
-- 在独立的消息上下文中执行子任务，与主循环共享同一组 host.* 工具。
-- 适用于需要并行或递归调用的场景。

local function spawn(task, max_steps)
    max_steps = max_steps or 5
    local messages = { {role = "user", content = "Sub-task: " .. task} }

    for step = 1, max_steps do
        local response_str = host.llm_chat(messages)
        local ok, response = pcall(json.decode, response_str)
        if not ok then
            return "Sub-agent parse error: " .. tostring(response_str)
        end

        local stop_reason = response.stop_reason
        local content = response.content
        table.insert(messages, {role = "assistant", content = content})

        if stop_reason == "end_turn" then
            for _, block in ipairs(content) do
                if block.type == "text" then
                    return block.text
                end
            end
            return "Sub-agent: no text response"
        end

        local tool_results = {}
        for _, block in ipairs(content) do
            if block.type == "tool_use" then
                local handler = host[block.name]
                local result_text
                if not handler then
                    result_text = "Unknown tool: " .. block.name
                else
                    local ok2, res = pcall(function()
                        if block.name == "read_file" then
                            return handler(block.input.path, block.input.limit)
                        elseif block.name == "write_file" then
                            return handler(block.input.path, block.input.content)
                        elseif block.name == "bash" then
                            return handler(block.input.command)
                        else
                            local args = {}
                            for _, v in pairs(block.input or {}) do
                                table.insert(args, v)
                            end
                            return handler(table.unpack(args))
                        end
                    end)
                    result_text = ok2 and tostring(res) or "Error: " .. tostring(res)
                end
                table.insert(tool_results, {
                    type = "tool_result",
                    tool_use_id = block.id,
                    content = result_text,
                })
            end
        end
        table.insert(messages, {role = "user", content = tool_results})
    end

    return "Sub-agent: step limit exceeded"
end

return { spawn = spawn }
