use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use thiserror::Error;
use tokio::sync::mpsc::Sender;

pub use reqwest::{Client, Url};

#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected sha256, got: {actual:?}")]
    Sha256Mismatch { expected: String, actual: String },

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
}

#[async_trait::async_trait]
pub trait DownloadClient {
    async fn download(&self, url: Url, sha256: String, progress: Sender<Progress>)
        -> Result<Bytes>;
}

#[async_trait::async_trait]
impl DownloadClient for reqwest::Client {
    async fn download(
        &self,
        url: Url,
        sha256: String,
        progress: Sender<Progress>,
    ) -> Result<Bytes> {
        let response = self.get(url).send().await?;
        // TODO: check response status
        if let Some(total) = response.content_length() {
            let mut bytes = if let Ok(total) = total.try_into() {
                BytesMut::with_capacity(total)
            } else {
                BytesMut::new()
            };
            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                bytes.extend(&chunk);
                downloaded = std::cmp::min(downloaded + (chunk.len() as u64), total);
                // ignore the error
                let _result = progress.send(Progress { downloaded, total }).await;
            }
            check_integrity(bytes.freeze(), sha256)
        } else {
            let bytes = response.bytes().await?;
            check_integrity(bytes, sha256)
        }
    }
}

fn check_integrity(bytes: Bytes, expected_sha256: String) -> Result<Bytes> {
    let actual_sha256 = sha256::digest(bytes.as_ref());
    if expected_sha256 != actual_sha256 {
        Err(Error::Sha256Mismatch {
            expected: expected_sha256,
            actual: actual_sha256,
        })
    } else {
        Ok(bytes)
    }
}
