use async_curl::actor::CurlActor;
use curl_http_client::{
    collector::Collector, error::Error, http_client::HttpClient, request::HttpRequest,
    response::HttpResponse,
};

#[derive(Clone)]
pub struct Curl {
    pub actor_handle: CurlActor<Collector>,
}

impl Curl {
    pub fn new() -> Self {
        Self {
            actor_handle: CurlActor::new(),
        }
    }

    fn to_curl_request(request: oauth2::HttpRequest) -> HttpRequest {
        let body = if request.body.is_empty() {
            None
        } else {
            Some(request.body)
        };
        HttpRequest {
            url: request.url,
            method: request.method,
            headers: request.headers,
            body,
        }
    }

    fn to_oauth_response(response: HttpResponse) -> oauth2::HttpResponse {
        let body = if let Some(body) = response.body {
            body
        } else {
            Vec::new()
        };
        oauth2::HttpResponse {
            status_code: response.status_code,
            headers: response.headers,
            body,
        }
    }

    pub async fn send(
        &self,
        request: oauth2::HttpRequest,
    ) -> Result<oauth2::HttpResponse, Error<Collector>> {
        log::trace!("Request Url: {}", request.url);
        log::trace!("Request Header: {:?}", request.headers);
        log::trace!("Request Method: {}", request.method);
        log::trace!(
            "Request Body: {}",
            std::str::from_utf8(request.body.as_slice()).unwrap_or_default()
        );

        let response = HttpClient::new(Collector::RamAndHeaders(Vec::new(), Vec::new()))
            .request(Curl::to_curl_request(request))?
            .nonblocking(self.actor_handle.clone())
            .perform()
            .await
            .map(Curl::to_oauth_response)?;

        log::trace!("Response Header: {:?}", response.headers);
        log::trace!(
            "Response Body: {}",
            std::str::from_utf8(response.body.as_slice()).unwrap_or_default()
        );
        log::info!("Response Status: {}", response.status_code);
        Ok(response)
    }
}

impl Default for Curl {
    fn default() -> Self {
        Self::new()
    }
}
