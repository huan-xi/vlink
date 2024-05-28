use std::sync::Arc;
use derive_new::new;
use crate::network::ctrl::NetworkCtrl;

#[derive(Clone, new)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(new)]
pub struct AppStateInner {
    ctrl: NetworkCtrl,
}