use ditto_http::{Client, DownloadClient, Progress, Url};
use miette::{IntoDiagnostic, Result, WrapErr};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    download_ninja(&client).await
}

async fn download_ninja(client: &Client) -> Result<()> {
    static URL: &str =
        "https://github.com/ninja-build/ninja/releases/download/v1.10.2/ninja-mac.zip";
    let url = Url::parse(URL).unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    tokio::spawn(async move {
        while let Some(Progress { downloaded, total }) = rx.recv().await {
            println!("{downloaded} / {total}")
        }
    });
    let _bytes = client
        .download(
            url,
            String::from("6fa359f491fac7e5185273c6421a000eea6a2f0febf0ac03ac900bd4d80ed2a5"),
            tx,
        )
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("downloading {URL}"))?;
    Ok(())
}
