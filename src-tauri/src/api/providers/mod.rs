pub mod coomer;
pub mod manager;
pub mod onlyhaven;
pub mod pawchive;
pub mod queue;
pub mod traits;

pub use coomer::CoomerProvider;
pub use manager::ProviderManager;
pub use onlyhaven::OnlyHavenProvider;
pub use pawchive::{PawchiveClient, PawchiveProvider};
pub use queue::{ProviderQueueConfig, ProviderRequestQueue};
pub use traits::{derive_subdomain_url, ProviderConfig, ProviderHealth, SourceProvider};

pub fn default_pawchive_services() -> Vec<String> {
    PawchiveProvider::default_services()
}

pub fn default_onlyhaven_services() -> Vec<String> {
    OnlyHavenProvider::default_services()
}

pub fn default_coomer_services() -> Vec<String> {
    CoomerProvider::default_services()
}
