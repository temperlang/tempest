local temper = require("temper-core")

do
  local i = 0
  while i < 5 do
    if i == 1 or i == 3 then
      i = temper.int32_add(i, 1)
      goto continue
    end
    print(tostring(i))
    i = temper.int32_add(i, 1)
    ::continue::
  end
end
do
  local i2 = 0
  while i2 < 5 do
    if i2 == 3 then
      break
    end
    print(tostring(i2))
    i2 = temper.int32_add(i2, 1)
  end
end
local n = 0
do
  local i3 = 0
  while i3 < 4 do
    local str = "row " .. i3 .. " ="
    local j = 0
    while true do
      local t = n
      n = temper.int32_add(t, 1)
      str = str .. " " .. t
      if i3 <= j then
        print(str)
        i3 = temper.int32_add(i3, 1)
        goto continue_outer
      end
      j = temper.int32_add(j, 1)
    end
    ::continue_outer::
  end
end
local prime = 1
do
  local i4 = 0
  while i4 < 25 do
    do
      local j2 = 0
      while j2 < 25 do
        do
          local k = 0
          while k < 25 do
            prime = temper.int32_add(prime, 1)
            if prime ~= 2 and temper.int32_mod(prime, 2) == 0 then
              i4 = temper.int32_add(i4, 1)
              goto continue_outer2
            end
            if prime ~= 3 and temper.int32_mod(prime, 3) == 0 then
              j2 = temper.int32_add(j2, 1)
              goto continue_outer3
            end
            if prime ~= 5 and temper.int32_mod(prime, 5) == 0 then
              k = temper.int32_add(k, 1)
              goto continue
            end
            if prime ~= 7 and temper.int32_mod(prime, 7) == 0 then
              k = temper.int32_add(k, 1)
              goto continue
            end
            print(tostring(prime))
            k = temper.int32_add(k, 1)
            ::continue::
          end
        end
        j2 = temper.int32_add(j2, 1)
        ::continue_outer3::
      end
    end
    i4 = temper.int32_add(i4, 1)
    ::continue_outer2::
  end
end
local val = 0
do
  local i5 = 0
  while i5 < 10 do
    local str2 = "row " .. i5 .. " ="
    local j3 = 0
    while true do
      local t2 = val
      val = temper.int32_add(t2, 1)
      str2 = str2 .. " " .. t2
      if i5 == 4 then
        print("break")
        goto break_outer
      end
      if i5 <= j3 then
        print(str2)
        i5 = temper.int32_add(i5, 1)
        goto continue_outer4
      end
      j3 = temper.int32_add(j3, 1)
    end
    ::continue_outer4::
  end
  ::break_outer::
end

return {}
