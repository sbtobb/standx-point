/*
[INPUT]:  Symbol identifiers and query parameters
[OUTPUT]: Market data (symbol info, prices, depth, klines)
[POS]:    HTTP layer - public market data endpoints (no auth required)
[UPDATE]: When adding new public endpoints or changing response format
*/

use crate::http::{Result, StandxClient};
use crate::types::{DepthBook, KlineData, SymbolInfo, SymbolPrice};

impl StandxClient {
    /// Query symbol information
    ///
    /// GET /api/query_symbol_info?symbol={symbol}
    pub async fn query_symbol_info(&self, symbol: &str) -> Result<Vec<SymbolInfo>> {
        let _endpoint = format!("/api/query_symbol_info?symbol={}", symbol);
        todo!("Implement HTTP GET for query_symbol_info")
    }

    /// Query symbol price data
    ///
    /// GET /api/query_symbol_price?symbol={symbol}
    pub async fn query_symbol_price(&self, symbol: &str) -> Result<SymbolPrice> {
        let _endpoint = format!("/api/query_symbol_price?symbol={}", symbol);
        todo!("Implement HTTP GET for query_symbol_price")
    }

    /// Query order book depth
    ///
    /// GET /api/query_depth_book?symbol={symbol}
    pub async fn query_depth_book(&self, symbol: &str) -> Result<DepthBook> {
        let _endpoint = format!("/api/query_depth_book?symbol={}", symbol);
        todo!("Implement HTTP GET for query_depth_book")
    }

    /// Get kline/candlestick history
    ///
    /// GET /api/kline/history?symbol={symbol}&from={from}&to={to}&resolution={resolution}
    pub async fn get_kline_history(
        &self,
        symbol: &str,
        from: u64,
        to: u64,
        resolution: &str,
    ) -> Result<KlineData> {
        let _endpoint = format!(
            "/api/kline/history?symbol={}&from={}&to={}&resolution={}",
            symbol, from, to, resolution
        );
        todo!("Implement HTTP GET for get_kline_history")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[ignore = "Requires HTTP client implementation"]
    async fn test_query_symbol_info() {
        let _server = MockServer::start().await;
        let _mock = Mock::given(method("GET"))
            .and(path("/api/query_symbol_info"))
            .and(query_param("symbol", "BTCUSDT"))
            .respond_with(ResponseTemplate::new(200));

        todo!("Implement HTTP client test for query_symbol_info");
    }
}
