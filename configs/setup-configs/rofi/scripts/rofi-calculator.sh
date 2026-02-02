#!/usr/bin/env bash

# // -- To launch a calculator in rofi using "rofi-calc" package -- //
rofi \
  -show calc \
  -modi calc \
  -no-show-match \
  -no-sort \
  -theme-str '
    textbox-prompt-colon {
        str: " 󰃬 Calculate:";
    }
    entry {
        placeholder: "";
    }
  '
