local temper = require("temper-core")

---@type integer
local native

---@param n integer
---@return integer
local function lambda(n)
  return temper.int32_add(n, 1)
end

---@class renamed.Checked
---@field end_ integer
---@field delete fun(self: renamed.Checked): integer
---@field local_ integer

---@class renamed.Checked.type: temper.Type
---@operator call: renamed.Checked
local Checked = temper.type("renamed.Checked")

---@param this renamed.Checked
---@param end_ integer
function Checked.constructor(this, end_)
  this.end_ = end_
end

---@param this renamed.Checked
---@return integer
function Checked.methods.delete(this)
  return temper.int32_mul(this.end_, 2)
end

---@param this renamed.Checked
---@return integer
function Checked.get.local_(this)
  return temper.int32_add(this.end_, native)
end

native = 2

return { lambda = lambda, native = native, Checked = Checked }
