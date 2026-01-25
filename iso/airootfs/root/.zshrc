# CVH Linux Live Environment

# Basic prompt
PS1='%F{cyan}[CVH Live]%f %F{green}%n@%m%f:%F{blue}%~%f %# '

# Aliases
alias ls='ls --color=auto'
alias ll='ls -la'
alias install='cvh-install'

# Welcome message
echo ""
echo "  Welcome to CVH Linux Live Environment!"
echo ""
echo "  To install CVH Linux, run: cvh-install"
echo "  To start Niri compositor, run: niri-session"
echo ""
