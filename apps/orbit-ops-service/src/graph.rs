// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
}

pub struct GraphManager {
    pub db_pool: PgPool,
}

impl GraphManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn add_edge_internal(&self, edge: &GraphEdge) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO knowledge_graph_edges (source_type, source_id, target_type, target_id, relation)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (source_type, source_id, target_type, target_id, relation) DO NOTHING"
        )
        .bind(&edge.source_type)
        .bind(&edge.source_id)
        .bind(&edge.target_type)
        .bind(&edge.target_id)
        .bind(&edge.relation)
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}

pub async fn create_edge(
    State(state): State<Arc<crate::ServiceState>>,
    Json(req): Json<CreateEdgeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let edge = GraphEdge {
        source_type: req.source_type.to_uppercase(),
        source_id: req.source_id,
        target_type: req.target_type.to_uppercase(),
        target_id: req.target_id,
        relation: req.relation.to_uppercase(),
    };

    state
        .graph
        .add_edge_internal(&edge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

#[derive(Serialize)]
pub struct ImpactTrace {
    pub node_type: String,
    pub node_id: String,
    pub upstream_relations: Vec<GraphEdge>,
    pub downstream_relations: Vec<GraphEdge>,
}

pub async fn query_impact(
    State(state): State<Arc<crate::ServiceState>>,
    Path((node_type, node_id)): Path<(String, String)>,
) -> Result<Json<ImpactTrace>, (StatusCode, String)> {
    let n_type = node_type.to_uppercase();

    // Query downstream: this node is the SOURCE
    let down_rows = sqlx::query(
        "SELECT * FROM knowledge_graph_edges WHERE source_type = $1 AND source_id = $2",
    )
    .bind(&n_type)
    .bind(&node_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let downstream_relations = down_rows
        .into_iter()
        .map(|row| GraphEdge {
            source_type: row.get("source_type"),
            source_id: row.get("source_id"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            relation: row.get("relation"),
        })
        .collect();

    // Query upstream: this node is the TARGET
    let up_rows = sqlx::query(
        "SELECT * FROM knowledge_graph_edges WHERE target_type = $1 AND target_id = $2",
    )
    .bind(&n_type)
    .bind(&node_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let upstream_relations = up_rows
        .into_iter()
        .map(|row| GraphEdge {
            source_type: row.get("source_type"),
            source_id: row.get("source_id"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            relation: row.get("relation"),
        })
        .collect();

    Ok(Json(ImpactTrace {
        node_type: n_type,
        node_id,
        upstream_relations,
        downstream_relations,
    }))
}
