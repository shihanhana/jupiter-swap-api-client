use std::collections::HashMap;

use quote::{InternalQuoteRequest, QuoteRequest, QuoteResponse};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use swap::{SwapInstructionsResponse, SwapInstructionsResponseInternal, SwapRequest, SwapResponse};
use thiserror::Error;
use serde::Deserialize;
use serde_json::Value;

pub mod quote;
pub mod route_plan_with_metadata;
pub mod serde_helpers;
pub mod swap;
pub mod transaction_config;

#[derive(Clone)]
pub struct JupiterSwapApiClient {
    pub base_path: String,
    client: Client,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Request failed with status {status}: {body}")]
    RequestFailed {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Failed to deserialize response: {0}")]
    DeserializationError(#[from] reqwest::Error),
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

async fn check_is_success(response: Response) -> Result<Response, ClientError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::RequestFailed { status, body });
    }
    Ok(response)
}

async fn check_status_code_and_deserialize<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ClientError> {
    let response = check_is_success(response).await?;
    let bytes = response.bytes().await.map_err(ClientError::DeserializationError)?;
    serde_json::from_slice(&bytes).map_err(ClientError::JsonError)
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    #[serde(flatten)]
    pub data: Value,
}

impl JupiterSwapApiClient {
    pub fn new(base_path: String) -> Self {
        Self { 
            base_path,
            client: Client::new(),
        }
    }

    pub async fn quote(&self, quote_request: &QuoteRequest) -> Result<QuoteResponse, ClientError> {
        let url = format!("{}/quote", self.base_path);
        let extra_args = quote_request.quote_args.clone();
        let internal_quote_request = InternalQuoteRequest::from(quote_request.clone());
        let response = self.client
            .get(url)
            .query(&internal_quote_request)
            .query(&extra_args)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap(
        &self,
        swap_request: &SwapRequest,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<SwapResponse, ClientError> {
        let response = self.client
            .post(format!("{}/swap", self.base_path))
            .query(&extra_args)
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap_instructions(
        &self,
        swap_request: &SwapRequest,
    ) -> Result<SwapInstructionsResponse, ClientError> {
        // 开始计时
        let start = std::time::Instant::now();
        
        // 构建并执行请求
        let request = self.client
            .post(format!("{}/swap-instructions", self.base_path))
            .json(swap_request)
            .build()?;
            
        println!("请求 URL: {}", request.url());
        
        // 发送请求
        let execute_start = std::time::Instant::now();
        let response = self.client.execute(request).await?;
        let execute_elapsed = execute_start.elapsed();
        println!("请求执行耗时: {:.3} ms", execute_elapsed.as_micros() as f64 / 1000.0);
            
        // 反序列化
        let deserialize_start = std::time::Instant::now();
        let result = check_status_code_and_deserialize::<SwapInstructionsResponseInternal>(response)
            .await
            .map(Into::into);
        let deserialize_elapsed = deserialize_start.elapsed();
        println!("反序列化耗时: {:.3} ms", deserialize_elapsed.as_micros() as f64 / 1000.0);
            
        // 计算总耗时
        let total_elapsed = start.elapsed();
        println!("总耗时: {:.3} ms", total_elapsed.as_micros() as f64 / 1000.0);
        
        result
    }

    pub async fn health(&self) -> Result<HealthResponse, ClientError> {
        let response = self.client
            .get(format!("{}/health", self.base_path))
            .send()
            .await?;
        
        response
            .json::<HealthResponse>()
            .await
            .map_err(ClientError::DeserializationError)
    }
}
