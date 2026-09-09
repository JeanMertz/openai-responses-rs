#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client as Http, StatusCode,
};
use serde_json::json;
use std::env;
use types::{Error, Include, InputItemList, Request, Response, ResponseResult};
#[cfg(feature = "stream")]
use {
    async_fn_stream::try_fn_stream,
    eventsource_stream::Eventsource as _,
    futures::{Stream, StreamExt},
    types::Event,
};

/// Types for interacting with the Responses API.
pub mod types;

/// The OpenAI Responses API Client.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
    responses_path: String,
    headers: HeaderMap,
}

/// Errors that can occur when creating a new Client.
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    /// The provided API key contains invalid header value characters. Only visible ASCII characters (32-127) are permitted.
    #[error(
        "The provided API key contains invalid header value characters. Only visible ASCII characters (32-127) are permitted."
    )]
    InvalidApiKey,
    /// Failed to create the HTTP Client
    #[error("Failed to create the HTTP Client: {0}")]
    CouldNotCreateClient(#[from] reqwest::Error),
    /// Could not retrieve the ``OPENAI_API_KEY`` env var
    #[error("Could not retrieve the $OPENAI_API_KEY env var")]
    ApiKeyNotFound,
}

#[cfg(feature = "stream")]
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// The request could not be sent.
    #[error(transparent)]
    Transport(#[from] reqwest::Error),

    /// The API answered with a non-success status.
    ///
    /// The body is read here because the response is consumed by the stream; a
    /// caller that only saw the status could not report what went wrong.
    #[error("HTTP {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        headers: Box<reqwest::header::HeaderMap>,
        body: String,
    },

    /// The response body is not a well-formed event stream.
    #[error("Failed to read the event stream: {0}")]
    Sse(String),

    #[error("Failed to parse event data: {0}")]
    Parsing(#[from] serde_json::Error),
}

impl Client {
    /// Creates a new Client with the given API key.
    ///
    /// # Errors
    /// - `CreateError::CouldNotCreateClient` if the HTTP Client could not be created.
    /// - `CreateError::InvalidApiKey` if the API key contains invalid header value characters.
    pub fn new(api_key: &str) -> Result<Self, CreateError> {
        let client = Http::builder()
            .default_headers(HeaderMap::from_iter([(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .map_err(|_| CreateError::InvalidApiKey)?,
            )]))
            .build()?;

        Ok(Self {
            client,
            base_url: "https://api.openai.com".to_owned(),
            responses_path: "/v1/responses".to_owned(),
            headers: HeaderMap::new(),
        })
    }

    /// Set the base URL for the OpenAI API.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the path the responses endpoints are served from.
    ///
    /// Defaults to `/v1/responses`. Set this when the API is reached through a
    /// host that serves the endpoint at a different path.
    #[must_use]
    pub fn with_responses_path(mut self, path: impl Into<String>) -> Self {
        self.responses_path = path.into();
        self
    }

    /// Send `headers` with every request.
    ///
    /// These are merged over the `Authorization` header set at construction, so
    /// an `Authorization` entry here replaces it.
    #[must_use]
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Creates a new Client from the `OPENAI_API_KEY` environment variable.
    ///
    /// # Errors
    /// - `CreateError::CouldNotCreateClient` if the HTTP Client could not be created.
    /// - `CreateError::InvalidApiKey` if the API key contains invalid header value characters.
    /// - `CreateError::ApiKeyNotFound` if the `OPENAI_API_KEY` environment variable is not set or contains an equal sign or NUL (`'='` or `'\0'`).
    pub fn from_env() -> Result<Self, CreateError> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| CreateError::ApiKeyNotFound)?;

        Self::new(&api_key)
    }

    /// Creates a model response.
    ///
    /// Provide [text](https://platform.openai.com/docs/guides/text) or [image](https://platform.openai.com/docs/guides/images) inputs to generate [text](https://platform.openai.com/docs/guides/text) or [JSON](https://platform.openai.com/docs/guides/structured-outputs) outputs.
    /// Have the model call your own [custom code](https://platform.openai.com/docs/guides/function-calling) or use built-in [tools](https://platform.openai.com/docs/guides/tools) like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search) to use your own data as input for the model's response.
    /// To receive a stream of tokens as they are generated, use the `stream` function instead.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code (except for 400, which will return an OpenAI error instead).
    pub async fn create(
        &self,
        mut request: Request,
    ) -> Result<Result<Response, Error>, reqwest::Error> {
        // Use the `stream` function to stream the response.
        request.stream = Some(false);

        let mut response = self
            .client
            .post(format!("{}{}", self.base_url, self.responses_path))
            .headers(self.headers.clone())
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::BAD_REQUEST {
            response = response.error_for_status()?;
        }

        response.json::<ResponseResult>().await.map(Into::into)
    }

    #[cfg(feature = "stream")]
    /// Creates a model response and streams it back as it is generated.
    ///
    /// Provide [text](https://platform.openai.com/docs/guides/text) or [image](https://platform.openai.com/docs/guides/images) inputs to generate [text](https://platform.openai.com/docs/guides/text) or [JSON](https://platform.openai.com/docs/guides/structured-outputs) outputs.
    /// Have the model call your own [custom code](https://platform.openai.com/docs/guides/function-calling) or use built-in [tools](https://platform.openai.com/docs/guides/tools) like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search) to use your own data as input for the model's response.
    ///
    /// To receive the response as a regular HTTP response, use the `create` function.
    ///
    /// The response body is parsed as SSE regardless of its `Content-Type`: a
    /// host that streams without labelling the response still returns a body
    /// the caller can read.
    pub fn stream(
        &self,
        mut request: Request,
    ) -> impl Stream<Item = Result<Event, StreamError>> + Send + 'static + use<'_> {
        // Use the `create` function to receive a regular HTTP response.
        request.stream = Some(true);

        let builder = self
            .client
            .post(format!("{}{}", self.base_url, self.responses_path))
            .headers(self.headers.clone())
            .json(&request);

        let stream = try_fn_stream(|emitter| async move {
            let response = builder.send().await?;
            let status = response.status();

            if !status.is_success() {
                let headers = Box::new(response.headers().clone());
                let body = response.text().await.unwrap_or_default();

                emitter
                    .emit_err(StreamError::Status {
                        status,
                        headers,
                        body,
                    })
                    .await;

                return Ok(());
            }

            let mut events = response.bytes_stream().eventsource();

            while let Some(event) = events.next().await {
                match event {
                    Ok(message) => match serde_json::from_str::<Event>(&message.data) {
                        Ok(event) => emitter.emit(event).await,
                        Err(error) => emitter.emit_err(StreamError::Parsing(error)).await,
                    },
                    // The body is broken from here on; what follows belongs to
                    // a response the caller has already been told is bad.
                    Err(error) => {
                        emitter.emit_err(StreamError::Sse(error.to_string())).await;
                        break;
                    }
                }
            }

            Ok(())
        });

        Box::pin(stream)
    }

    /// Retrieves a model response with the given ID.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code (except for 400, which will return an OpenAI error instead).
    pub async fn get(
        &self,
        response_id: &str,
        include: Option<Include>,
    ) -> Result<Result<Response, Error>, reqwest::Error> {
        let mut response = self
            .client
            .get(format!(
                "{}{}/{response_id}",
                self.base_url, self.responses_path
            ))
            .headers(self.headers.clone())
            .query(&json!({ "include": include }))
            .send()
            .await?;

        if response.status() != StatusCode::BAD_REQUEST {
            response = response.error_for_status()?;
        }

        response.json::<ResponseResult>().await.map(Into::into)
    }

    /// Deletes a model response with the given ID.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code.
    pub async fn delete(&self, response_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .delete(format!(
                "{}{}/{response_id}",
                self.base_url, self.responses_path
            ))
            .headers(self.headers.clone())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Returns a list of input items for a given response.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code.
    pub async fn list_inputs(&self, response_id: &str) -> Result<InputItemList, reqwest::Error> {
        self.client
            .get(format!(
                "{}{}/{response_id}/inputs",
                self.base_url, self.responses_path
            ))
            .headers(self.headers.clone())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}
