use aws_config::Region;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::{env, path::Path};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn upload_to_s3(
    bucket: &str,
    key: &str,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_path.display()).into());
    }

    let access_key =
        env::var("AWS_ACCESS_KEY_ID").map_err(|_| "Missing AWS_ACCESS_KEY_ID in .env")?;
    let secret_key =
        env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| "Missing AWS_SECRET_ACCESS_KEY in .env")?;
    let region_str = env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());

    let region = Region::new(region_str.clone());

    let credentials = Credentials::new(access_key, secret_key, None, None, "from-env");
    let region_provider = RegionProviderChain::first_try(region);

    let config = aws_config::defaults(BehaviorVersion::v2025_01_17())
        .region(region_provider)
        .credentials_provider(credentials)
        // .timeout_config(
        //     TimeoutConfig::builder()
        //         .operation_timeout(Duration::from_secs(15))
        //         .build(),
        // )
        .load()
        .await;

    let client = Client::new(&config);

    let mut file = File::open(file_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

    println!("☁️ Uploading to S3: s3://{}/{}", bucket, key);
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buffer))
        .send()
        .await?;

    println!("✅ Uploaded to S3: s3://{}/{}", bucket, key);
    Ok(())
}
