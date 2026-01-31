-- Default folder icon script
-- Demonstrates the Lua API for file/folder icons

Icon = {
    -- Metadata
    name = "folder",
    version = "1.0",
    author = "CVH Linux",

    -- Size (set by daemon based on config)
    width = 64,
    height = 80,

    -- State
    path = "",
    selected = false,
    hovered = false,
}

-- Initialize the icon
function Icon:init()
    -- Called when the icon is created
    -- self.path is set by the daemon
    print("Folder icon initialized: " .. cvh.file.basename(self.path))
end

-- Render the icon
function Icon:render(canvas)
    local margin = 4
    local icon_size = self.width - margin * 2
    local folder_color = "#E5C07B"
    local text_color = "#ABB2BF"

    -- Clear canvas
    canvas:clear("#00000000")

    -- Draw selection highlight if selected
    if self.selected then
        canvas:fill_rect(0, 0, self.width, self.height, "#88C0D040")
    end

    -- Draw hover highlight
    if self.hovered and not self.selected then
        canvas:fill_rect(0, 0, self.width, self.height, "#88C0D020")
    end

    -- Draw folder shape
    -- Tab part
    canvas:fill_rect(margin, margin + 8, icon_size * 0.4, 8, folder_color)

    -- Main body
    canvas:fill_rect(margin, margin + 12, icon_size, icon_size - 12, folder_color)

    -- Draw label background
    local label_y = self.width + 2
    canvas:fill_rect(0, label_y, self.width, 18, "#00000080")

    -- Label text would be rendered by the daemon
end

-- Handle mouse click
function Icon:on_click(button, x, y)
    if button == 1 then
        -- Left click: toggle selection
        self.selected = not self.selected
        return "select"
    elseif button == 3 then
        -- Right click: context menu
        return "context_menu"
    end
    return nil
end

-- Handle double-click
function Icon:on_double_click()
    -- Open the folder
    cvh.open(self.path)
    return "open"
end

-- Handle hover state
function Icon:on_hover(entered)
    self.hovered = entered
end

-- Handle drag-and-drop
function Icon:on_drop(items)
    -- Move dropped items into this folder
    for i, item in ipairs(items) do
        print("Moving " .. item .. " to " .. self.path)
        -- cvh.move(item, self.path)
    end
    return "move"
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

-- Return the Icon table
return Icon
