use std::sync::Arc;

use crate::{
    models::{AppState, VerifyQuery},
    utils::{get_error, CompletedTasksTrait},
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use starknet::{
    core::types::{BlockId, CallFunction, FieldElement},
    macros::selector,
    providers::Provider,
};

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VerifyQuery>,
) -> impl IntoResponse {
    let task_id = 24;
    let addr = &query.addr;

    // check if user has debt
    let call_result = state
        .provider
        .call_contract(
            CallFunction {
                contract_address: state.conf.quests.zklend.contract,
                entry_point_selector: selector!("user_has_debt"),
                calldata: vec![*addr],
            },
            BlockId::Latest,
        )
        .await;

    match call_result {
        Ok(result) => {
            if result.result[0] == FieldElement::ZERO {
                get_error("You didn't borrow any liquidity.".to_string())
            } else {
                match state.upsert_completed_task(query.addr, task_id).await {
                    Ok(_) => (StatusCode::OK, Json(json!({"res": true}))).into_response(),
                    Err(e) => get_error(format!("{}", e)),
                }
            }
        }
        Err(e) => get_error(format!("{}", e)),
    }
}