local temper = require("temper-core")

---@param n integer
---@return integer
local function twice(n)
  return temper.int32_mul(n, 2)
end

local runs = 0
local base = 10
print("init " .. twice(1))

---@return integer
local function runs__get()
  return runs
end

---@param value integer
local function runs__set(value)
  runs = value
end

return {
  twice = twice,
  ["$test"] = { twice = twice, base = base, runs__get = runs__get, runs__set = runs__set },
}
