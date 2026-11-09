---@param b boolean
---@return string
local function explicit(b)
  if b then
    return "true"
  end
  return "false"
end

---@param b boolean
---@return string
local function halfAndHalf(b)
  if b then
    return "true"
  end
  return "false"
end

---@param b boolean
---@return string
local function implicit(b)
  if b then
    return "true"
  else
    return "false"
  end
end

---@param a boolean
---@param b boolean
---@return string
local function nested(a, b)
  local result = "neither"
  if a then
    if b then
      result = "both"
    end
  else
    result = "not a"
  end
  return result
end

local t = true
local f = not t
print("explicit    true  -> " .. explicit(t))
print("explicit    false -> " .. explicit(f))
print("halfAndHalf true  -> " .. halfAndHalf(t))
print("halfAndHalf false -> " .. halfAndHalf(f))
print("implicit    true  -> " .. implicit(t))
print("implicit    false -> " .. implicit(f))
print("nested      true  false -> " .. nested(t, f))
print("nested      false true  -> " .. nested(f, t))

return {}
