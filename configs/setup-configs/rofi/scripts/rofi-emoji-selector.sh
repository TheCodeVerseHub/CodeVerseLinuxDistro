#!/usr/bin/env bash

# // -- To launch a emoji-selector in rofi using "rofi-emoji" package -- //
rofi \
  -modi emoji \
  -show emoji \
  -theme-str '
    textbox-prompt-colon {
        str: " 󰞅 Emoji:";
    }
    entry {
        placeholder: "";
    }
  '
