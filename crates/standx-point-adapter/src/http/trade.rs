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
        let _ = req;
        todo!("Implement HTTP POST for new_order with body signature")
    }

    /// Cancel an existing order
    ///
    /// POST /api/cancel_order
    /// Requires: Authorization header + body signature headers
    pub async fn cancel_order(&self, req: CancelOrderRequest) -> Result<CancelOrderResponse> {
        let _ = req;
        todo!("Implement HTTP POST for cancel_order with body signature")
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
        let _ = req;
        todo!("Implement HTTP POST for change_leverage with body signature")
    }
}
