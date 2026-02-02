#!/usr/bin/env bash

notify-send "Getting list of available Wi-Fi networks..."

# Get a list of available wifi connections and morph it into a nice-looking list
wifi_list=$(nmcli --fields "SECURITY,SSID" device wifi list | sed 1d | sed 's/  */ /g' | sed -E "s/WPA*.?\S/ /g" | sed "s/^--/ /g" | sed "s/  //g" | sed "/--/d")

# Get currently connected SSID
current_ssid=$(nmcli -t -f ACTIVE,SSID dev wifi | awk -F: '$1=="yes"{print $2}')

# Add checkmark to currently connected network (only if there is one)
if [ -n "$current_ssid" ]; then
    wifi_list=$(awk -v ssid="$current_ssid" '
    $0 ~ ssid && $0 !~ /✓$/ {
        print $0 " ✓"
        next
    }
    { print }
    ' <<< "$wifi_list")
fi

connected=$(nmcli -fields WIFI g)
if [[ "$connected" =~ "enabled" ]]; then
	toggle="󰖪  Disable Wi-Fi"
elif [[ "$connected" =~ "disabled" ]]; then
	toggle="󰖩  Enable Wi-Fi"
fi

# Use rofi to select wifi network
chosen_network=$(printf "%s\n%s\n" "$toggle" "$wifi_list" \
  | uniq -u \
  | rofi -dmenu -i \
    -selected-row 1 \
    -p "Wi-Fi SSID:" \
    -theme-str '
      textbox-prompt-colon { str: " Network:"; }
      entry { placeholder: ""; }
      ')

[ -z "$chosen_network" ] && exit

# Get name of connection
chosen_id="$chosen_network"
chosen_id="${chosen_id# }"
chosen_id="${chosen_id# }"
chosen_id="${chosen_id% ✓}"

if [ "$chosen_network" = "" ]; then
	exit
elif [ "$chosen_network" = "󰖩  Enable Wi-Fi" ]; then
	nmcli radio wifi on
elif [ "$chosen_network" = "󰖪  Disable Wi-Fi" ]; then
	nmcli radio wifi off
else
	# Message to show when connection is activated successfully
  	success_message="You are now connected to the Wi-Fi network \"$chosen_id\"."
	# Get saved connections
	saved_connections=$(nmcli -g NAME connection)
	if [[ $(echo "$saved_connections" | grep -w "$chosen_id") = "$chosen_id" ]]; then
		nmcli connection up id "$chosen_id" | grep "successfully" && notify-send "Connection Established" "$success_message"
    else
      if [[ "$chosen_network" =~ "" ]]; then
          wifi_password=$(rofi -dmenu -p "Password:" \
            -theme-str '
            textbox-prompt-colon { str: "Password:"; }
            entry { placeholder: ""; }
          ')
          nmcli device wifi connect "$chosen_id" password "$wifi_password" \
            | grep "successfully" && notify-send "Connection Established" "$success_message"
      else
          nmcli device wifi connect "$chosen_id" \
            | grep "successfully" && notify-send "Connection Established" "$success_message"
      fi
    fi
fi
