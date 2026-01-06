pub mod client;
pub mod utils;

pub use client::AuthClient;
pub use utils::{
    create_auth_response, create_error_response, create_missing_api_key_response, extract_api_key,
    extract_api_key_from_headers,
};
