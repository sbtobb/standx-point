/*
[INPUT]:  API schema definitions and serde requirements
[OUTPUT]: Typed Rust structs with serialization support
[POS]:    Data layer - type definitions for API communication
[UPDATE]: When API schema changes or new types added
[UPDATE]: 2026-02-08 allow missing Order.avail_locked in deserialization
*/

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::enums::{MarginMode, OrderStatus, OrderType, Side, TimeInForce};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub base_asset: String,
    pub base_decimals: u32,
    pub created_at: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub def_leverage: Decimal,
    pub depth_ticks: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(with = "rust_decimal::serde::str")]
    pub maker_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_leverage: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_open_orders: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_order_qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_position_size: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub min_order_qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price_cap_ratio: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price_floor_ratio: Decimal,
    pub price_tick_decimals: u32,
    pub qty_tick_decimals: u32,
    pub quote_asset: String,
    pub quote_decimals: u32,
    pub symbol: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub taker_fee: Decimal,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub avail_locked: Decimal,
    pub cl_ord_id: String,
    pub closed_block: i64,
    pub created_at: String,
    pub created_block: i64,
    #[serde(with = "rust_decimal::serde::str")]
    pub fill_avg_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fill_qty: Decimal,
    pub id: i64,
    #[serde(with = "rust_decimal::serde::str")]
    pub leverage: Decimal,
    pub liq_id: i64,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub margin: Decimal,
    pub order_type: OrderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    pub position_id: i64,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub qty: Decimal,
    pub reduce_only: bool,
    pub remark: String,
    pub side: Side,
    pub source: String,
    pub status: OrderStatus,
    pub symbol: String,
    pub time_in_force: TimeInForce,
    pub updated_at: String,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub bankruptcy_price: Decimal,
    pub created_at: String,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub entry_price: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub entry_value: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub holding_margin: Decimal,
    pub id: i64,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub initial_margin: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub leverage: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub liq_price: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub maint_margin: Decimal,
    pub margin_asset: String,
    pub margin_mode: MarginMode,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub mark_price: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub mmr: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub position_value: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub qty: Decimal,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub realized_pnl: Decimal,
    pub status: String,
    pub symbol: String,
    pub time: String,
    pub updated_at: String,
    #[serde(
        default,
        deserialize_with = "serde_helpers::deserialize_decimal_or_zero",
        serialize_with = "serde_helpers::serialize_decimal"
    )]
    pub upnl: Decimal,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    #[serde(with = "rust_decimal::serde::str")]
    pub isolated_balance: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub isolated_upnl: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cross_balance: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cross_margin: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cross_upnl: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub locked: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cross_available: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub upnl: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub equity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl_freeze: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub created_at: String,
    pub fee_asset: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub fee_qty: Decimal,
    pub id: i64,
    pub order_id: i64,
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub qty: Decimal,
    pub side: Side,
    pub symbol: String,
    pub updated_at: String,
    pub user: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub value: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolPrice {
    pub base: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub index_price: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub last_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub mark_price: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub mid_price: Option<Decimal>,
    pub quote: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub spread_ask: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub spread_bid: Option<Decimal>,
    pub symbol: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthLevel(
    #[serde(with = "rust_decimal::serde::str")] pub Decimal,
    #[serde(with = "rust_decimal::serde::str")] pub Decimal,
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthBook {
    pub asks: Vec<DepthLevel>,
    pub bids: Vec<DepthLevel>,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KlineData {
    pub s: String,
    pub t: Vec<i64>,
    #[serde(
        deserialize_with = "serde_helpers::deserialize_decimal_vec",
        serialize_with = "serde_helpers::serialize_decimal_vec"
    )]
    pub c: Vec<Decimal>,
    #[serde(
        deserialize_with = "serde_helpers::deserialize_decimal_vec",
        serialize_with = "serde_helpers::serialize_decimal_vec"
    )]
    pub o: Vec<Decimal>,
    #[serde(
        deserialize_with = "serde_helpers::deserialize_decimal_vec",
        serialize_with = "serde_helpers::serialize_decimal_vec"
    )]
    pub h: Vec<Decimal>,
    #[serde(
        deserialize_with = "serde_helpers::deserialize_decimal_vec",
        serialize_with = "serde_helpers::serialize_decimal_vec"
    )]
    pub l: Vec<Decimal>,
    #[serde(
        deserialize_with = "serde_helpers::deserialize_decimal_vec",
        serialize_with = "serde_helpers::serialize_decimal_vec"
    )]
    pub v: Vec<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundingRate {
    pub id: i64,
    pub symbol: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub funding_rate: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub index_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub mark_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub premium: Decimal,
    pub time: String,
    pub created_at: String,
    pub updated_at: String,
}

mod serde_helpers {
    use super::Decimal;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;
    use std::str::FromStr;

    pub fn deserialize_decimal_vec<'de, D>(deserializer: D) -> Result<Vec<Decimal>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<String> = Vec::deserialize(deserializer)?;
        values
            .into_iter()
            .map(|value| Decimal::from_str(&value).map_err(serde::de::Error::custom))
            .collect()
    }

    pub fn serialize_decimal_vec<S>(values: &[Decimal], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<String> = values.iter().map(Decimal::to_string).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize_decimal_or_zero<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value.is_null() {
            return Ok(Decimal::ZERO);
        }

        if let Some(raw) = value.as_str() {
            if raw.trim().is_empty() {
                return Ok(Decimal::ZERO);
            }
            return Decimal::from_str(raw).map_err(serde::de::Error::custom);
        }

        if value.is_number() {
            return Decimal::from_str(&value.to_string()).map_err(serde::de::Error::custom);
        }

        Err(serde::de::Error::custom("invalid decimal value"))
    }

    pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn order_deserializes_without_avail_locked() {
        let value = json!({
            "cl_ord_id": "cl-1",
            "closed_block": 0,
            "created_at": "0",
            "created_block": 0,
            "fill_avg_price": "0",
            "fill_qty": "0",
            "id": 1,
            "leverage": "1",
            "liq_id": 0,
            "margin": "0",
            "order_type": "limit",
            "position_id": 0,
            "price": "100",
            "qty": "1",
            "reduce_only": false,
            "remark": "",
            "side": "buy",
            "source": "test",
            "status": "open",
            "symbol": "BTC-USD",
            "time_in_force": "gtc",
            "updated_at": "0",
            "user": "user"
        });

        let order: Order = serde_json::from_value(value).expect("order should deserialize");

        assert_eq!(order.avail_locked, Decimal::ZERO);
    }

    #[test]
    fn order_deserializes_without_margin() {
        let value = json!({
            "cl_ord_id": "cl-1",
            "closed_block": 0,
            "created_at": "0",
            "created_block": 0,
            "fill_avg_price": "0",
            "fill_qty": "0",
            "id": 1,
            "leverage": "1",
            "liq_id": 0,
            "order_type": "limit",
            "position_id": 0,
            "price": "100",
            "qty": "1",
            "reduce_only": false,
            "remark": "",
            "side": "buy",
            "source": "test",
            "status": "open",
            "symbol": "BTC-USD",
            "time_in_force": "gtc",
            "updated_at": "0",
            "user": "user"
        });

        let order: Order = serde_json::from_value(value).expect("order should deserialize");

        assert_eq!(order.margin, Decimal::ZERO);
    }
}
