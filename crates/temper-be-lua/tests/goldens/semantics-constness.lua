local temper = require("temper-core")

do
  local nci = 1
  print("nci = " .. nci)
  nci = 2
  local _ = nil
  print("nci = " .. nci)
  ---@type integer
  local nc
  nc = 1
  print("nc = " .. nc)
  nc = 2
  local _ = nil
  print("nc = " .. nc)
  do
    local i = 0
    while i < 3 do
      ---@type integer
      local ncl
      ncl = i
      local _ = nil
      print("ncl = " .. ncl)
      i = temper.int32_add(i, 1)
    end
  end
  local ci = 1
  print("ci = " .. ci)
  print("failed to set ci")
  print("ci = " .. ci)
  ---@type integer
  local c
  c = 1
  print("c = " .. c)
  print("failed to set c")
  print("c = " .. c)
  do
    local i2 = 0
    while i2 < 3 do
      ---@type integer
      local cl
      cl = i2
      local _ = nil
      print("cl = " .. cl)
      i2 = temper.int32_add(i2, 1)
    end
  end
end

return {}
