# CVH Linux Zsh Configuration

# Path to Oh My Zsh installation
export ZSH="$HOME/.oh-my-zsh"

# Theme
ZSH_THEME="robbyrussell"

# Plugins
plugins=(
    git
    zsh-syntax-highlighting
    zsh-autosuggestions
    sudo
    history
    colored-man-pages
)

# Load Oh My Zsh (if installed)
[[ -f $ZSH/oh-my-zsh.sh ]] && source $ZSH/oh-my-zsh.sh

# =============================================================================
# ENVIRONMENT VARIABLES
# =============================================================================
export EDITOR="nano"
export VISUAL="$EDITOR"
export PAGER="less"
export LANG="en_US.UTF-8"
export LC_ALL="en_US.UTF-8"

# XDG Base Directories
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_STATE_HOME="$HOME/.local/state"

# Wayland
export QT_QPA_PLATFORM="wayland"
export SDL_VIDEODRIVER="wayland"
export MOZ_ENABLE_WAYLAND="1"
export GDK_BACKEND="wayland"

# =============================================================================
# CVH FUZZY FINDER INTEGRATION
# =============================================================================
export CVH_FUZZY_DEFAULT_OPTS="--height 40 --border"

if command -v cvh-fuzzy &> /dev/null; then
    # CTRL-T: Find and insert file path
    cvh-fuzzy-file-widget() {
        local selected
        selected=$(cvh-fuzzy --mode files --print0 2>/dev/null | tr '\0' ' ')
        if [[ -n "$selected" ]]; then
            LBUFFER="${LBUFFER}${selected}"
        fi
        zle redisplay
    }
    zle -N cvh-fuzzy-file-widget
    bindkey '^T' cvh-fuzzy-file-widget

    # CTRL-R: Search command history
    cvh-fuzzy-history-widget() {
        local selected
        selected=$(fc -rl 1 | cvh-fuzzy --mode history 2>/dev/null | sed 's/^ *[0-9]* *//')
        if [[ -n "$selected" ]]; then
            LBUFFER="$selected"
        fi
        zle redisplay
    }
    zle -N cvh-fuzzy-history-widget
    bindkey '^R' cvh-fuzzy-history-widget

    # ALT-C: cd into selected directory
    cvh-fuzzy-cd-widget() {
        local dir
        dir=$(cvh-fuzzy --mode dirs 2>/dev/null)
        if [[ -n "$dir" && -d "$dir" ]]; then
            cd "$dir"
            zle accept-line
        fi
        zle redisplay
    }
    zle -N cvh-fuzzy-cd-widget
    bindkey '\ec' cvh-fuzzy-cd-widget
fi

# =============================================================================
# ALIASES
# =============================================================================
alias ls='ls --color=auto'
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'
alias grep='grep --color=auto'
alias df='df -h'
alias du='du -h'
alias free='free -h'
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

# CVH Linux aliases
alias ff='cvh-fuzzy'
alias ffa='cvh-fuzzy --mode apps'
alias fff='cvh-fuzzy --mode files'
alias edit='$EDITOR'

# =============================================================================
# HISTORY CONFIGURATION
# =============================================================================
HISTFILE="$HOME/.zsh_history"
HISTSIZE=10000
SAVEHIST=10000
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE
setopt SHARE_HISTORY
setopt APPEND_HISTORY
setopt HIST_REDUCE_BLANKS

# =============================================================================
# DIRECTORY NAVIGATION
# =============================================================================
setopt AUTO_CD
setopt AUTO_PUSHD
setopt PUSHD_IGNORE_DUPS
setopt PUSHD_SILENT

# =============================================================================
# COMPLETION
# =============================================================================
autoload -Uz compinit && compinit
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'
zstyle ':completion:*' list-colors "${(s.:.)LS_COLORS}"

# =============================================================================
# AUTO-START NIRI
# =============================================================================
# Start Niri if on tty1 and not already in a Wayland session
if [[ -z "$WAYLAND_DISPLAY" ]] && [[ "$XDG_VTNR" -eq 1 ]]; then
    exec niri-session
fi
