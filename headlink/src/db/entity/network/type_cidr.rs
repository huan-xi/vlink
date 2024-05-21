use std::str::FromStr;
use ip_network::Ipv4Network;
use sea_orm::{ColIdx, QueryResult, TryGetable, TryGetError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ipv4NetworkWrapper(pub Ipv4Network);



impl Into<Ipv4Network> for Ipv4NetworkWrapper{
    fn into(self) -> Ipv4Network {
        self.0
    }
}
impl TryGetable for Ipv4NetworkWrapper {
    fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        // row
        //                         .try_get::<Option<$type>, _>(idx.as_sqlx_postgres_index())
        //                         .map_err(|e| sqlx_error_to_query_err(e).into())
        //                         .and_then(|opt| opt.ok_or_else(|| err_null_idx_col(idx))),

        let idx = index.as_sqlx_postgres_index();
        // Ok(Ipv4NetworkWrapper(Ipv4Network::from_str(res.try_get("", idx)?)?))
        todo!();
    }
}

impl From<Ipv4NetworkWrapper> for sea_orm::Value {
    fn from(value: Ipv4NetworkWrapper) -> Self {
        value.0.to_string().into()
    }
}
