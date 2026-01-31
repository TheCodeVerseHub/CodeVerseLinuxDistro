//! IPC Protocol definitions for Lua-Rust communication
//!
//! Defines the message format for communication between the main daemon
//! and sandboxed Lua icon processes via Unix sockets.
//!
//! Supports both bincode (efficient binary) and JSON (compatible with Lua) serialization.

use serde::{Deserialize, Serialize};

use crate::lua::DrawCommand;

/// Encoding format for IPC messages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IpcEncoding {
    /// Binary encoding using bincode (more efficient)
    #[default]
    Bincode,
    /// JSON encoding (compatible with Lua IPC handler)
    Json,
}

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
///
/// Uses internally tagged JSON serialization to produce `{"type":"Handshake", ...}`
/// format that the Lua IPC handler expects.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
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
///
/// Uses internally tagged JSON serialization to produce `{"type":"HandshakeAck", ...}`
/// format that matches the Lua IPC handler's response format.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
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

/// IPC serialization helpers
#[allow(dead_code)]
impl Request {
    /// Serialize request to bytes using the specified encoding
    pub fn serialize(&self, encoding: IpcEncoding) -> Result<Vec<u8>, String> {
        match encoding {
            IpcEncoding::Bincode => {
                bincode::serialize(self).map_err(|e| e.to_string())
            }
            IpcEncoding::Json => {
                serde_json::to_vec(self).map_err(|e| e.to_string())
            }
        }
    }

    /// Deserialize request from bytes using the specified encoding
    pub fn deserialize(data: &[u8], encoding: IpcEncoding) -> Result<Self, String> {
        match encoding {
            IpcEncoding::Bincode => {
                bincode::deserialize(data).map_err(|e| e.to_string())
            }
            IpcEncoding::Json => {
                serde_json::from_slice(data).map_err(|e| e.to_string())
            }
        }
    }
}

#[allow(dead_code)]
impl Response {
    /// Serialize response to bytes using the specified encoding
    pub fn serialize(&self, encoding: IpcEncoding) -> Result<Vec<u8>, String> {
        match encoding {
            IpcEncoding::Bincode => {
                bincode::serialize(self).map_err(|e| e.to_string())
            }
            IpcEncoding::Json => {
                serde_json::to_vec(self).map_err(|e| e.to_string())
            }
        }
    }

    /// Deserialize response from bytes using the specified encoding
    pub fn deserialize(data: &[u8], encoding: IpcEncoding) -> Result<Self, String> {
        match encoding {
            IpcEncoding::Bincode => {
                bincode::deserialize(data).map_err(|e| e.to_string())
            }
            IpcEncoding::Json => {
                serde_json::from_slice(data).map_err(|e| e.to_string())
            }
        }
    }
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
        // Note: Request uses internally tagged JSON for Lua IPC compatibility,
        // which is not supported by bincode. Use JSON encoding instead.
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
        let encoded = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Request uses internally tagged JSON for Lua IPC compatibility
        let request = Request::Event {
            event: IconEvent::Click {
                button: 1,
                x: 32.0,
                y: 32.0,
            },
        };
        let encoded = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Response uses internally tagged JSON for Lua IPC compatibility
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
        let encoded = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&encoded, IpcEncoding::Json).unwrap();
        match decoded {
            Response::Render { commands } => {
                assert_eq!(commands.len(), 2);
            }
            _ => panic!("Expected Render response"),
        }
    }

    #[test]
    fn test_handshake_request_serialization() {
        // Note: Request uses internally tagged JSON for Lua IPC compatibility
        let request = Request::Handshake {
            version: PROTOCOL_VERSION,
        };
        let encoded = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Response uses internally tagged JSON for Lua IPC compatibility
        let response = Response::Error {
            message: "Script execution failed".to_string(),
        };
        let encoded = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Request uses internally tagged JSON for Lua IPC compatibility
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
        let encoded = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Response uses internally tagged JSON for Lua IPC compatibility
        let response = Response::Position {
            position: Position { x: 100, y: 200 },
        };
        let encoded = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Response uses internally tagged JSON for Lua IPC compatibility
        let response = Response::Event {
            handled: true,
            action: Some(EventAction {
                action: "spawn".to_string(),
                payload: Some("xdg-open /path/to/file".to_string()),
            }),
        };
        let encoded = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&encoded, IpcEncoding::Json).unwrap();
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
        // Note: Response uses internally tagged JSON for Lua IPC compatibility
        let response = Response::Event {
            handled: false,
            action: None,
        };
        let encoded = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&encoded, IpcEncoding::Json).unwrap();
        match decoded {
            Response::Event { handled, action } => {
                assert!(!handled);
                assert!(action.is_none());
            }
            _ => panic!("Expected Event response"),
        }
    }

    // =========================================================================
    // JSON Serialization Tests
    // =========================================================================

    #[test]
    fn test_request_json_serialization() {
        let request = Request::Handshake { version: PROTOCOL_VERSION };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Request::Handshake { version } => {
                assert_eq!(version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Handshake request"),
        }
    }

    #[test]
    fn test_response_json_serialization() {
        let response = Response::HandshakeAck {
            version: PROTOCOL_VERSION,
            success: true,
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Response::HandshakeAck { version, success } => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert!(success);
            }
            _ => panic!("Expected HandshakeAck response"),
        }
    }

    #[test]
    fn test_render_request_json_serialization() {
        let request = Request::Render {
            metadata: IconMetadata {
                path: "/test/file.txt".to_string(),
                name: "file.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                is_directory: false,
                size: Some(1024),
                width: 64,
                height: 80,
                icon_type: IconType::File,
                selected: true,
                hovered: false,
            },
            context: RenderContext {
                canvas_width: 128,
                canvas_height: 160,
                device_pixel_ratio: 1.0,
            },
        };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Request::Render { metadata, context } => {
                assert_eq!(metadata.path, "/test/file.txt");
                assert_eq!(metadata.name, "file.txt");
                assert!(metadata.selected);
                assert_eq!(context.canvas_width, 128);
            }
            _ => panic!("Expected Render request"),
        }
    }

    #[test]
    fn test_render_response_json_with_commands() {
        use crate::lua::DrawCommand;
        let response = Response::Render {
            commands: vec![
                DrawCommand::Clear { color: "#000000".to_string() },
                DrawCommand::FillRect {
                    x: 10.0, y: 20.0, w: 50.0, h: 60.0,
                    color: "#FF0000".to_string(),
                },
            ],
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Response::Render { commands } => {
                assert_eq!(commands.len(), 2);
            }
            _ => panic!("Expected Render response"),
        }
    }

    #[test]
    fn test_position_request_json_serialization() {
        let request = Request::Position {
            input: PositionInput {
                screen_width: 1920,
                screen_height: 1080,
                icon_count: 25,
                icon_index: 10,
                cell_width: Some(96),
                cell_height: Some(96),
            },
        };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Request::Position { input } => {
                assert_eq!(input.screen_width, 1920);
                assert_eq!(input.icon_index, 10);
                assert_eq!(input.cell_width, Some(96));
            }
            _ => panic!("Expected Position request"),
        }
    }

    #[test]
    fn test_position_response_json_serialization() {
        let response = Response::Position {
            position: Position { x: 200, y: 300 },
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Response::Position { position } => {
                assert_eq!(position.x, 200);
                assert_eq!(position.y, 300);
            }
            _ => panic!("Expected Position response"),
        }
    }

    #[test]
    fn test_event_request_json_serialization() {
        let request = Request::Event {
            event: IconEvent::Click {
                button: 1,
                x: 32.5,
                y: 48.0,
            },
        };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let decoded = Request::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Request::Event { event } => match event {
                IconEvent::Click { button, x, y } => {
                    assert_eq!(button, 1);
                    assert!((x - 32.5).abs() < 0.01);
                    assert!((y - 48.0).abs() < 0.01);
                }
                _ => panic!("Expected Click event"),
            },
            _ => panic!("Expected Event request"),
        }
    }

    #[test]
    fn test_error_response_json_serialization() {
        let response = Response::Error {
            message: "Script execution failed: syntax error".to_string(),
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let decoded = Response::deserialize(&json_data, IpcEncoding::Json).unwrap();
        match decoded {
            Response::Error { message } => {
                assert!(message.contains("syntax error"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    // =========================================================================
    // JSON Shape Tests (verify "type" field for Lua compatibility)
    // =========================================================================

    #[test]
    fn test_request_json_has_type_field() {
        let request = Request::Handshake { version: PROTOCOL_VERSION };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        // Verify the JSON contains a "type" field with value "Handshake"
        assert!(json_str.contains(r#""type":"Handshake""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""version":"#),
            "JSON should contain version field: {}", json_str);
    }

    #[test]
    fn test_request_shutdown_json_has_type_field() {
        let request = Request::Shutdown;
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        // Shutdown should serialize to just {"type":"Shutdown"}
        assert!(json_str.contains(r#""type":"Shutdown""#),
            "JSON should contain type field: {}", json_str);
    }

    #[test]
    fn test_response_json_has_type_field() {
        let response = Response::HandshakeAck {
            version: PROTOCOL_VERSION,
            success: true,
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        // Verify the JSON contains a "type" field with value "HandshakeAck"
        assert!(json_str.contains(r#""type":"HandshakeAck""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""version":"#),
            "JSON should contain version field: {}", json_str);
        assert!(json_str.contains(r#""success":"#),
            "JSON should contain success field: {}", json_str);
    }

    #[test]
    fn test_response_shutdown_ack_json_has_type_field() {
        let response = Response::ShutdownAck;
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        // ShutdownAck should serialize to just {"type":"ShutdownAck"}
        assert!(json_str.contains(r#""type":"ShutdownAck""#),
            "JSON should contain type field: {}", json_str);
    }

    #[test]
    fn test_render_request_json_shape() {
        let request = Request::Render {
            metadata: IconMetadata {
                path: "/test/file.txt".to_string(),
                name: "file.txt".to_string(),
                mime_type: None,
                is_directory: false,
                size: Some(1024),
                width: 64,
                height: 80,
                icon_type: IconType::File,
                selected: false,
                hovered: false,
            },
            context: RenderContext {
                canvas_width: 128,
                canvas_height: 160,
                device_pixel_ratio: 1.0,
            },
        };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        // Verify internally tagged format
        assert!(json_str.contains(r#""type":"Render""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""metadata":"#),
            "JSON should contain metadata field at top level: {}", json_str);
        assert!(json_str.contains(r#""context":"#),
            "JSON should contain context field at top level: {}", json_str);
    }

    #[test]
    fn test_position_request_json_shape() {
        let request = Request::Position {
            input: PositionInput {
                screen_width: 1920,
                screen_height: 1080,
                icon_count: 20,
                icon_index: 5,
                cell_width: Some(96),
                cell_height: Some(96),
            },
        };
        let json_data = request.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        assert!(json_str.contains(r#""type":"Position""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""input":"#),
            "JSON should contain input field at top level: {}", json_str);
    }

    #[test]
    fn test_render_response_json_shape() {
        use crate::lua::DrawCommand;
        let response = Response::Render {
            commands: vec![
                DrawCommand::Clear { color: "#000000".to_string() },
            ],
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        assert!(json_str.contains(r#""type":"Render""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""commands":"#),
            "JSON should contain commands field at top level: {}", json_str);
    }

    #[test]
    fn test_event_response_json_shape() {
        let response = Response::Event {
            handled: true,
            action: Some(EventAction {
                action: "open".to_string(),
                payload: Some("/path/to/file".to_string()),
            }),
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        assert!(json_str.contains(r#""type":"Event""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""handled":"#),
            "JSON should contain handled field at top level: {}", json_str);
        assert!(json_str.contains(r#""action":"#),
            "JSON should contain action field at top level: {}", json_str);
    }

    #[test]
    fn test_error_response_json_shape() {
        let response = Response::Error {
            message: "Test error".to_string(),
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        assert!(json_str.contains(r#""type":"Error""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""message":"#),
            "JSON should contain message field at top level: {}", json_str);
    }

    #[test]
    fn test_position_response_json_shape() {
        let response = Response::Position {
            position: Position { x: 100, y: 200 },
        };
        let json_data = response.serialize(IpcEncoding::Json).unwrap();
        let json_str = String::from_utf8(json_data).unwrap();

        assert!(json_str.contains(r#""type":"Position""#),
            "JSON should contain type field: {}", json_str);
        assert!(json_str.contains(r#""position":"#),
            "JSON should contain position field at top level: {}", json_str);
    }

    #[test]
    fn test_json_roundtrip_all_request_variants() {
        // Test that all request variants can be serialized and deserialized with JSON
        let requests = vec![
            Request::Handshake { version: 1 },
            Request::Render {
                metadata: IconMetadata {
                    path: "/test".to_string(),
                    name: "test".to_string(),
                    mime_type: None,
                    is_directory: false,
                    size: None,
                    width: 64,
                    height: 64,
                    icon_type: IconType::File,
                    selected: false,
                    hovered: false,
                },
                context: RenderContext {
                    canvas_width: 64,
                    canvas_height: 64,
                    device_pixel_ratio: 1.0,
                },
            },
            Request::Event {
                event: IconEvent::Click { button: 1, x: 0.0, y: 0.0 },
            },
            Request::Position {
                input: PositionInput {
                    screen_width: 1920,
                    screen_height: 1080,
                    icon_count: 1,
                    icon_index: 0,
                    cell_width: None,
                    cell_height: None,
                },
            },
            Request::Shutdown,
        ];

        for request in requests {
            let json_data = request.serialize(IpcEncoding::Json).unwrap();
            let decoded = Request::deserialize(&json_data, IpcEncoding::Json);
            assert!(decoded.is_ok(), "Failed to deserialize request: {:?}", request);
        }
    }

    #[test]
    fn test_json_roundtrip_all_response_variants() {
        use crate::lua::DrawCommand;

        // Test that all response variants can be serialized and deserialized with JSON
        let responses = vec![
            Response::HandshakeAck { version: 1, success: true },
            Response::Render { commands: vec![DrawCommand::Clear { color: "#000".to_string() }] },
            Response::Event { handled: true, action: None },
            Response::Position { position: Position { x: 0, y: 0 } },
            Response::Error { message: "test".to_string() },
            Response::ShutdownAck,
        ];

        for response in responses {
            let json_data = response.serialize(IpcEncoding::Json).unwrap();
            let decoded = Response::deserialize(&json_data, IpcEncoding::Json);
            assert!(decoded.is_ok(), "Failed to deserialize response: {:?}", response);
        }
    }
}
