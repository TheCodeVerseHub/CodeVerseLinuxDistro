//! IPC Protocol definitions for Lua-Rust communication
//!
//! Defines the message format for communication between the main daemon
//! and sandboxed Lua icon processes via Unix sockets with bincode serialization.

use serde::{Deserialize, Serialize};

use crate::lua::DrawCommand;

/// Protocol version for compatibility checking
#[allow(dead_code)]
pub const PROTOCOL_VERSION: u32 = 1;

/// Position of an icon on screen
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Type of icon being rendered
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IconType {
    /// Regular file icon
    File,
    /// Directory/folder icon
    Directory,
    /// Application/executable icon
    Application,
    /// Symlink icon
    Symlink,
    /// Custom/script-defined icon
    Custom(String),
}

/// Metadata about an icon's associated file/folder
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IconMetadata {
    /// Path to the file or folder
    pub path: String,
    /// Display name (file/folder name)
    pub name: String,
    /// MIME type if known
    pub mime_type: Option<String>,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (for files)
    pub size: Option<u64>,
    /// Icon width in pixels
    pub width: u32,
    /// Icon height in pixels
    pub height: u32,
    /// Type of icon
    pub icon_type: IconType,
    /// Whether the icon is currently selected
    pub selected: bool,
    /// Whether the icon is currently hovered
    pub hovered: bool,
}

/// Events that can be sent to an icon script
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IconEvent {
    /// Mouse click event
    Click {
        button: u32,
        x: f64,
        y: f64,
    },
    /// Mouse hover enter
    HoverEnter,
    /// Mouse hover exit
    HoverExit,
    /// File(s) dropped on the icon
    Drop {
        paths: Vec<String>,
    },
    /// Icon selected
    Selected,
    /// Icon deselected
    Deselected,
}

/// Render context providing canvas dimensions and other rendering info
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RenderContext {
    /// Canvas width in pixels
    pub canvas_width: u32,
    /// Canvas height in pixels
    pub canvas_height: u32,
    /// Device pixel ratio for HiDPI support
    pub device_pixel_ratio: f32,
}

/// Position computation inputs sent to Lua for calculating icon position
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PositionInput {
    /// Screen width in pixels
    pub screen_width: u32,
    /// Screen height in pixels
    pub screen_height: u32,
    /// Total number of icons being positioned
    pub icon_count: u32,
    /// Index of this icon in the layout
    pub icon_index: u32,
    /// Grid cell width (if using grid layout)
    pub cell_width: Option<u32>,
    /// Grid cell height (if using grid layout)
    pub cell_height: Option<u32>,
}

/// Request messages sent from Rust to Lua process
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    /// Handshake to verify protocol version
    Handshake {
        version: u32,
    },
    /// Request to render the icon
    Render {
        metadata: IconMetadata,
        /// Canvas dimensions and render context
        context: RenderContext,
    },
    /// Send an event to the icon script
    Event {
        event: IconEvent,
    },
    /// Request position computation from Lua
    Position {
        /// Inputs for position calculation
        input: PositionInput,
    },
    /// Request to shutdown the Lua process
    Shutdown,
}

/// Action to perform in response to an event
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventAction {
    /// Action type (e.g., "open", "spawn", "notify", "none")
    pub action: String,
    /// Payload for the action (e.g., path to open, command to spawn)
    pub payload: Option<String>,
}

/// Response messages sent from Lua process to Rust
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    /// Handshake acknowledgement
    HandshakeAck {
        version: u32,
        success: bool,
    },
    /// Render result with draw commands
    Render {
        commands: Vec<DrawCommand>,
    },
    /// Event handling result with action to perform
    Event {
        /// Whether the event was handled
        handled: bool,
        /// Action to perform (if any)
        action: Option<EventAction>,
    },
    /// Position computation result
    Position {
        /// Computed position for the icon
        position: Position,
    },
    /// Error response
    Error {
        message: String,
    },
    /// Shutdown acknowledgement
    ShutdownAck,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_is_set() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    #[test]
    fn test_position_serialization() {
        let pos = Position { x: 100, y: 200 };
        let encoded = bincode::serialize(&pos).unwrap();
        let decoded: Position = bincode::deserialize(&encoded).unwrap();
        assert_eq!(pos, decoded);
    }

    #[test]
    fn test_icon_metadata_serialization() {
        let metadata = IconMetadata {
            path: "/home/user/Desktop/file.txt".to_string(),
            name: "file.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            is_directory: false,
            size: Some(1024),
            width: 64,
            height: 64,
            icon_type: IconType::File,
            selected: false,
            hovered: true,
        };
        let encoded = bincode::serialize(&metadata).unwrap();
        let decoded: IconMetadata = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.path, metadata.path);
        assert_eq!(decoded.name, metadata.name);
        assert_eq!(decoded.icon_type, IconType::File);
        assert!(!decoded.selected);
        assert!(decoded.hovered);
    }

    #[test]
    fn test_icon_type_serialization() {
        let icon_type = IconType::Custom("my_custom_icon".to_string());
        let encoded = bincode::serialize(&icon_type).unwrap();
        let decoded: IconType = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, icon_type);
    }

    #[test]
    fn test_request_render_serialization() {
        let request = Request::Render {
            metadata: IconMetadata {
                path: "/test".to_string(),
                name: "test".to_string(),
                mime_type: None,
                is_directory: true,
                size: None,
                width: 64,
                height: 64,
                icon_type: IconType::Directory,
                selected: true,
                hovered: false,
            },
            context: RenderContext {
                canvas_width: 128,
                canvas_height: 128,
                device_pixel_ratio: 2.0,
            },
        };
        let encoded = bincode::serialize(&request).unwrap();
        let decoded: Request = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Request::Render { metadata, context } => {
                assert_eq!(metadata.path, "/test");
                assert!(metadata.is_directory);
                assert_eq!(metadata.icon_type, IconType::Directory);
                assert!(metadata.selected);
                assert!(!metadata.hovered);
                assert_eq!(context.canvas_width, 128);
                assert_eq!(context.canvas_height, 128);
                assert!((context.device_pixel_ratio - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected Render request"),
        }
    }

    #[test]
    fn test_render_context_serialization() {
        let context = RenderContext {
            canvas_width: 256,
            canvas_height: 256,
            device_pixel_ratio: 1.5,
        };
        let encoded = bincode::serialize(&context).unwrap();
        let decoded: RenderContext = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, context);
    }

    #[test]
    fn test_request_event_serialization() {
        let request = Request::Event {
            event: IconEvent::Click {
                button: 1,
                x: 32.0,
                y: 32.0,
            },
        };
        let encoded = bincode::serialize(&request).unwrap();
        let decoded: Request = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Request::Event { event } => match event {
                IconEvent::Click { button, x, y } => {
                    assert_eq!(button, 1);
                    assert!((x - 32.0).abs() < 0.001);
                    assert!((y - 32.0).abs() < 0.001);
                }
                _ => panic!("Expected Click event"),
            },
            _ => panic!("Expected Event request"),
        }
    }

    #[test]
    fn test_response_render_serialization() {
        let response = Response::Render {
            commands: vec![
                DrawCommand::Clear {
                    color: "#FFFFFF".to_string(),
                },
                DrawCommand::FillRect {
                    x: 0.0,
                    y: 0.0,
                    w: 64.0,
                    h: 64.0,
                    color: "#0000FF".to_string(),
                },
            ],
        };
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: Response = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Response::Render { commands } => {
                assert_eq!(commands.len(), 2);
            }
            _ => panic!("Expected Render response"),
        }
    }

    #[test]
    fn test_handshake_request_serialization() {
        let request = Request::Handshake {
            version: PROTOCOL_VERSION,
        };
        let encoded = bincode::serialize(&request).unwrap();
        let decoded: Request = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Request::Handshake { version } => {
                assert_eq!(version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Handshake request"),
        }
    }

    #[test]
    fn test_icon_event_drop_serialization() {
        let event = IconEvent::Drop {
            paths: vec!["/path/to/file1".to_string(), "/path/to/file2".to_string()],
        };
        let encoded = bincode::serialize(&event).unwrap();
        let decoded: IconEvent = bincode::deserialize(&encoded).unwrap();
        match decoded {
            IconEvent::Drop { paths } => {
                assert_eq!(paths.len(), 2);
                assert_eq!(paths[0], "/path/to/file1");
            }
            _ => panic!("Expected Drop event"),
        }
    }

    #[test]
    fn test_response_error_serialization() {
        let response = Response::Error {
            message: "Script execution failed".to_string(),
        };
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: Response = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Response::Error { message } => {
                assert_eq!(message, "Script execution failed");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_position_input_serialization() {
        let input = PositionInput {
            screen_width: 1920,
            screen_height: 1080,
            icon_count: 10,
            icon_index: 5,
            cell_width: Some(96),
            cell_height: Some(96),
        };
        let encoded = bincode::serialize(&input).unwrap();
        let decoded: PositionInput = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_request_position_serialization() {
        let request = Request::Position {
            input: PositionInput {
                screen_width: 1920,
                screen_height: 1080,
                icon_count: 10,
                icon_index: 3,
                cell_width: None,
                cell_height: None,
            },
        };
        let encoded = bincode::serialize(&request).unwrap();
        let decoded: Request = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Request::Position { input } => {
                assert_eq!(input.screen_width, 1920);
                assert_eq!(input.screen_height, 1080);
                assert_eq!(input.icon_count, 10);
                assert_eq!(input.icon_index, 3);
                assert!(input.cell_width.is_none());
            }
            _ => panic!("Expected Position request"),
        }
    }

    #[test]
    fn test_response_position_serialization() {
        let response = Response::Position {
            position: Position { x: 100, y: 200 },
        };
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: Response = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Response::Position { position } => {
                assert_eq!(position.x, 100);
                assert_eq!(position.y, 200);
            }
            _ => panic!("Expected Position response"),
        }
    }

    #[test]
    fn test_event_action_serialization() {
        let action = EventAction {
            action: "open".to_string(),
            payload: Some("/home/user/Documents".to_string()),
        };
        let encoded = bincode::serialize(&action).unwrap();
        let decoded: EventAction = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_response_event_with_action_serialization() {
        let response = Response::Event {
            handled: true,
            action: Some(EventAction {
                action: "spawn".to_string(),
                payload: Some("xdg-open /path/to/file".to_string()),
            }),
        };
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: Response = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Response::Event { handled, action } => {
                assert!(handled);
                let action = action.unwrap();
                assert_eq!(action.action, "spawn");
                assert_eq!(action.payload.unwrap(), "xdg-open /path/to/file");
            }
            _ => panic!("Expected Event response"),
        }
    }

    #[test]
    fn test_response_event_no_action_serialization() {
        let response = Response::Event {
            handled: false,
            action: None,
        };
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: Response = bincode::deserialize(&encoded).unwrap();
        match decoded {
            Response::Event { handled, action } => {
                assert!(!handled);
                assert!(action.is_none());
            }
            _ => panic!("Expected Event response"),
        }
    }
}
