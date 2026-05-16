-- 技能加载器
--
-- 技能存放在 agent_plugins/skills/<name>/SKILL.md 中。
-- Agent 可在运行中按需加载技能来获取领域知识。

local function load_skill(skill_name)
    local skill_path = "agent_plugins/skills/" .. skill_name .. "/SKILL.md"
    local ok, content = pcall(host.read_file, skill_path)
    if not ok then
        return nil, "Failed to read skill: " .. tostring(content)
    end
    return content
end

local function list_skills()
    local ok, skills = pcall(host.list_skills)
    if not ok then
        return {}
    end
    return skills
end

return { load_skill = load_skill, list_skills = list_skills }
