# CVH Fuzzy Zsh Integration
if command -v cvh-fuzzy &> /dev/null; then
    cvh-fuzzy-file-widget() {
        local selected=$(cvh-fuzzy --mode files)
        LBUFFER="${LBUFFER}${selected}"
        zle redisplay
    }
    zle -N cvh-fuzzy-file-widget
    bindkey '^T' cvh-fuzzy-file-widget

    cvh-fuzzy-history-widget() {
        local selected=$(fc -rl 1 | cvh-fuzzy --mode stdin | sed 's/^ *[0-9]* *//')
        LBUFFER="$selected"
        zle redisplay
    }
    zle -N cvh-fuzzy-history-widget
    bindkey '^R' cvh-fuzzy-history-widget
fi
