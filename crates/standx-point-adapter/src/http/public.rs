/*
[INPUT]:  Symbol identifiers and query parameters
[OUTPUT]: Market data (symbol info, prices, depth, klines)
[POS]:    HTTP layer - public market data endpoints (no auth required)
[UPDATE]: When adding new public endpoints or changing response format
[UPDATE]: 2026-02-07 Added public endpoint GET implementations and tests
*/

use crate::http::{Result, StandxClient};
use crate::types::{DepthBook, KlineData, SymbolInfo, SymbolPrice};
use reqwest::Method;

impl StandxClient {
    /// Query symbol information
    ///
    /// GET /api/query_symbol_info?symbol={symbol}
    pub async fn query_symbol_info(&self, symbol: &str) -> Result<Vec<SymbolInfo>> {
        let endpoint = format!("/api/query_symbol_info?symbol={}", symbol);
        let builder = self.trading_request(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }

    /// Query symbol price data
    ///
    /// GET /api/query_symbol_price?symbol={symbol}
    pub async fn query_symbol_price(&self, symbol: &str) -> Result<SymbolPrice> {
        let endpoint = format!("/api/query_symbol_price?symbol={}", symbol);
        let builder = self.trading_request(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }

    /// Query order book depth
    ///
    /// GET /api/query_depth_book?symbol={symbol}
    pub async fn query_depth_book(&self, symbol: &str) -> Result<DepthBook> {
        let endpoint = format!("/api/query_depth_book?symbol={}", symbol);
        let builder = self.trading_request(Method::GET, &endpoint)?;
        self.send_json(builder).await
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
        let endpoint = format!(
            "/api/kline/history?symbol={}&from={}&to={}&resolution={}",
            symbol, from, to, resolution
        );
        let builder = self.trading_request(Method::GET, &endpoint)?;
        self.send_json(builder).await
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{ClientConfig, StandxClient};
    use crate::types::{DepthBook, DepthLevel, KlineData, SymbolInfo, SymbolPrice};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_query_symbol_info() {
        let server = MockServer::start().await;
        let mock_response = r#"[
            {
                "base_asset": "BTC",
                "base_decimals": 8,
                "created_at": "2024-01-01T00:00:00Z",
                "def_leverage": "10",
                "depth_ticks": "0.1",
                "enabled": true,
                "maker_fee": "0.0002",
                "max_leverage": "50",
                "max_open_orders": "200",
                "max_order_qty": "1000",
                "max_position_size": "1000",
                "min_order_qty": "0.001",
                "price_cap_ratio": "0.1",
                "price_floor_ratio": "0.1",
                "price_tick_decimals": 2,
                "qty_tick_decimals": 3,
                "quote_asset": "USDT",
                "quote_decimals": 6,
                "symbol": "BTCUSDT",
                "taker_fee": "0.0006",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        ]"#;

        let _mock = Mock::given(method("GET"))
            .and(path("/api/query_symbol_info"))
            .and(query_param("symbol", "BTCUSDT"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_raw(mock_response, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = StandxClient::with_config_and_base_urls(
            ClientConfig::default(),
            &server.uri(),
            &server.uri(),
        )
        .expect("client init");

        let response = client
            .query_symbol_info("BTCUSDT")
            .await
            .expect("query_symbol_info failed");

        let expected = vec![SymbolInfo {
            base_asset: "BTC".to_string(),
            base_decimals: 8,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            def_leverage: "10".parse().expect("def_leverage"),
            depth_ticks: "0.1".to_string(),
            enabled: true,
            maker_fee: "0.0002".parse().expect("maker_fee"),
            max_leverage: "50".parse().expect("max_leverage"),
            max_open_orders: "200".parse().expect("max_open_orders"),
            max_order_qty: "1000".parse().expect("max_order_qty"),
            max_position_size: "1000".parse().expect("max_position_size"),
            min_order_qty: "0.001".parse().expect("min_order_qty"),
            price_cap_ratio: "0.1".parse().expect("price_cap_ratio"),
            price_floor_ratio: "0.1".parse().expect("price_floor_ratio"),
            price_tick_decimals: 2,
            qty_tick_decimals: 3,
            quote_asset: "USDT".to_string(),
            quote_decimals: 6,
            symbol: "BTCUSDT".to_string(),
            taker_fee: "0.0006".parse().expect("taker_fee"),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        assert_eq!(response, expected);
    }

    #[tokio::test]
    async fn test_query_symbol_price() {
        let server = MockServer::start().await;
        let mock_response = r#"{
            "base": "BTC",
            "index_price": "120.5",
            "last_price": null,
            "mark_price": "120.6",
            "mid_price": "120.55",
            "quote": "USDT",
            "spread_ask": null,
            "spread_bid": "120.4",
            "symbol": "BTCUSDT",
            "time": "2024-01-01T00:00:00Z"
        }"#;

        let _mock = Mock::given(method("GET"))
            .and(path("/api/query_symbol_price"))
            .and(query_param("symbol", "BTCUSDT"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_raw(mock_response, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = StandxClient::with_config_and_base_urls(
            ClientConfig::default(),
            &server.uri(),
            &server.uri(),
        )
        .expect("client init");

        let response = client
            .query_symbol_price("BTCUSDT")
            .await
            .expect("query_symbol_price failed");

        let expected = SymbolPrice {
            base: "BTC".to_string(),
            index_price: "120.5".parse().expect("index_price"),
            last_price: None,
            mark_price: "120.6".parse().expect("mark_price"),
            mid_price: Some("120.55".parse().expect("mid_price")),
            quote: "USDT".to_string(),
            spread_ask: None,
            spread_bid: Some("120.4".parse().expect("spread_bid")),
            symbol: "BTCUSDT".to_string(),
            time: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(response, expected);
    }

    #[tokio::test]
    async fn test_query_depth_book() {
        let server = MockServer::start().await;
        let mock_response = r#"{
            "asks": [["100.5", "1.2"], ["101.0", "2.0"]],
            "bids": [["99.5", "1.0"], ["99.0", "3.0"]],
            "symbol": "BTCUSDT"
        }"#;

        let _mock = Mock::given(method("GET"))
            .and(path("/api/query_depth_book"))
            .and(query_param("symbol", "BTCUSDT"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_raw(mock_response, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = StandxClient::with_config_and_base_urls(
            ClientConfig::default(),
            &server.uri(),
            &server.uri(),
        )
        .expect("client init");

        let response = client
            .query_depth_book("BTCUSDT")
            .await
            .expect("query_depth_book failed");

        let expected = DepthBook {
            asks: vec![
                DepthLevel("100.5".parse().expect("ask_price"), "1.2".parse().expect("ask_qty")),
                DepthLevel("101.0".parse().expect("ask_price"), "2.0".parse().expect("ask_qty")),
            ],
            bids: vec![
                DepthLevel("99.5".parse().expect("bid_price"), "1.0".parse().expect("bid_qty")),
                DepthLevel("99.0".parse().expect("bid_price"), "3.0".parse().expect("bid_qty")),
            ],
            symbol: "BTCUSDT".to_string(),
        };

        assert_eq!(response, expected);
    }

    #[tokio::test]
    async fn test_get_kline_history() {
        let server = MockServer::start().await;
        let mock_response = r#"{
            "s": "ok",
            "t": [1700000000, 1700000060],
            "c": ["100.2", "100.5"],
            "o": ["100.0", "100.2"],
            "h": ["100.6", "100.8"],
            "l": ["99.8", "100.1"],
            "v": ["1.2", "2.4"]
        }"#;

        let _mock = Mock::given(method("GET"))
            .and(path("/api/kline/history"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("from", "1700000000"))
            .and(query_param("to", "1700000060"))
            .and(query_param("resolution", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_raw(mock_response, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = StandxClient::with_config_and_base_urls(
            ClientConfig::default(),
            &server.uri(),
            &server.uri(),
        )
        .expect("client init");

        let response = client
            .get_kline_history("BTCUSDT", 1_700_000_000, 1_700_000_060, "1")
            .await
            .expect("get_kline_history failed");

        let expected = KlineData {
            s: "ok".to_string(),
            t: vec![1_700_000_000, 1_700_000_060],
            c: vec!["100.2".parse().expect("c"), "100.5".parse().expect("c")],
            o: vec!["100.0".parse().expect("o"), "100.2".parse().expect("o")],
            h: vec!["100.6".parse().expect("h"), "100.8".parse().expect("h")],
            l: vec!["99.8".parse().expect("l"), "100.1".parse().expect("l")],
            v: vec!["1.2".parse().expect("v"), "2.4".parse().expect("v")],
        };

        assert_eq!(response, expected);
    }
}
