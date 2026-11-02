local temper = require("temper-core")

---@param text string
---@param open? string|temper.Null
---@param close? string|temper.Null
---@return string
local function wrap(text, open, close)
  if temper.is_null(open) then
    open = "["
  end
  ---@cast open string
  if temper.is_null(close) then
    close = open
  end
  ---@cast close string
  return open .. text .. close
end

---@class defaults.helpers.Pad
---@field left integer
---@field right integer
---@field apply fun(self: defaults.helpers.Pad, text: string, fill?: string|temper.Null): string

---@class defaults.helpers.Pad.type: temper.Type
---@operator call: defaults.helpers.Pad
local Pad = temper.type("defaults.helpers.Pad")

---@param this defaults.helpers.Pad
---@param left? integer|temper.Null
---@param right? integer|temper.Null
function Pad.constructor(this, left, right)
  if temper.is_null(left) then
    left = 1
  end
  ---@cast left integer
  if temper.is_null(right) then
    right = 2
  end
  ---@cast right integer
  this.left = left
  this.right = right
end

---@param this defaults.helpers.Pad
---@param text string
---@param fill? string|temper.Null
---@return string
function Pad.methods.apply(this, text, fill)
  if temper.is_null(fill) then
    fill = "."
  end
  ---@cast fill string
  local out = text
  do
    local i = 0
    while i < this.left do
      out = fill .. out
      i = temper.int32_add(i, 1)
    end
  end
  do
    local i = 0
    while i < this.right do
      out = out .. fill
      i = temper.int32_add(i, 1)
    end
  end
  return out
end

return { wrap = wrap, Pad = Pad }
