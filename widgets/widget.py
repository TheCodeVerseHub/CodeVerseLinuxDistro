"""
An App to show the current time with theme support.
"""

import json
import sys
from datetime import datetime
from pathlib import Path

from textual.app import App, ComposeResult
from textual.widgets import Digits


def load_themes():
    """Load themes from themes.json file."""
    themes_file = Path(__file__).parent / "themes.json"
    with open(themes_file) as f:
        data = json.load(f)
    return data["themes"]


def get_theme_css(theme_name: str) -> str:
    """Generate CSS for a specific theme."""
    themes = load_themes()
    if theme_name not in themes:
        available = ", ".join(themes.keys())
        raise ValueError(f"Theme '{theme_name}' not found. Available: {available}")
    
    theme = themes[theme_name]
    return f"""
    Screen {{ 
        align: center middle; 
        background: {theme['background']}; 
    }}
    Digits {{ 
        width: auto; 
        color: {theme['foreground']}; 
    }}
    """


class ClockApp(App):
    CSS = """
    Screen { align: center middle; background: #282828; }
    Digits { width: auto; color: #fbf1c7; }
    """
    
    def __init__(self, theme_name: str = "gruvbox", **kwargs):
        self.theme_name = theme_name
        # Override CSS with theme
        ClockApp.CSS = get_theme_css(theme_name)
        super().__init__(**kwargs)
    
    def compose(self) -> ComposeResult:
        yield Digits("")
    
    def on_ready(self) -> None:
        self.update_clock()
        self.set_interval(1, self.update_clock)
    
    def update_clock(self) -> None:
        clock = datetime.now().time()
        self.query_one(Digits).update(f"{clock:%T}")


if __name__ == "__main__":
    # Parse command line arguments for theme selection
    theme = "gruvbox"
    if len(sys.argv) > 1:
        theme = sys.argv[1]
    
    try:
        app = ClockApp(theme_name=theme)
        app.run()
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)
