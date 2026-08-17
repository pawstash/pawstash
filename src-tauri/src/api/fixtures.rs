use axum::{
    body::Body,
    extract::{Path, Query},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub const MOCK_SESSION_TOKEN: &str = "pawchive_auth_session_token_xyz987";
pub const MOCK_COOKIE_HEADER: &str = "session=pawchive_auth_session_token_xyz987";

pub fn mock_creator_profile_fixture() -> serde_json::Value {
    json!({
        "id": "3340149",
        "name": "ArtisanPaws",
        "service": "patreon",
        "public_id": null,
        "relation_id": null,
        "indexed": 1720000000,
        "updated": 1723000000,
        "kemono_favorited": 420,
        "ever_imported": true
    })
}

pub fn mock_favorites_fixture() -> serde_json::Value {
    json!([
        {
            "id": "142680139",
            "service": "patreon",
            "name": "Special Illustration Pack",
            "faved_seq": 101,
            "indexed": "2026-06-15 12:00:00",
            "updated": "2026-06-16 14:30:00"
        },
        {
            "id": "3340149",
            "service": "patreon",
            "name": "ArtisanPaws",
            "faved_seq": 102,
            "indexed": "2026-06-01 08:00:00",
            "updated": "2026-06-10 10:15:00"
        }
    ])
}

pub fn mock_post_fixture() -> serde_json::Value {
    json!({
        "id": "142680139",
        "user": "3340149",
        "service": "patreon",
        "title": "Summer Special Highres Release",
        "content": "<p>Thank you all for the support! Here is the archive.</p>",
        "substring": "Thank you all for the support",
        "published": "2026-06-15 12:00:00",
        "added": "2026-06-15 12:05:00",
        "edited": null,
        "attachments": [
            {
                "name": "summer_illustration.png",
                "path": "/data/33/40/summer_illustration.png",
                "server": "file1",
                "size": 15482910
            }
        ],
        "file": {
            "name": "preview.jpg",
            "path": "/data/33/40/preview.jpg",
            "server": "img1",
            "size": 254810
        },
        "favorite_count": 215,
        "attachment_count": 1
    })
}

pub fn mock_post_comments_fixture() -> serde_json::Value {
    json!([
        {
            "id": "c1001",
            "parent_id": null,
            "commenter": "u552",
            "commenter_name": "FriendlySupporter",
            "content": "Incredible work on the lighting!",
            "published": "2026-06-15 13:00:00",
            "revisions": [
                {
                    "id": 1,
                    "content": "Nice work on the lighting!",
                    "added": "2026-06-15 12:45:00"
                }
            ]
        }
    ])
}

pub fn create_mock_pawchive_router() -> Router {
    Router::new()
        // 1. Authenticated Login flow
        .route(
            "/account/login",
            post(|_headers: HeaderMap, body: String| async move {
                if body.contains("username=test_user") && body.contains("password=correct_pass") {
                    Response::builder()
                        .status(StatusCode::FOUND)
                        .header(header::LOCATION, "/account")
                        .header(
                            header::SET_COOKIE,
                            format!("{MOCK_COOKIE_HEADER}; Path=/; Secure; HttpOnly; SameSite=Lax"),
                        )
                        .body(Body::from("Redirecting to account..."))
                        .unwrap()
                } else {
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::LOCATION, "/account/login")
                        .body(Body::from("Invalid username or password"))
                        .unwrap()
                }
            }),
        )
        // 2. Logout flow
        .route(
            "/account/logout",
            get(|headers: HeaderMap| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie == Some(MOCK_COOKIE_HEADER) || cookie == Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    (StatusCode::OK, "Logged out successfully")
                } else {
                    (StatusCode::OK, "No active session")
                }
            }),
        )
        // 3. Authenticated Favorites listing
        .route(
            "/api/v1/account/favorites",
            get(|headers: HeaderMap, Query(params): Query<HashMap<String, String>>| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie != Some(MOCK_COOKIE_HEADER) && cookie != Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
                }

                let favs = mock_favorites_fixture();
                if let Some(kind) = params.get("type") {
                    let filtered: Vec<serde_json::Value> = favs
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter(|item| {
                            if kind == "post" {
                                item.get("name").and_then(|v| v.as_str()).map(|n| n.contains("Pack")).unwrap_or(false)
                            } else {
                                item.get("name").and_then(|v| v.as_str()).map(|n| !n.contains("Pack")).unwrap_or(false)
                            }
                        })
                        .cloned()
                        .collect();
                    (StatusCode::OK, Json(filtered)).into_response()
                } else {
                    (StatusCode::OK, Json(favs)).into_response()
                }
            }),
        )
        // 4. Post favorites mutation (Add / Delete)
        .route(
            "/api/v1/favorites/post/:service/:creator_id/:post_id",
            post(|headers: HeaderMap, Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie != Some(MOCK_COOKIE_HEADER) && cookie != Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    return (StatusCode::UNAUTHORIZED, "Unauthorized");
                }
                (StatusCode::OK, "Added favorite")
            })
            .delete(|headers: HeaderMap, Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie != Some(MOCK_COOKIE_HEADER) && cookie != Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    return (StatusCode::UNAUTHORIZED, "Unauthorized");
                }
                (StatusCode::OK, "Removed favorite")
            }),
        )
        // 5. Creator favorites mutation (Add / Delete)
        .route(
            "/api/v1/favorites/creator/:service/:creator_id",
            post(|headers: HeaderMap, Path((_svc, _user)): Path<(String, String)>| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie != Some(MOCK_COOKIE_HEADER) && cookie != Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    return (StatusCode::UNAUTHORIZED, "Unauthorized");
                }
                (StatusCode::OK, "Added creator favorite")
            })
            .delete(|headers: HeaderMap, Path((_svc, _user)): Path<(String, String)>| async move {
                let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                if cookie != Some(MOCK_COOKIE_HEADER) && cookie != Some(&format!("session={MOCK_SESSION_TOKEN}")) {
                    return (StatusCode::UNAUTHORIZED, "Unauthorized");
                }
                (StatusCode::OK, "Removed creator favorite")
            }),
        )
        // 6. Creator Profile
        .route(
            "/api/v1/:service/user/:creator_id/profile",
            get(|Path((_svc, _user)): Path<(String, String)>| async move {
                Json(mock_creator_profile_fixture())
            }),
        )
        // 7. Creator Posts
        .route(
            "/api/v1/:service/user/:creator_id",
            get(|Path((_svc, _user)): Path<(String, String)>| async move {
                Json(vec![mock_post_fixture()])
            }),
        )
        // 8. Single Post
        .route(
            "/api/v1/:service/user/:creator_id/post/:post_id",
            get(|Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                Json(mock_post_fixture())
            }),
        )
        // 9. Post Comments
        .route(
            "/api/v1/:service/user/:creator_id/post/:post_id/comments",
            get(|Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                Json(mock_post_comments_fixture())
            }),
        )
        // 10. Post Flagging
        .route(
            "/api/v1/:service/user/:creator_id/post/:post_id/flag",
            post(|Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                (StatusCode::OK, Json(json!({"success": true})))
            })
            .get(|Path((_svc, _user, _post)): Path<(String, String, String)>| async move {
                (StatusCode::OK, "flagged")
            }),
        )
        // 11. Hash search
        .route(
            "/api/v1/search_hash/:hash",
            get(|Path(hash): Path<String>| async move {
                Json(json!({
                    "id": 8801,
                    "hash": hash,
                    "mtime": "2026-06-15 12:00:00",
                    "ctime": "2026-06-15 12:00:00",
                    "mime": "image/png",
                    "ext": "png",
                    "added": "2026-06-15 12:05:00",
                    "size": 15482910,
                    "ihash": null,
                    "posts": [
                        {
                            "id": "142680139",
                            "user": "3340149",
                            "service": "patreon",
                            "title": "Summer Special Highres Release"
                        }
                    ],
                    "discord_posts": []
                }))
            }),
        )
        // 12. App version
        .route(
            "/api/v1/app_version",
            get(|| async move { "pawchive-v1.4.2" }),
        )
}

pub async fn spawn_mock_pawchive_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = create_mock_pawchive_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::pawchive::PawchiveClient;
    use crate::config::settings::{AppSettings, ProxyMode};

    #[tokio::test]
    async fn authenticated_login_flow_success() {
        let (addr, handle) = spawn_mock_pawchive_server().await;
        let settings = AppSettings {
            api_domain: format!("http://127.0.0.1:{}", addr.port()),
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        let client = PawchiveClient::new(settings).unwrap();

        // 1. Successful Login
        let cookie = client.login("test_user", "correct_pass").await.unwrap();
        assert!(cookie.contains(MOCK_SESSION_TOKEN));

        // 2. Update client with session cookie
        let auth_settings = AppSettings {
            api_domain: format!("http://127.0.0.1:{}", addr.port()),
            session_cookie: cookie.clone(),
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        client.update_settings(auth_settings).await.unwrap();

        // 3. Fetch Account Favorites (Authorized)
        let favorites = client.fetch_account_favorites(None).await.unwrap();
        assert_eq!(favorites.len(), 2);
        assert_eq!(favorites[0].id, "142680139");
        assert_eq!(favorites[1].id, "3340149");

        // 4. Fetch filtered favorites
        let post_favs = client.fetch_account_favorites(Some("post")).await.unwrap();
        assert_eq!(post_favs.len(), 1);
        assert_eq!(post_favs[0].id, "142680139");

        // 5. Add / Delete Post Favorite
        let add_fav = client
            .set_post_favorite("patreon", "3340149", "142680139", true)
            .await
            .unwrap();
        assert!(add_fav.success);
        let del_fav = client
            .set_post_favorite("patreon", "3340149", "142680139", false)
            .await
            .unwrap();
        assert!(del_fav.success);

        // 6. Add / Delete Creator Favorite
        let add_creator = client
            .set_creator_favorite("patreon", "3340149", true)
            .await
            .unwrap();
        assert!(add_creator.success);
        let del_creator = client
            .set_creator_favorite("patreon", "3340149", false)
            .await
            .unwrap();
        assert!(del_creator.success);

        // 7. Post details & comments
        let post = client
            .fetch_post("patreon", "3340149", "142680139")
            .await
            .unwrap();
        assert_eq!(post.id, "142680139");
        assert_eq!(post.title, "Summer Special Highres Release");

        let comments = client
            .fetch_post_comments("patreon", "3340149", "142680139")
            .await
            .unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments[0].commenter_name.as_deref(),
            Some("FriendlySupporter")
        );
        assert_eq!(comments[0].revisions.len(), 1);

        // 8. Flagging post & checking flag status
        let flag_res = client
            .flag_post("patreon", "3340149", "142680139")
            .await
            .unwrap();
        assert!(flag_res.success);
        assert!(client
            .is_post_flagged("patreon", "3340149", "142680139")
            .await
            .unwrap());

        // 9. Logout
        client.logout().await.unwrap();

        handle.abort();
    }

    #[tokio::test]
    async fn authenticated_login_flow_rejection() {
        let (addr, handle) = spawn_mock_pawchive_server().await;
        let settings = AppSettings {
            api_domain: format!("http://127.0.0.1:{}", addr.port()),
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        let client = PawchiveClient::new(settings).unwrap();

        // Failed login
        let err = client
            .login("test_user", "wrong_password")
            .await
            .unwrap_err();
        assert!(err.contains("rejected the username or password"));

        handle.abort();
    }

    #[tokio::test]
    async fn authenticated_favorites_unauthorized_without_cookie() {
        let (addr, handle) = spawn_mock_pawchive_server().await;
        let settings = AppSettings {
            api_domain: format!("http://127.0.0.1:{}", addr.port()),
            session_cookie: String::new(),
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        let client = PawchiveClient::new(settings).unwrap();

        // Fetch favorites without cookie returns 401 Unauthorized error
        let err = client.fetch_account_favorites(None).await.unwrap_err();
        assert!(err.contains("401"));

        handle.abort();
    }
}
