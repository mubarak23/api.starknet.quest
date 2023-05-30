use crate::models::AppState;
use crate::utils::get_error;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct TokenURI {
    name: String,
    description: String,
    image: String,
    attributes: Option<Vec<Attribute>>,
}

#[derive(Serialize)]
pub struct Attribute {
    trait_type: String,
    value: u32,
}

#[derive(Deserialize)]
pub struct LevelQuery {
    level: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(level_query): Query<LevelQuery>,
) -> Response {
    let level = level_query
        .level
        .and_then(|level_str| level_str.parse::<u32>().ok());

    fn get_level(level_int: u32) -> &'static str {
        match level_int {
            2 => "Silver",
            3 => "Gold",
            _ => "Bronze",
        }
    }

    match level {
        Some(level_int) if level_int > 0 && level_int <= 3 => {
            let image_link = format!(
                "{}/starkfighter/level{}.webp",
                state.conf.variables.app_link, level_int
            );
            let response = TokenURI {
                name: format!("StarkFighter {} Arcade", get_level(level_int)),
                description: "A starknet.quest NFT won during the Starkfighter event.".into(),
                image: image_link,
                attributes: Some(vec![Attribute {
                    trait_type: "level".into(),
                    value: level_int,
                }]),
            };
            (StatusCode::OK, Json(response)).into_response()
        }

        Some(4) => (
            StatusCode::OK,
            Json(TokenURI {
                name: "Starknet ID Tribe Totem".into(),
                description: "A Starknet Quest NFT won for creating a StarknetID profile.".into(),
                image: format!(
                    "{}/starknetid/nft1.webp",
                    state.conf.variables.app_link
                ),
                attributes: None,
            }),
        )
            .into_response(),

        _ => get_error("Error, this level is not correct".into()),
    }
}
