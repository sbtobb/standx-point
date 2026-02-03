/*
[INPUT]:  Query parameters and JWT authentication
[OUTPUT]: User account data (orders, positions, balances)
[POS]:    HTTP layer - user data endpoints (require JWT auth)
[UPDATE]: When adding new user endpoints or changing query parameters
*/

// ### User Endpoints

use crate::http::{Result, StandxClient};
use crate::types::{Balance, OrderStatus, PaginatedOrders, Position};
use reqwest::Method;

impl StandxClient {
    /// Query user orders with optional filters
    ///
    /// GET /api/query_orders?symbol={symbol}&status={status}&limit={limit}
    pub async fn query_orders(
        &self,
        symbol: Option<&str>,
        status: Option<OrderStatus>,
        limit: Option<u32>,
    ) -> Result<PaginatedOrders> {
        let mut params = Vec::new();
        if let Some(s) = symbol {
            params.push(format!("symbol={}", s));
        }
        if let Some(st) = status {
            let status_value = serde_json::to_string(&st)
                .unwrap()
                .trim_matches('"')
                .to_string();
            params.push(format!("status={}", status_value));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }

        let endpoint = if params.is_empty() {
            "/api/query_orders".to_string()
        } else {
            format!("/api/query_orders?{}", params.join("&"))
        };

        let builder = self.trading_request_with_jwt(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }

    /// Query open orders for a symbol
    ///
    /// GET /api/query_open_orders?symbol={symbol}
    pub async fn query_open_orders(&self, symbol: Option<&str>) -> Result<PaginatedOrders> {
        let endpoint = if let Some(s) = symbol {
            format!("/api/query_open_orders?symbol={}", s)
        } else {
            "/api/query_open_orders".to_string()
        };

        let builder = self.trading_request_with_jwt(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }

    /// Query user positions
    ///
    /// GET /api/query_positions?symbol={symbol}
    pub async fn query_positions(&self, symbol: Option<&str>) -> Result<Vec<Position>> {
        let endpoint = if let Some(s) = symbol {
            format!("/api/query_positions?symbol={}", s)
        } else {
            "/api/query_positions".to_string()
        };

        let builder = self.trading_request_with_jwt(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }

    /// Query user balance
    ///
    /// GET /api/query_balance
    pub async fn query_balance(&self) -> Result<Balance> {
        let endpoint = "/api/query_balance";
        let builder = self.trading_request_with_jwt(Method::GET, endpoint)?;
        self.send_json(builder).await
    }
}

#[cfg(test)]
mod tests {
    // Tests will be implemented once HTTP client methods are ready

    #[tokio::test]
    #[ignore = "Requires HTTP client implementation"]
    async fn test_query_balance() {
        // Test will be implemented
    }
}
