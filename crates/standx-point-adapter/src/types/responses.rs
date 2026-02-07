/*
[INPUT]:  API schema definitions and serde requirements
[OUTPUT]: Typed Rust response structs with serialization support
[POS]:    Data layer - type definitions for API communication
[UPDATE]: When API schema changes or new types added
*/

use serde::{Deserialize, Serialize};

use super::enums::Chain;
use super::models::{Balance, Order, Position};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewOrderResponse {
    pub code: i32,
    pub message: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    pub code: i32,
    pub message: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeLeverageResponse {
    pub code: i32,
    pub message: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginatedOrders {
    #[serde(rename = "page_size")]
    pub page_size: u32,
    pub result: Vec<Order>,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PositionsResponse(pub Vec<Position>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BalanceResponse(pub Balance);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthSigninResponse {
    pub success: bool,
    #[serde(rename = "signedData")]
    pub signed_data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthLoginResponse {
    pub token: String,
    pub address: String,
    pub alias: String,
    pub chain: Chain,
    #[serde(rename = "perpsAlpha")]
    pub perps_alpha: bool,
}
