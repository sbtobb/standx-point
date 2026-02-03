/*
[INPUT]:  API schema definitions and serde requirements
[OUTPUT]: Typed Rust request structs with serialization support
[POS]:    Data layer - type definitions for API communication
[UPDATE]: When API schema changes or new types added
*/

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::enums::{Chain, MarginMode, OrderStatus, OrderType, Side, TimeInForce};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewOrderRequest {
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    #[serde(with = "rust_decimal::serde::str")]
    pub qty: Decimal,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    #[serde(with = "rust_decimal::serde::str_option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_mode: Option<MarginMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage: Option<u32>,
    #[serde(with = "rust_decimal::serde::str_option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sl_price: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeLeverageRequest {
    pub symbol: String,
    pub leverage: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryOrdersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OrderStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_type: Option<OrderType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPositionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthSigninRequest {
    pub chain: Chain,
    pub address: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthLoginRequest {
    pub chain: Chain,
    pub signature: String,
    #[serde(rename = "signedData")]
    pub signed_data: String,
    #[serde(rename = "expiresSeconds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_seconds: Option<u64>,
}
