-- Host code plugging its own client into std/net, for `functional.rs`'s
-- `runs`: before the generated `netresponse` and `netresponse-not-found`
-- libraries run, it sets a client that answers as the test server does,
-- later, from an I/O source the runner waits on when no step is ready.
-- They log as the funtests expect, then this prints how many requests the
-- client saw.

local temper = require("temper-core")

local pending = {}
local sent = 0

-- Answers one request each time the runner waits.
temper.add_io_source(function()
  local answer = table.remove(pending, 1)
  if answer ~= nil then
    answer.respond(answer.response)
  end
  return #pending > 0
end)

-- Answers as the test server: 404 at `/missing`, nothing at port 1, and
-- otherwise what the method was and whether the body was JSON.
temper.set_net_client({
  send = function(_, request, respond)
    sent = sent + 1
    local response
    if request.url:find(":1/", 1, true) then
      response = nil
    elseif request.url:find("/missing", 1, true) then
      response = { status = 404, content_type = "text/plain", body_content = "not here" }
    else
      local json = tostring(request.body_mime_type == "application/json")
      response = {
        status = 200,
        content_type = "application/json",
        body_content = '{"method": "' .. request.method .. '", "json": ' .. json .. "}",
      }
    end
    pending[#pending + 1] = { respond = respond, response = response }
  end,
})

require("netresponse")
require("netresponse-not-found")
print("requests " .. sent)
temper.set_net_client(nil)
