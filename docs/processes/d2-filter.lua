-- Generic Pandoc filter to process D2 code blocks
-- Usage: pandoc --lua-filter d2-filter.lua ...
local function d2(code)
  local name = pandoc.sha1(code)
  local out_file = "/tmp/" .. name .. ".svg"
  
  local f = io.open(out_file, "r")
  if f then
    f:close()
  else
    local handle = io.popen("d2 - " .. out_file, "w")
    if handle then
      handle:write(code)
      handle:close()
    else
      return nil
    end
  end
  
  return out_file
end

function CodeBlock(el)
  if el.classes[1] == "d2" then
    local svg_path = d2(el.text)
    if svg_path then
      local f = io.open(svg_path, "r")
      if f then
        local svg_content = f:read("*all")
        f:close()
        return pandoc.RawBlock("html", svg_content)
      end
    end
  end
end
