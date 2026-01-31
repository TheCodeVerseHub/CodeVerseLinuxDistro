-- CVH Icons Lua IPC Handler
-- Runs inside bubblewrap sandbox and handles communication with the Rust daemon
--
-- This script:
-- 1. Reads JSON-encoded Request messages from stdin
-- 2. Processes requests (render, events, position, etc.)
-- 3. Sends JSON-encoded Response messages to stdout

local PROTOCOL_VERSION = 1

-- Simple JSON implementation for sandboxed environment
local json = {}

function json.encode(val)
    local t = type(val)
    if t == "nil" then
        return "null"
    elseif t == "boolean" then
        return val and "true" or "false"
    elseif t == "number" then
        if val ~= val then -- NaN
            return "null"
        elseif val == math.huge then
            return "1e309"
        elseif val == -math.huge then
            return "-1e309"
        else
            return tostring(val)
        end
    elseif t == "string" then
        -- Escape special characters
        local escaped = val:gsub('\\', '\\\\')
                           :gsub('"', '\\"')
                           :gsub('\n', '\\n')
                           :gsub('\r', '\\r')
                           :gsub('\t', '\\t')
        return '"' .. escaped .. '"'
    elseif t == "table" then
        -- Check if array or object
        local is_array = true
        local max_index = 0
        for k, _ in pairs(val) do
            if type(k) ~= "number" or k <= 0 or math.floor(k) ~= k then
                is_array = false
                break
            end
            if k > max_index then max_index = k end
        end
        -- Empty table is treated as object
        if max_index == 0 then is_array = false end

        if is_array then
            local parts = {}
            for i = 1, max_index do
                table.insert(parts, 1, json.encode(val[i]))
            end
            -- Reverse to maintain order
            local result = {}
            for i = #parts, 1, -1 do
                table.insert(result, 1, parts[i])
            end
            -- Actually use simple loop
            local arr_parts = {}
            for i = 1, max_index do
                arr_parts[i] = json.encode(val[i])
            end
            return "[" .. table.concat(arr_parts, ",") .. "]"
        else
            local parts = {}
            for k, v in pairs(val) do
                local key = type(k) == "string" and k or tostring(k)
                table.insert(parts, 1, json.encode(key) .. ":" .. json.encode(v))
            end
            -- Rebuild to fix ordering
            local obj_parts = {}
            for k, v in pairs(val) do
                local key = type(k) == "string" and k or tostring(k)
                table.insert(obj_parts, json.encode(key) .. ":" .. json.encode(v))
            end
            return "{" .. table.concat(obj_parts, ",") .. "}"
        end
    else
        return "null"
    end
end

function json.decode(str)
    local pos = 1

    local function skip_whitespace()
        while pos <= #str do
            local c = str:sub(pos, pos)
            if c == " " or c == "\t" or c == "\n" or c == "\r" then
                pos = pos + 1
            else
                break
            end
        end
    end

    local function parse_value()
        skip_whitespace()
        local c = str:sub(pos, pos)

        if c == '"' then
            return parse_string()
        elseif c == '{' then
            return parse_object()
        elseif c == '[' then
            return parse_array()
        elseif c == 't' then
            if str:sub(pos, pos + 3) == "true" then
                pos = pos + 4
                return true
            end
        elseif c == 'f' then
            if str:sub(pos, pos + 4) == "false" then
                pos = pos + 5
                return false
            end
        elseif c == 'n' then
            if str:sub(pos, pos + 3) == "null" then
                pos = pos + 4
                return nil
            end
        elseif c == '-' or (c >= '0' and c <= '9') then
            return parse_number()
        end

        error("Invalid JSON at position " .. pos)
    end

    function parse_string()
        pos = pos + 1 -- skip opening quote
        local result = ""
        while pos <= #str do
            local c = str:sub(pos, pos)
            if c == '"' then
                pos = pos + 1
                return result
            elseif c == '\\' then
                pos = pos + 1
                local esc = str:sub(pos, pos)
                if esc == 'n' then result = result .. '\n'
                elseif esc == 'r' then result = result .. '\r'
                elseif esc == 't' then result = result .. '\t'
                elseif esc == '"' then result = result .. '"'
                elseif esc == '\\' then result = result .. '\\'
                elseif esc == '/' then result = result .. '/'
                elseif esc == 'u' then
                    -- Unicode escape (simplified, just skip)
                    pos = pos + 4
                    result = result .. '?'
                else
                    result = result .. esc
                end
                pos = pos + 1
            else
                result = result .. c
                pos = pos + 1
            end
        end
        error("Unterminated string")
    end

    function parse_number()
        local start = pos
        -- Optional minus
        if str:sub(pos, pos) == '-' then pos = pos + 1 end
        -- Integer part
        while pos <= #str and str:sub(pos, pos) >= '0' and str:sub(pos, pos) <= '9' do
            pos = pos + 1
        end
        -- Decimal part
        if str:sub(pos, pos) == '.' then
            pos = pos + 1
            while pos <= #str and str:sub(pos, pos) >= '0' and str:sub(pos, pos) <= '9' do
                pos = pos + 1
            end
        end
        -- Exponent
        local e = str:sub(pos, pos)
        if e == 'e' or e == 'E' then
            pos = pos + 1
            local sign = str:sub(pos, pos)
            if sign == '+' or sign == '-' then pos = pos + 1 end
            while pos <= #str and str:sub(pos, pos) >= '0' and str:sub(pos, pos) <= '9' do
                pos = pos + 1
            end
        end
        return tonumber(str:sub(start, pos - 1))
    end

    function parse_array()
        pos = pos + 1 -- skip [
        local arr = {}
        local idx = 1
        skip_whitespace()
        if str:sub(pos, pos) == ']' then
            pos = pos + 1
            return arr
        end
        while true do
            arr[idx] = parse_value()
            idx = idx + 1
            skip_whitespace()
            local c = str:sub(pos, pos)
            if c == ']' then
                pos = pos + 1
                return arr
            elseif c == ',' then
                pos = pos + 1
            else
                error("Expected ',' or ']' in array at position " .. pos)
            end
        end
    end

    function parse_object()
        pos = pos + 1 -- skip {
        local obj = {}
        skip_whitespace()
        if str:sub(pos, pos) == '}' then
            pos = pos + 1
            return obj
        end
        while true do
            skip_whitespace()
            local key = parse_string()
            skip_whitespace()
            if str:sub(pos, pos) ~= ':' then
                error("Expected ':' after key at position " .. pos)
            end
            pos = pos + 1
            obj[key] = parse_value()
            skip_whitespace()
            local c = str:sub(pos, pos)
            if c == '}' then
                pos = pos + 1
                return obj
            elseif c == ',' then
                pos = pos + 1
            else
                error("Expected ',' or '}' in object at position " .. pos)
            end
        end
    end

    local result = parse_value()
    skip_whitespace()
    return result
end

-- IPC Communication
local IPC = {}

-- Read a length-prefixed message from stdin
function IPC.receive()
    -- Read 4-byte length prefix (little-endian)
    local len_bytes = io.read(4)
    if not len_bytes or #len_bytes < 4 then
        return nil, "Connection closed or read error"
    end

    local b1, b2, b3, b4 = string.byte(len_bytes, 1, 4)
    local length = b1 + b2 * 256 + b3 * 65536 + b4 * 16777216

    if length > 1048576 then -- 1 MB limit
        return nil, "Message too large"
    end

    -- Read the JSON message
    local data = io.read(length)
    if not data or #data < length then
        return nil, "Incomplete message"
    end

    local ok, result = pcall(json.decode, data)
    if not ok then
        return nil, "JSON decode error: " .. tostring(result)
    end

    return result
end

-- Send a length-prefixed message to stdout
function IPC.send(msg)
    local data = json.encode(msg)
    local length = #data

    -- Write 4-byte length prefix (little-endian)
    local b1 = length % 256
    local b2 = math.floor(length / 256) % 256
    local b3 = math.floor(length / 65536) % 256
    local b4 = math.floor(length / 16777216) % 256

    io.write(string.char(b1, b2, b3, b4))
    io.write(data)
    io.flush()
end

-- Canvas implementation for collecting draw commands
local Canvas = {}
Canvas.__index = Canvas

function Canvas.new(width, height)
    local self = setmetatable({}, Canvas)
    self.width = width or 64
    self.height = height or 80
    self.commands = {}
    return self
end

function Canvas:fill_rect(x, y, w, h, color)
    table.insert(self.commands, 1, {
        type = "FillRect",
        x = x, y = y, w = w, h = h,
        color = color
    })
    -- Fix ordering by reinserting at end
    local cmd = table.remove(self.commands, 1)
    self.commands[#self.commands + 1] = cmd
end

function Canvas:stroke_rect(x, y, w, h, color, stroke_width)
    self.commands[#self.commands + 1] = {
        type = "StrokeRect",
        x = x, y = y, w = w, h = h,
        color = color, width = stroke_width or 1
    }
end

function Canvas:fill_circle(cx, cy, r, color)
    self.commands[#self.commands + 1] = {
        type = "FillCircle",
        cx = cx, cy = cy, r = r,
        color = color
    }
end

function Canvas:stroke_circle(cx, cy, r, color, stroke_width)
    self.commands[#self.commands + 1] = {
        type = "StrokeCircle",
        cx = cx, cy = cy, r = r,
        color = color, width = stroke_width or 1
    }
end

function Canvas:line(x1, y1, x2, y2, color, stroke_width)
    self.commands[#self.commands + 1] = {
        type = "Line",
        x1 = x1, y1 = y1, x2 = x2, y2 = y2,
        color = color, width = stroke_width or 1
    }
end

function Canvas:text(text, x, y, size, color, align)
    self.commands[#self.commands + 1] = {
        type = "Text",
        text = text, x = x, y = y,
        size = size, color = color,
        align = align or "left"
    }
end

function Canvas:image(path, x, y, w, h)
    self.commands[#self.commands + 1] = {
        type = "Image",
        path = path, x = x, y = y, w = w, h = h
    }
end

function Canvas:clear(color)
    self.commands[#self.commands + 1] = {
        type = "Clear",
        color = color
    }
end

function Canvas:get_width()
    return self.width
end

function Canvas:get_height()
    return self.height
end

-- Icon script manager
local IconManager = {}
IconManager.loaded_script = nil
IconManager.icon = nil

function IconManager.load_script(script_path)
    -- Reset any previously loaded script
    IconManager.icon = nil
    Icon = nil

    -- Load and execute the script
    local chunk, err = loadfile(script_path)
    if not chunk then
        return false, "Failed to load script: " .. tostring(err)
    end

    local ok, result = pcall(chunk)
    if not ok then
        return false, "Failed to execute script: " .. tostring(result)
    end

    -- The script should define a global Icon table or return it
    if type(result) == "table" then
        IconManager.icon = result
    elseif type(Icon) == "table" then
        IconManager.icon = Icon
    else
        return false, "Script did not define an Icon table"
    end

    IconManager.loaded_script = script_path
    return true
end

function IconManager.set_metadata(metadata)
    if not IconManager.icon then
        return false, "No icon loaded"
    end

    local icon = IconManager.icon
    icon.path = metadata.path or ""
    icon.name = metadata.name or ""
    icon.width = metadata.width or 64
    icon.height = metadata.height or 80
    icon.selected = metadata.selected or false
    icon.hovered = metadata.hovered or false

    return true
end

function IconManager.call_init()
    if not IconManager.icon then
        return false, "No icon loaded"
    end

    if type(IconManager.icon.init) == "function" then
        local ok, err = pcall(IconManager.icon.init, IconManager.icon)
        if not ok then
            return false, "init() failed: " .. tostring(err)
        end
    end

    return true
end

function IconManager.call_render(canvas_width, canvas_height)
    if not IconManager.icon then
        return nil, "No icon loaded"
    end

    local canvas = Canvas.new(canvas_width, canvas_height)

    if type(IconManager.icon.render) == "function" then
        local ok, err = pcall(IconManager.icon.render, IconManager.icon, canvas)
        if not ok then
            return nil, "render() failed: " .. tostring(err)
        end
    end

    return canvas.commands
end

function IconManager.call_event(event)
    if not IconManager.icon then
        return nil, "No icon loaded"
    end

    local icon = IconManager.icon
    local handled = false
    local action = nil

    if event.type == "Click" then
        if type(icon.on_click) == "function" then
            local ok, result = pcall(icon.on_click, icon, event.button, event.x, event.y)
            if ok and result then
                handled = true
                if type(result) == "string" then
                    action = { action = result, payload = icon.path }
                end
            end
        end
    elseif event.type == "HoverEnter" then
        if type(icon.on_hover) == "function" then
            local ok, _ = pcall(icon.on_hover, icon, true)
            handled = ok
        end
    elseif event.type == "HoverExit" then
        if type(icon.on_hover) == "function" then
            local ok, _ = pcall(icon.on_hover, icon, false)
            handled = ok
        end
    elseif event.type == "Drop" then
        if type(icon.on_drop) == "function" then
            local ok, result = pcall(icon.on_drop, icon, event.paths)
            if ok and result then
                handled = true
                if type(result) == "string" then
                    action = { action = result, payload = nil }
                end
            elseif ok then
                handled = true
            end
        end
    elseif event.type == "Selected" then
        icon.selected = true
        handled = true
    elseif event.type == "Deselected" then
        icon.selected = false
        handled = true
    end

    return { handled = handled, action = action }
end

function IconManager.call_get_position(input)
    if not IconManager.icon then
        return nil, "No icon loaded"
    end

    local icon = IconManager.icon

    if type(icon.get_position) == "function" then
        local ok, result = pcall(icon.get_position, icon, input)
        if ok and type(result) == "table" then
            return { x = result.x or 0, y = result.y or 0 }
        elseif not ok then
            return nil, "get_position() failed: " .. tostring(result)
        end
    end

    -- Default grid-based positioning
    local cell_w = input.cell_width or 96
    local cell_h = input.cell_height or 96
    local margin = 20
    local cols = math.floor((input.screen_width - margin * 2) / cell_w)
    if cols < 1 then cols = 1 end

    local col = input.icon_index % cols
    local row = math.floor(input.icon_index / cols)

    return {
        x = margin + col * cell_w,
        y = margin + row * cell_h
    }
end

-- Request handlers
local Handlers = {}

function Handlers.Handshake(request)
    local remote_version = request.version or 0
    local success = (remote_version == PROTOCOL_VERSION)
    return {
        type = "HandshakeAck",
        version = PROTOCOL_VERSION,
        success = success
    }
end

function Handlers.Render(request)
    local metadata = request.metadata
    local context = request.context

    -- Determine script path from icon_type or use default
    local script_path = request.script_path
    if not script_path then
        -- Use environment variable or default
        script_path = os.getenv("CVH_ICON_SCRIPT") or "/usr/share/cvh-icons/scripts/file.lua"
    end

    -- Load script if not already loaded or if different
    if IconManager.loaded_script ~= script_path then
        local ok, err = IconManager.load_script(script_path)
        if not ok then
            return { type = "Error", message = err }
        end
    end

    -- Set metadata
    local ok, err = IconManager.set_metadata(metadata)
    if not ok then
        return { type = "Error", message = err }
    end

    -- Call init if first time
    ok, err = IconManager.call_init()
    if not ok then
        return { type = "Error", message = err }
    end

    -- Call render
    local commands, err = IconManager.call_render(
        context.canvas_width or metadata.width or 64,
        context.canvas_height or metadata.height or 80
    )
    if not commands then
        return { type = "Error", message = err }
    end

    return {
        type = "Render",
        commands = commands
    }
end

function Handlers.Event(request)
    local event = request.event

    local result, err = IconManager.call_event(event)
    if not result then
        return { type = "Error", message = err }
    end

    return {
        type = "Event",
        handled = result.handled,
        action = result.action
    }
end

function Handlers.Position(request)
    local input = request.input

    local position, err = IconManager.call_get_position(input)
    if not position then
        return { type = "Error", message = err }
    end

    return {
        type = "Position",
        position = position
    }
end

function Handlers.Shutdown(request)
    return { type = "ShutdownAck" }
end

-- Main IPC loop
local function main()
    -- Set stdin/stdout to binary mode if possible
    if io.stdin.setvbuf then
        io.stdin:setvbuf("no")
    end
    if io.stdout.setvbuf then
        io.stdout:setvbuf("no")
    end

    local running = true

    while running do
        local request, err = IPC.receive()

        if not request then
            -- Connection closed or error
            if err then
                io.stderr:write("IPC receive error: " .. err .. "\n")
            end
            break
        end

        -- Dispatch to handler
        local request_type = request.type
        local handler = Handlers[request_type]

        local response
        if handler then
            local ok, result = pcall(handler, request)
            if ok then
                response = result
            else
                response = {
                    type = "Error",
                    message = "Handler error: " .. tostring(result)
                }
            end
        else
            response = {
                type = "Error",
                message = "Unknown request type: " .. tostring(request_type)
            }
        end

        -- Send response
        local ok, err = pcall(IPC.send, response)
        if not ok then
            io.stderr:write("IPC send error: " .. tostring(err) .. "\n")
            break
        end

        -- Check for shutdown
        if request_type == "Shutdown" then
            running = false
        end
    end
end

-- Run the main loop
main()
