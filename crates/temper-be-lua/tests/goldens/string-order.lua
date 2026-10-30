local temper = require("temper-core")

---@param a string
---@param b string
---@return string
local function show(a, b)
  return temper.str_cmp(a, b) .. " " .. tostring(a < b) .. " " .. tostring(a <= b) .. " " .. tostring(a > b) .. " " .. tostring(a >= b)
end

---@param a number
---@param b number
---@return integer
local function cmp(a, b)
  return temper.float_cmp(a, b)
end

print(show("｡", "😀"))
print(show("😀", "｡"))
print(show("", "𐀀"))
print(show("￿", "􏿿"))
print(show("a｡b", "a😀"))
print(show("😀", "😁"))
print(show("😀", "😀"))
print(show("😀", "😀a"))
print(show("", "😀"))
print(show("", ""))
print(show("z", "a"))
print(show("abc", "abd"))
print(cmp(2.5, 1.0) .. " " .. cmp(1.0, 2.5) .. " " .. cmp(1.5, 1.5))

return {}
