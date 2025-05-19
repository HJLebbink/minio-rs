use crate::s3::http::BaseUrl;
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct MadminClient {
    http_client: reqwest::Client,
    pub(crate) shared: Arc<SharedClientItems>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SharedClientItems {
    pub(crate) base_url: BaseUrl,
    //pub(crate) provider: Option<Arc<Box<(dyn Provider + Send + Sync + 'static)>>>,
}

impl SharedClientItems {
    pub fn new(base_url: BaseUrl) -> Self {
        Self {
            base_url,
            // provider,
        }
    }
}
