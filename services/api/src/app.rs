use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use auth::middleware::{auth_layer, AuthState};
use axum::routing::get;
use axum::{middleware, Router};
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::repository::ListingPhotoRepository;
use tower_http::trace::TraceLayer;
use user_domain::repository::UserRepository;

use crate::backend_authorization::{enforce_backend_roles, BackendAuthorizationState};
use crate::backend_rate_limit::{
    self, enforce_backend_rate_limit, AllowAllBackendRateLimiter, BackendRateLimitState,
    RedisBackendRateLimiter,
};
use crate::listing_marker_serving::ListingMarkerServingGateway;
use crate::startup::{
    build_building_reader, build_internal_auth_secret, build_parcel_lookup,
    build_photo_download_issuer, build_photo_upload_issuer, build_redis_pool_shared,
    build_verifier, connect_postgres, is_production_env, required_env, StartupError,
};
use crate::{http, routes, startup, traffic_auth_policy};

pub async fn run() -> ExitCode {
    match async_main().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(event = "startup_failed", error = %error, "api startup failed");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn async_main() -> Result<(), StartupError> {
    let database_url = required_env("DATABASE_URL")?;
    let dev_mode = env::var("AUTH_DEV_MODE").unwrap_or_default() == "true";
    let is_production = is_production_env();

    let pool = connect_postgres(&database_url).await?;

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
    let app_state = routes::users::UsersState {
        user_repo: user_repo.clone(),
    };

    let listing_repo: Arc<dyn ListingRepository> = Arc::new(PgListingRepository::new(pool.clone()));
    let photo_repo: Arc<dyn ListingPhotoRepository> =
        Arc::new(PgListingPhotoRepository::new(pool.clone()));

    let parcel_lookup = build_parcel_lookup(is_production)?;
    let photo_upload_issuer = build_photo_upload_issuer(is_production)?;
    let photo_download_issuer = build_photo_download_issuer(is_production)?;
    let photo_object_verifier = startup::build_photo_object_verifier(is_production)?;

    let listings_state = routes::listings::ListingsState {
        listing_repo,
        photo_repo,
        parcel_lookup,
        photo_upload_issuer,
        photo_download_issuer,
        photo_object_verifier,
    };

    let verifier = build_verifier(dev_mode, is_production)?;
    let redis_pool_shared = build_redis_pool_shared(is_production)?;
    let backend_rate_limiter: Arc<dyn backend_rate_limit::BackendRateLimiter> =
        if let Some(redis_pool) = &redis_pool_shared {
            Arc::new(RedisBackendRateLimiter::new(redis_pool.clone()))
        } else {
            Arc::new(AllowAllBackendRateLimiter)
        };
    let backend_rate_limit_state = BackendRateLimitState::new(
        backend_rate_limiter,
        traffic_auth_policy::BACKEND_RATE_POLICIES,
    );
    let backend_authorization_state =
        BackendAuthorizationState::new(traffic_auth_policy::BACKEND_ROLE_POLICIES);

    let jti_denylist: Option<Arc<dyn auth::jti_denylist::JtiDenylist>> =
        redis_pool_shared.as_ref().map(|pool| {
            let dl: Arc<dyn auth::jti_denylist::JtiDenylist> = Arc::new(
                auth::jti_denylist::RedisJtiDenylist::new(pool.as_ref().clone()),
            );
            dl
        });

    let auth_state = AuthState {
        verifier,
        user_repo,
        jti_denylist,
        fail_closed_on_denylist_error: is_production,
    };

    let internal_auth_secret = build_internal_auth_secret(is_production)?;
    let auth_event_state = routes::auth_event::AuthEventState {
        pool,
        internal_auth_secret,
    };

    let health_state = routes::health::HealthState {
        pool: auth_event_state.pool.clone(),
        redis_pool: redis_pool_shared.clone(),
    };
    let listing_marker_serving = Arc::new(ListingMarkerServingGateway::new(
        listings_state.listing_repo.clone(),
        redis_pool_shared.clone(),
    ));

    let public: Router<()> = routes::health::public_router(health_state, !is_production);
    let protected: Router<()> = Router::new()
        .route("/users/me", get(routes::users::me))
        .route("/users/:id", get(routes::users::get_user))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let listing_marker_tiles_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
            get(routes::listing_marker_tiles::get_listing_marker_tile),
        )
        .with_state(routes::listing_marker_tiles::ListingMarkerTilesState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_counts_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-counts/listing",
            get(routes::listing_marker_counts::get_listing_marker_count),
        )
        .with_state(routes::listing_marker_counts::ListingMarkerCountsState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_filters_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-filters/listing",
            axum::routing::post(routes::listing_marker_filters::post_listing_marker_filter),
        )
        .with_state(routes::listing_marker_filters::ListingMarkerFiltersState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_masks_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-masks/listing/:z/:x/:y",
            get(routes::listing_marker_masks::get_listing_marker_mask),
        )
        .with_state(routes::listing_marker_masks::ListingMarkerMasksState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_tombstones_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-tombstones/listing/:z/:x/:y",
            get(routes::listing_marker_tombstones::get_listing_marker_tombstones),
        )
        .with_state(
            routes::listing_marker_tombstones::ListingMarkerTombstonesState {
                serving: listing_marker_serving.clone(),
            },
        )
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_deltas_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-deltas/listing/:z/:x/:y_pbf",
            get(routes::listing_marker_deltas::get_listing_marker_deltas),
        )
        .with_state(routes::listing_marker_deltas::ListingMarkerDeltasState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));

    let listings_router: Router<()> = Router::new()
        .route(
            "/listings",
            get(routes::listings::get_listings).post(routes::listings::create_listing),
        )
        .route(
            "/listings/:id",
            get(routes::listings::get_listing_detail).patch(routes::listings::patch_listing),
        )
        .route(
            "/listings/:id/submit-for-review",
            axum::routing::post(routes::listings::submit_for_review),
        )
        .route(
            "/listings/:id/revise",
            axum::routing::post(routes::listings::revise),
        )
        .route(
            "/listings/:id/photos",
            axum::routing::post(routes::listings::request_photo_upload),
        )
        .route(
            "/listings/:listing_id/photos/:photo_id",
            get(routes::listings::get_photo_download_redirect)
                .delete(routes::listings::delete_photo),
        )
        .route(
            "/listings/:listing_id/photos/:photo_id/confirm",
            axum::routing::post(routes::listings::confirm_photo_upload),
        )
        .with_state(listings_state.clone())
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let parcels_state = routes::parcels::ParcelsState {
        parcel_lookup: listings_state.parcel_lookup.clone(),
    };
    let parcels_router: Router<()> = Router::new()
        .route("/api/parcels/:pnu", get(routes::parcels::get_parcel))
        .with_state(parcels_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let building_reader = build_building_reader(is_production)?;
    let buildings_state = routes::buildings::BuildingsState {
        reader: building_reader,
    };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let notification_repo: Arc<dyn notification_domain::repository::NotificationRepository> =
        Arc::new(db::notification::PgNotificationRepository::new(
            auth_event_state.pool.clone(),
        ));
    let listing_repo_shared: Arc<dyn listing_domain::repository::ListingRepository> = Arc::new(
        db::listing::PgListingRepository::new(auth_event_state.pool.clone()),
    );
    let bookmark_repo: Arc<dyn bookmark_domain::repository::BookmarkRepository> = Arc::new(
        db::bookmark::PgBookmarkRepository::new(auth_event_state.pool.clone()),
    );
    let bookmarks_state = routes::bookmarks::BookmarksState {
        bookmark_repo,
        listing_repo: listing_repo_shared.clone(),
        notification_repo: notification_repo.clone(),
    };
    let bookmarks_router: Router<()> = Router::new()
        .route(
            "/listings/:id/bookmark",
            axum::routing::post(routes::bookmarks::toggle_bookmark)
                .delete(routes::bookmarks::delete_bookmark),
        )
        .route("/me/bookmarks", get(routes::bookmarks::list_my_bookmarks))
        .with_state(bookmarks_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let admin_listings_state = routes::admin_listings::AdminListingsState {
        listing_repo: listing_repo_shared,
        notification_repo: notification_repo.clone(),
    };
    let admin_router: Router<()> = Router::new()
        .route(
            "/admin/listings/:id/approve",
            axum::routing::post(routes::admin_listings::approve_listing),
        )
        .route(
            "/admin/listings/:id/reject",
            axum::routing::post(routes::admin_listings::reject_listing),
        )
        .with_state(admin_listings_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    let notifications_state = routes::notifications::NotificationsState { notification_repo };
    let notifications_router: Router<()> = Router::new()
        .route(
            "/me/notifications",
            get(routes::notifications::list_notifications),
        )
        .route(
            "/me/notifications/unread-count",
            get(routes::notifications::unread_count),
        )
        .route(
            "/me/notifications/:id/read",
            axum::routing::patch(routes::notifications::mark_read),
        )
        .route(
            "/me/notifications/mark-all-read",
            axum::routing::post(routes::notifications::mark_all_read),
        )
        .with_state(notifications_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state,
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state,
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));

    let platform_core_events_router: Router<()> = Router::new()
        .route(
            "/internal/platform-core/events",
            axum::routing::post(routes::platform_core_events::post_platform_core_event),
        )
        .with_state(routes::platform_core_events::PlatformCoreEventsState {
            pool: auth_event_state.pool.clone(),
            internal_auth_secret: auth_event_state.internal_auth_secret.clone(),
        });

    let metrics_router: Router<()> = Router::new()
        .route("/internal/metrics", get(routes::metrics::get_metrics))
        .with_state(routes::metrics::MetricsState {
            pool: auth_event_state.pool.clone(),
            internal_auth_secret: auth_event_state.internal_auth_secret.clone(),
        });

    let internal: Router<()> = Router::new()
        .route(
            "/internal/auth/event",
            axum::routing::post(routes::auth_event::post_auth_event),
        )
        .with_state(auth_event_state);

    let app = public
        .merge(protected)
        .merge(listing_marker_tiles_router)
        .merge(listing_marker_counts_router)
        .merge(listing_marker_filters_router)
        .merge(listing_marker_masks_router)
        .merge(listing_marker_tombstones_router)
        .merge(listing_marker_deltas_router)
        .merge(listings_router)
        .merge(parcels_router)
        .merge(buildings_router)
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(platform_core_events_router)
        .merge(metrics_router)
        .merge(internal)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));

    let addr = env::var("API_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|source| StartupError::Bind {
            addr: addr.clone(),
            source,
        })?;
    axum::serve(listener, app)
        .await
        .map_err(|source| StartupError::Serve { source })
}
