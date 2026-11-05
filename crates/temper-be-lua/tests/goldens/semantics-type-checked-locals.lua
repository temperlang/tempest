local temper = require("temper-core")

local function yay()
  print("yay")
end

local function meh()
  print("meh")
end

local function boo()
  print("boo")
end

---@param x integer
---@return integer
local function f(x)
  return x
end

---@param x integer
---@return integer
local function g(x)
  return x
end

---@param x integer
---@return integer
local function i(x)
  return x
end

print("Checkpoint 0")
if not temper.pcall(function()
  ---@type integer
  local a
  temper.bubble()
end) then
  yay()
end
print("Checkpoint 1")
local a2 = 1
yay()
local _ = nil
print("Checkpoint 2")
do
  local a3 = 1
  if not temper.pcall(function()
    temper.bubble()
  end) then
    meh()
  end
  if a3 == 1 then
    meh()
  else
    meh()
  end
end
print("Checkpoint 3")
local a4 = 1
a4 = 2
yay()
print("Checkpoint 4")
if not temper.pcall(function()
  f(0)
  yay()
end) then
  boo()
end
if not temper.pcall(function()
  temper.bubble()
end) then
  meh()
end
print("Checkpoint 5")
if not temper.pcall(function()
  g(0)
  yay()
end) then
  boo()
end
if not temper.pcall(function()
  temper.bubble()
end) then
  meh()
end
print("Checkpoint 6")
if not temper.pcall(function()
  i(0)
  yay()
end) then
  boo()
end
if not temper.pcall(function()
  temper.bubble()
end) then
  meh()
end

return {}
