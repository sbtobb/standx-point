## Architecture
- **Position**: WebSocket client for real-time data streams
- **Logic**: Connect -> Subscribe -> Receive Messages -> Forward to channels
- **Constraints**: No business logic, just message forwarding

## Members
- `client.rs`: WebSocket connection and subscription management
- `message.rs`: Message type definitions

## Conventions
- Use tokio-tungstenite for WebSocket
- Use mpsc channels for message passing
- Ping/Pong handled automatically by library
