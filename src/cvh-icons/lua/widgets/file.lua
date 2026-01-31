-- Default file icon script
-- Demonstrates the Lua API for file icons

Icon = {
    -- Metadata
    name = "file",
    version = "1.0",
    author = "CVH Linux",

    -- Size
    width = 64,
    height = 80,

    -- State
    path = "",
    selected = false,
    hovered = false,
    extension = "",
}

-- File type colors
local type_colors = {
    -- Code files
    lua = "#51A0CF",
    py = "#3776AB",
    rs = "#DEA584",
    js = "#F7DF1E",
    ts = "#3178C6",
    go = "#00ADD8",
    c = "#A8B9CC",
    cpp = "#00599C",
    h = "#A8B9CC",

    -- Documents
    txt = "#ABB2BF",
    md = "#519ABA",
    pdf = "#FF0000",
    doc = "#2B579A",

    -- Images
    png = "#89CFF0",
    jpg = "#89CFF0",
    gif = "#89CFF0",
    svg = "#FFB13B",

    -- Archives
    zip = "#E34C26",
    tar = "#E34C26",
    gz = "#E34C26",

    -- Default
    default = "#ABB2BF",
}

function Icon:init()
    self.extension = cvh.file.extension(self.path)
    print("File icon initialized: " .. cvh.file.basename(self.path))
end

function Icon:get_color()
    local ext = string.lower(self.extension)
    return type_colors[ext] or type_colors.default
end

function Icon:render(canvas)
    local margin = 4
    local icon_size = self.width - margin * 2
    local file_color = self:get_color()
    local fold_size = 12

    -- Clear
    canvas:clear("#00000000")

    -- Selection/hover highlight
    if self.selected then
        canvas:fill_rect(0, 0, self.width, self.height, "#88C0D040")
    elseif self.hovered then
        canvas:fill_rect(0, 0, self.width, self.height, "#88C0D020")
    end

    -- Draw file shape (rectangle with folded corner)
    -- Main body
    canvas:fill_rect(margin, margin, icon_size - fold_size, icon_size, file_color)
    canvas:fill_rect(margin, margin + fold_size, icon_size, icon_size - fold_size, file_color)

    -- Fold triangle (darker shade)
    -- Note: Would need triangle drawing support for proper fold
    canvas:fill_rect(margin + icon_size - fold_size, margin, fold_size, fold_size, "#00000040")

    -- Extension label on the file
    if self.extension ~= "" then
        local ext_display = string.upper(string.sub(self.extension, 1, 4))
        -- Text would be rendered by daemon
    end

    -- Label background
    local label_y = self.width + 2
    canvas:fill_rect(0, label_y, self.width, 18, "#00000080")
end

function Icon:on_click(button, x, y)
    if button == 1 then
        self.selected = not self.selected
        return "select"
    elseif button == 3 then
        return "context_menu"
    end
    return nil
end

function Icon:on_double_click()
    -- Open with default application
    cvh.open(self.path)
    return "open"
end

function Icon:on_hover(entered)
    self.hovered = entered
end

function Icon:on_drop(items)
    -- Files can accept drops (e.g., for associations or operations)
    for i, item in ipairs(items) do
        print("Dropped " .. item .. " onto file " .. self.path)
    end
    return "drop_received"
end

-- Calculate icon position on screen
-- input: { screen_width, screen_height, icon_count, icon_index, cell_width, cell_height }
-- returns: { x, y }
function Icon:get_position(input)
    local cell_w = input.cell_width or 96
    local cell_h = input.cell_height or 96
    local margin = 20

    -- Calculate number of columns that fit on screen
    local cols = math.floor((input.screen_width - margin * 2) / cell_w)
    if cols < 1 then cols = 1 end

    -- Calculate row and column for this icon
    local col = input.icon_index % cols
    local row = math.floor(input.icon_index / cols)

    return {
        x = margin + col * cell_w,
        y = margin + row * cell_h
    }
end

return Icon
