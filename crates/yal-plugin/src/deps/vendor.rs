use mlua::{Lua, Result as LuaResult};

/// Add a package.searcher that loads pure-Lua files from `vendor_dir`.
/// Supports `require("a.b.c")` -> vendor/a/b/c.lua or vendor/a/b/c/init.lua
pub fn add_vendor_searcher(lua: &Lua, vendor_dir: &std::path::Path) -> LuaResult<()> {
    let vendor = vendor_dir.to_string_lossy().replace('\\', "\\\\");
    let script = format!(
        r#"
local vendor = "{vendor}"
table.insert(package.searchers, 2, function(modname)
  local rel = (modname:gsub('%.','/'))
  local tried = {{
    vendor .. "/" .. rel .. ".lua",
    vendor .. "/" .. rel .. "/init.lua",
  }}
  for _, p in ipairs(tried) do
    local f = io.open(p, "r")
    if f then
      local src = f:read("*a"); f:close()
      local chunk, err = load(src, "@"..p)
      if not chunk then return ("\nerror loading %s: %s"):format(p, err) end
      return chunk
    end
  end
  return "\nno module '"..modname.."' in vendor ("..table.concat(tried, ", ")..")"
end)
"#
    );
    lua.load(&script).exec()
}
