Origin: https://antigravity.google/schemas/v1/settings.json
Retrieved: 2026-11-25

# Antigravity CLI `settings.json` Schema Excerpts (Status Line & Adjacent Display Keys)

Verbatim line excerpts from the published v1 JSON Schema (`$id: https://antigravity.google/schemas/v1/settings.json`).

## Top-level `statusLine` property (lines 199-211)

```json
    "statusLine": {
      "anyOf": [
        {
          "$ref": "#/$defs/statusLine"
        },
        {
          "type": "boolean",
          "deprecated": true,
          "description": "Not recommended: a boolean is read as an object without a command, so no script runs."
        }
      ],
      "description": "Runs your own script to draw the status line below the prompt box."
    },
```

## `$defs.statusLine` definition (lines 303-336)

```json
    "statusLine": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "type": {
          "type": "string",
          "enum": [
            "command"
          ],
          "default": "command",
          "description": "Optional. \"command\" is the only supported type."
        },
        "command": {
          "type": "string",
          "description": "Shell command to run, through sh -c (cmd /c on Windows). It receives the session state as JSON on stdin, and its output becomes the status line. Multi-line output and ANSI colors are supported, and a leading ~ expands to your home directory."
        },
        "padding": {
          "type": "integer",
          "minimum": 0,
          "default": 0,
          "description": "Number of blank lines above the status line."
        },
        "enabled": {
          "type": "boolean",
          "default": true,
          "description": "Set to false to stop running command and show the built-in status line instead. The command stays in settings.json."
        },
        "stack_with_default": {
          "type": "boolean",
          "default": false,
          "description": "When true, shows your status line below the built-in one instead of replacing it."
        }
      }
    },
```

## Adjacent display keys: `altScreenMode` (lines 32-41), `clearScrollbackOnResize` (lines 52-56), `colorScheme` (lines 57-71), `hideActiveItems` (lines 118-122)

```json
    "altScreenMode": {
      "type": "string",
      "enum": [
        "default",
        "always",
        "never"
      ],
      "default": "default",
      "description": "How the TUI uses the terminal: \"default\" renders the conversation inline and opens full-screen views in the alternate screen, \"always\" runs the whole TUI in the alternate screen, and \"never\" never uses the alternate screen. /config: Rendering Mode (\"never\" is available only here)."
    },
    "clearScrollbackOnResize": {
      "type": "boolean",
      "default": true,
      "description": "In inline rendering, clears the terminal scrollback when the CLI redraws the conversation, such as after you resize the window."
    },
    "colorScheme": {
      "type": "string",
      "enum": [
        "terminal",
        "light",
        "solarized light",
        "colorblind-friendly light",
        "dark",
        "solarized dark",
        "colorblind-friendly dark",
        "tokyo night"
      ],
      "default": "terminal",
      "description": "Color theme. \"terminal\" uses your terminal's colors. /config: Color Scheme."
    },
    "hideActiveItems": {
      "type": "boolean",
      "default": false,
      "description": "Hides the list of active subagents and background tasks below the prompt box. /config: Show Active Tasks (inverted)."
    },
```
