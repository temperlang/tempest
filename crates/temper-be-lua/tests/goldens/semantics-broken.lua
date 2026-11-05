local temper = require("temper-core")

---@return integer
local function undefinedPlusArgInside()
  return temper.bubble()
end

---@param string string
---@param minSize integer
---@param pad string
---@return string
local function padLeft(string, minSize, pad)
  local needed = temper.int32_sub(minSize, temper.string_countbetween(string, 1, temper.string_end(string)))
  if needed <= 0 then
    return string
  end
  return temper.bubble()
end

temper.bubble()

return {}
