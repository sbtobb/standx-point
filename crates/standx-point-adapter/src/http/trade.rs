/*
[INPUT]:  Order requests with body signature headers
[OUTPUT]: Order responses and confirmation
[POS]:    HTTP layer - trading endpoints (require auth + body signature)
[UPDATE]: When adding new trading endpoints or changing order flow
*/

use crate::http::{Result, StandxClient};
use crate::types::{
    CancelOrderRequest, CancelOrderResponse, ChangeLeverageRequest, ChangeLeverageResponse,
    NewOrderRequest, NewOrderResponse,
};

impl StandxClient {
    /// Create a new order
    ///
    /// POST /api/new_order
    /// Requires: Authorization header + body signature headers
    pub async fn new_order(&self, req: NewOrderRequest) -> Result<NewOrderResponse> {
        let payload = serde_json::to_string(&req)?;
        let timestamp = crate::http::RequestSigner::timestamp_millis();

        let (builder, _signature) =
            self.trading_post_with_jwt_and_signature("/api/new_order", &payload, timestamp)?;

        let builder = builder.body(payload);
        self.send_json(builder).await
    }

    /// Cancel an existing order
    ///
    /// POST /api/cancel_order
    /// Requires: Authorization header + body signature headers
    pub async fn cancel_order(&self, req: CancelOrderRequest) -> Result<CancelOrderResponse> {
        let payload = serde_json::to_string(&req)?;
        let timestamp = crate::http::RequestSigner::timestamp_millis();

        let (builder, _signature) =
            self.trading_post_with_jwt_and_signature("/api/cancel_order", &payload, timestamp)?;

        let builder = builder.body(payload);
        self.send_json(builder).await
    }

    /// Change leverage for a symbol
    ///
    /// POST /api/change_leverage
    /// Requires: Authorization header + body signature headers
    pub async fn change_leverage(
        &self,
        symbol: &str,
        leverage: u32,
    ) -> Result<ChangeLeverageResponse> {
        let req = ChangeLeverageRequest {
            symbol: symbol.to_string(),
            leverage,
        };
        let payload = serde_json::to_string(&req)?;
        let timestamp = crate::http::RequestSigner::timestamp_millis();

        let (builder, _signature) =
            self.trading_post_with_jwt_and_signature("/api/change_leverage", &payload, timestamp)?;

        let builder = builder.body(payload);
        self.send_json(builder).await
    }
}
