use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Response wrapper for mutation endpoints (create/update).
/// Includes the Postgres transaction ID for Electric sync.
///
/// **SQLite compatibility**: `pg_current_xact_id()` is Postgres-only.
/// For local SQLite backends, always set `txid: 0`. The field is preserved
/// so the frontend `MutationResponse<T>` interface remains identical across
/// both Postgres (remote) and SQLite (local) deployments.
#[derive(Debug, Serialize, Deserialize)]
pub struct MutationResponse<T> {
    pub data: T,
    pub txid: i64,
}

/// Response wrapper for delete endpoints.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct DeleteResponse {
    pub txid: i64,
}
