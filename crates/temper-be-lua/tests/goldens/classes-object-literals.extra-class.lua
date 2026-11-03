local temper = require("temper-core")

---Gives a class's instances a `__tostring` that calls their `toString`.
---@param class temper.Type
local function with_tostring(class)
  local construct = class.constructor
  ---@type table?
  local meta = nil
  class.constructor = function(this, ...)
    if meta == nil then
      meta = {}
      for k, v in _G.pairs(_G.getmetatable(this)) do
        meta[k] = v
      end
      meta.__tostring = function(self)
        return self:toString()
      end
    end
    _G.setmetatable(this, meta)
    construct(this, ...)
  end
end

---@class object-literals.extra-class.Stringable
---@field toString fun(self: object-literals.extra-class.Stringable): string

---@class object-literals.extra-class.Stringable.type: temper.Type
local Stringable = temper.type("object-literals.extra-class.Stringable")

---@class object-literals.extra-class.Circle: object-literals.extra-class.Stringable
---@field radius integer
---@field toString fun(self: object-literals.extra-class.Circle): string

---@class object-literals.extra-class.Circle.type: temper.Type
---@operator call: object-literals.extra-class.Circle
local Circle = temper.type("object-literals.extra-class.Circle", Stringable)

---@param this object-literals.extra-class.Circle
---@param radius integer
function Circle.constructor(this, radius)
  this.radius = radius
end

---@param this object-literals.extra-class.Circle
---@return string
function Circle.methods.toString(this)
  return "Circle of radius " .. this.radius
end

with_tostring(Circle)

---@class object-literals.extra-class.Rectangle
---@field width integer
---@field height integer

---@class object-literals.extra-class.Rectangle.type: temper.Type
---@operator call: object-literals.extra-class.Rectangle
local Rectangle = temper.type("object-literals.extra-class.Rectangle")

---@param this object-literals.extra-class.Rectangle
---@param width integer
---@param height integer
function Rectangle.constructor(this, width, height)
  this.width = width
  this.height = height
end

return { Stringable = Stringable, Circle = Circle, Rectangle = Rectangle }
