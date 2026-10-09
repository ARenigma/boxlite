//! Image handlers (`/v1/images`).
//!
//! These answer from `runtime.images()`, which on the embedded runtime `serve`
//! runs is the local image cache. `usage` counts a catalog against a limit, and
//! a cache has neither, so it responds `400 UnsupportedError`; so does moving a
//! tag, since a cache keeps no catalog of tags.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use boxlite::runtime::types::ImageDetail;

use super::super::types::{
    ImageDetailResponse, ImageInfoResponse, ImageTagEventResponse, ImageUsageResponse,
    ImageVersionResponse, ListImagesResponse, MoveTagRequest,
};
use super::super::{AppState, error_from_boxlite};

pub(in crate::commands::serve) async fn list_images(
    State(state): State<Arc<AppState>>,
) -> Response {
    let handle = match state.runtime.images() {
        Ok(h) => h,
        Err(e) => return error_from_boxlite(&e),
    };
    match handle.list().await {
        Ok(images) => {
            let images = images
                .into_iter()
                .map(|image| ImageInfoResponse {
                    cached_at: image.cached_at.to_rfc3339(),
                    size_bytes: image.size.map(|size| size.0),
                    reference: image.reference,
                    repository: image.repository,
                    tag: image.tag,
                    id: image.id,
                })
                .collect();
            Json(ListImagesResponse { images }).into_response()
        }
        Err(e) => error_from_boxlite(&e),
    }
}

pub(in crate::commands::serve) async fn get_image(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let handle = match state.runtime.images() {
        Ok(h) => h,
        Err(e) => return error_from_boxlite(&e),
    };
    match handle.get(&name).await {
        Ok(detail) => Json(detail_response(detail)).into_response(),
        Err(e) => error_from_boxlite(&e),
    }
}

pub(in crate::commands::serve) async fn move_image_tag(
    State(state): State<Arc<AppState>>,
    Path((name, tag)): Path<(String, String)>,
    Json(request): Json<MoveTagRequest>,
) -> Response {
    let handle = match state.runtime.images() {
        Ok(h) => h,
        Err(e) => return error_from_boxlite(&e),
    };
    match handle.move_tag(&name, &tag, &request.digest).await {
        Ok(detail) => Json(detail_response(detail)).into_response(),
        Err(e) => error_from_boxlite(&e),
    }
}

/// An image in the box API's shape, for a read and for a tag move alike.
fn detail_response(detail: ImageDetail) -> ImageDetailResponse {
    ImageDetailResponse {
        name: detail.name,
        tags: detail.tags,
        curated: detail.curated,
        versions: detail
            .versions
            .into_iter()
            .map(|version| ImageVersionResponse {
                recorded_at: version.recorded_at.to_rfc3339(),
                last_used_at: version.last_used_at.map(|time| time.to_rfc3339()),
                upstream_missing_at: version.upstream_missing_at.map(|time| time.to_rfc3339()),
                digest: version.digest,
                size_bytes: version.size_bytes,
                source_ref: version.source_ref,
                tags: version.tags,
            })
            .collect(),
        history: detail
            .history
            .into_iter()
            .map(|event| ImageTagEventResponse {
                moved_at: event.moved_at.to_rfc3339(),
                tag: event.tag,
                from_digest: event.from_digest,
                to_digest: event.to_digest,
                actor: event.actor,
            })
            .collect(),
    }
}

pub(in crate::commands::serve) async fn remove_image(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let handle = match state.runtime.images() {
        Ok(h) => h,
        Err(e) => return error_from_boxlite(&e),
    };
    match handle.remove(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_from_boxlite(&e),
    }
}

pub(in crate::commands::serve) async fn image_usage(
    State(state): State<Arc<AppState>>,
) -> Response {
    let handle = match state.runtime.images() {
        Ok(h) => h,
        Err(e) => return error_from_boxlite(&e),
    };
    match handle.usage().await {
        Ok(usage) => Json(ImageUsageResponse {
            count: usage.count,
            limit: usage.limit,
            known_bytes: usage.known_bytes,
        })
        .into_response(),
        Err(e) => error_from_boxlite(&e),
    }
}

#[cfg(test)]
mod tests {
    use boxlite::runtime::types::{ImageTagEvent, ImageVersion};
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;

    /// serve's body names each field a catalog's does, so a REST runtime
    /// reads a version's tags and times, and the history, from serve too.
    #[test]
    fn a_detail_carries_version_tags_times_and_the_history() {
        let day = |day| Utc.with_ymd_and_hms(2026, 9, day, 0, 0, 0).unwrap();
        let detail = ImageDetail {
            name: "quay.io/acme/app".to_string(),
            tags: vec!["stable".to_string()],
            curated: false,
            versions: vec![ImageVersion {
                digest: "sha256:bb".to_string(),
                size_bytes: None,
                source_ref: "quay.io/acme/app:v2".to_string(),
                recorded_at: day(1),
                tags: vec!["stable".to_string()],
                last_used_at: Some(day(2)),
                upstream_missing_at: None,
            }],
            history: vec![ImageTagEvent {
                tag: "stable".to_string(),
                from_digest: Some("sha256:aa".to_string()),
                to_digest: "sha256:bb".to_string(),
                actor: None,
                moved_at: day(3),
            }],
        };

        let body = serde_json::to_value(detail_response(detail)).unwrap();

        assert_eq!(
            body,
            json!({
                "name": "quay.io/acme/app",
                "tags": ["stable"],
                "curated": false,
                "versions": [{
                    "digest": "sha256:bb",
                    "size_bytes": null,
                    "source_ref": "quay.io/acme/app:v2",
                    "recorded_at": "2026-09-01T00:00:00+00:00",
                    "tags": ["stable"],
                    "last_used_at": "2026-09-02T00:00:00+00:00",
                    "upstream_missing_at": null,
                }],
                "history": [{
                    "tag": "stable",
                    "from_digest": "sha256:aa",
                    "to_digest": "sha256:bb",
                    "actor": null,
                    "moved_at": "2026-09-03T00:00:00+00:00",
                }],
            })
        );
    }
}
