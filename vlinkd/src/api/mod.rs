use std::sync::Arc;
use axum::{Router, ServiceExt};
use log::error;
use crate::api::state::{AppState, AppStateInner};
use crate::network::ctrl::NetworkCtrl;

mod router;
mod state;
mod network;

const WEB_PORT: u16 = 5514;

pub async fn start_http_server(listen_addr: Option<String>, ctrl: NetworkCtrl) -> anyhow::Result<()> {
    let addr = listen_addr.unwrap_or(format!("0.0.0.0:{WEB_PORT}"));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let state = AppState::new(Arc::new(AppStateInner::new(ctrl)));

    let app = Router::new()
        .nest("/api", router::api())
        .with_state(state);
    let api_server = axum::serve(listener, app.into_make_service());

    tokio::spawn(async move {
        if let Err(e) = api_server.await {
            error!("api_server stop error:{:?}", e);
        }
    });


    // .with_graceful_shutdown(async move {
    //     let _ = api_server_ch.await;
    // });

    Ok(())
}