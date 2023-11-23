/*
 this endpoint will return static data of leaderboard and position of user address
 Steps to get data over different time intervals :
 1) iterate over one week timestamps and add total points and get top 3 and get user position
 2) iterate over one month timestamps and add total points and get top 3 and get user position
 3) iterate over all timestamps and add total points and get top 3 and get user position
 */

use std::collections::HashMap;
use crate::{models::AppState, utils::get_error};
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};

use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use reqwest::StatusCode;
use std::sync::Arc;
use chrono::{Duration, Utc};
use mongodb::Collection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetLeaderboardInfoQuery {
    /*
    user address
    */
    addr: String,
}

pub async fn get_leaderboard_toppers(
    collection: &Collection<Document>,
    days: i64,
    address: &String,
) -> Document {
    let time_gap = if days > 0 {
        (Utc::now() - Duration::days(days)).timestamp_millis()
    } else {
        0
    };

    let leaderboard_pipeline = vec![
        doc! {
        "$match": doc! {
            "timestamp": doc! {
                "$gte": time_gap
            }
        }
    },
        doc! {
        "$sort": doc! {
            "experience": -1,
            "timestamp": 1,
            "_id": 1
        }
    },
        doc! {
        "$facet": doc! {
            "best_users": [
                doc! {
                    "$limit": 3
                },
                doc! {
                    "$lookup": doc! {
                        "from": "achieved",
                        "localField": "_id",
                        "foreignField": "addr",
                        "as": "associatedAchievement"
                    }
                },
                doc! {
                    "$project": doc! {
                        "_id": 0,
                        "address": "$_id",
                        "xp": "$experience",
                        "achievements": doc! {
                            "$size": "$associatedAchievement"
                        }
                    }
                }
            ],
            "total_users": [
                doc! {
                    "$count": "total"
                }
            ],
            "rank": [
                doc! {
                    "$addFields": doc! {
                        "tempSortField": 1
                    }
                },
                doc! {
                    "$setWindowFields": doc! {
                        "sortBy": doc! {
                            "tempSortField": -1
                        },
                        "output": doc! {
                            "rank": doc! {
                                "$documentNumber": doc! {}
                            }
                        }
                    }
                },
                doc! {
                    "$match": doc! {
                        "_id": address
                    }
                },
                doc! {
                    "$project": doc! {
                        "_id": 0,
                        "rank": "$rank"
                    }
                },
                doc! {
                    "$unwind": "$rank"
                }
            ]
        }
    },
        doc! {
        "$project": doc! {
            "best_users": 1,
            "total_users": doc! {
                "$arrayElemAt": [
                    "$total_users.total",
                    0
                ]
            },
            "position": doc! {
                "$arrayElemAt": [
                    "$rank.rank",
                    0
                ]
            }
        }
    },
    ];


    return match collection.aggregate(leaderboard_pipeline, None).await {
        Ok(mut cursor) => {
            let mut query_result = Vec::new();
            while let Some(result) = cursor.try_next().await.unwrap() {
                query_result.push(result)
            }
            if query_result.is_empty() {
                return Document::new();
            }
            query_result[0].clone()
        }
        Err(_err) => {
            Document::new()
        }
    };
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetLeaderboardInfoQuery>,
) -> impl IntoResponse {
    let addr: String = query.addr.to_string();
    let mut error_flag = Document::new();
    let users_collection = state.db.collection::<Document>("leaderboard_table");

    // fetch weekly toppers and check if valid result
    let weekly_toppers_result = get_leaderboard_toppers(&users_collection, 7, &addr).await;
    let weekly_toppers = match weekly_toppers_result.is_empty() {
        true => {
            error_flag.insert("status", true);
            error_flag.clone()
        }
        false => weekly_toppers_result.clone(),
    };

    // fetch monthly toppers and check if valid result
    let monthly_toppers_result = get_leaderboard_toppers(&users_collection, 30, &addr).await;
    let monthly_toppers = match monthly_toppers_result.is_empty() {
        true => {
            error_flag.insert("status", true);
            error_flag.clone()
        }
        false => monthly_toppers_result.clone(),
    };

    // fetch all time toppers and check if valid result
    let all_time_toppers_result = get_leaderboard_toppers(&users_collection, -1, &addr).await;
    let all_time_toppers = match all_time_toppers_result.is_empty() {
        true => {
            error_flag.insert("status", true);
            error_flag.clone()
        }
        false => all_time_toppers_result.clone(),
    };


    // check if any error occurred
    if error_flag.contains_key("status") {
        return get_error("Error querying leaderboard".to_string());
    }

    let mut res: HashMap<String, Document> = HashMap::new();
    res.insert("weekly".to_string(), weekly_toppers);
    res.insert("monthly".to_string(), monthly_toppers);
    res.insert("all_time".to_string(), all_time_toppers);
    (StatusCode::OK, Json(res)).into_response()
}