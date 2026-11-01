local temper = require("temper-core")

---@class constructor-callbacks.Hi
---@field ha fun(arg0: any): any

---@class constructor-callbacks.Hi.type: temper.Type
---@operator call: constructor-callbacks.Hi
local Hi = temper.type("constructor-callbacks.Hi")

---@param this constructor-callbacks.Hi
---@param ha fun(arg0: any): any
function Hi.constructor(this, ha)
  this.ha = ha
end

---@generic T
---@param ha fun(arg0: T): T
---@return constructor-callbacks.Hi
local function makeHi(ha)
  return Hi(ha)
end

---@param j integer
---@return constructor-callbacks.Hi
local function newIntHi(j)
  return Hi(function(i) return temper.int32_add(i, j) end)
end

---@param j integer
---@return constructor-callbacks.Hi
local function makeIntHi(j)
  return makeHi(function(i) return temper.int32_add(i, j) end)
end

return { Hi = Hi, makeHi = makeHi, newIntHi = newIntHi, makeIntHi = makeIntHi }
