/*
[INPUT]:  WebSocket test scenarios
[OUTPUT]: Test results for WebSocket client
[POS]:    Integration tests - WebSocket
[UPDATE]: When WebSocket client changes
*/

use standx_point_adapter::StandxWebSocket;

#[test]
fn test_websocket_creation() {
    let mut ws = StandxWebSocket::new();
    assert!(ws.take_receiver().is_some());
}

#[test]
fn test_websocket_default() {
    let mut ws: StandxWebSocket = Default::default();
    assert!(ws.take_receiver().is_some());
}

#[test]
fn test_websocket_receiver_take_once() {
    let mut ws = StandxWebSocket::new();
    assert!(ws.take_receiver().is_some());
    assert!(ws.take_receiver().is_none());
}
