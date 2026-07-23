use std::time::Duration;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;

#[derive(Clone)]
pub struct R2 {
    shadow_client: Client,
    sanctum_client: Client,
    dp_client: Client,
    shadow_bucket: String,
    sanctum_bucket: String,
    dp_bucket: String,
}

/// Build an S3 client for one R2 bucket with its OWN credentials, so shadow and
/// sanctum can use separate access keys (and be split across accounts later).
/// `endpoint` may include a trailing `/<bucket>` path - we strip it because
/// `force_path_style(true)` re-appends the bucket itself.
fn build_client(endpoint_env: &str, id_env: &str, secret_env: &str, region: &str, bucket: &str) -> Client {
    let mut endpoint = std::env::var(endpoint_env)
        .unwrap_or_else(|_| panic!("{} must be set", endpoint_env));
    let suffix = format!("/{}", bucket);
    if endpoint.ends_with(&suffix) {
        endpoint.truncate(endpoint.len() - suffix.len());
    }
    let access_id = std::env::var(id_env)
        .unwrap_or_else(|_| panic!("{} must be set", id_env));
    let access_secret = std::env::var(secret_env)
        .unwrap_or_else(|_| panic!("{} must be set", secret_env));

    let creds = Credentials::new(access_id, access_secret, None, None, "static");
    let conf = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .force_path_style(true)
        .build();
    Client::from_conf(conf)
}

impl R2 {
    pub async fn new() -> Self {
        let region = std::env::var("CF_R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let shadow_bucket = std::env::var("CF_R2_SHADOW_BUCKET").unwrap_or_else(|_| "silo-cat-shadow".to_string());
        let sanctum_bucket = std::env::var("CF_R2_SANCTUM_BUCKET").unwrap_or_else(|_| "silo-cat-sanctum".to_string());

        // Each bucket gets its own credentials (shadow vs sanctum) so they can
        // be rotated / split independently.
        let shadow_client = build_client(
            "CF_R2_SHADOW_API_URL", "CF_R2_SHADOW_ACCESS_ID", "CF_R2_SHADOW_ACCESS_SECRET",
            &region, &shadow_bucket,
        );
        let sanctum_client = build_client(
            "CF_R2_SANCTUM_API_URL", "CF_R2_SANCTUM_ACCESS_ID", "CF_R2_SANCTUM_ACCESS_SECRET",
            &region, &sanctum_bucket,
        );

        // Display-picture bucket (avatars). dev + staging share one bucket; prod
        // has its own. Keys may be the same as sanctum's (granted bucket access).
        let dp_bucket = std::env::var("CF_R2_DP_BUCKET").unwrap_or_else(|_| "silocat-dp-staging".to_string());
        let dp_client = build_client(
            "CF_R2_DP_API_URL", "CF_R2_DP_ACCESS_ID", "CF_R2_DP_ACCESS_SECRET",
            &region, &dp_bucket,
        );

        Self { shadow_client, sanctum_client, dp_client, shadow_bucket, sanctum_bucket, dp_bucket }
    }

    /// Upload bytes directly (server-side put). Used for normalized display
    /// pictures, which are small and produced in-process.
    pub async fn put_object(
        &self,
        storage: &str,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        let (client, bucket) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str()),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str()),
            "dp" => (&self.dp_client, self.dp_bucket.as_str()),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };
        client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(bytes))
            .content_type(content_type)
            .send()
            .await?;
        Ok(())
    }

    /// Generate a pre-signed URL for uploading a file
    pub async fn presigned_put_url(
        &self,
        storage: &str,
        key: &str,
    ) -> anyhow::Result<String> {

        let (client, bucket, time) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str(), 24),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str(), 24),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };

        let link_expiry = Duration::from_secs(60 * 60 * time);

        let presigned_req = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .presigned(
                PresigningConfig::expires_in(link_expiry)
                    .map_err(|e| anyhow::anyhow!("Failed presigning config: {:?}", e))?,
            )
            .await?;

        Ok(presigned_req.uri().to_string())
    }

    /// Generate a pre-signed URL for downloading a file
    pub async fn presigned_get_url(
        &self,
        storage: &str,
        key: &str
    ) -> anyhow::Result<String> {

        // Shorter download-URL lifetime bounds the window in which an
        // already-issued URL keeps working after a share is revoked / a "once"
        // link is spent. 2h is ample for chunked downloads (clients can request
        // fresh URLs). Avatars ("dp") can live longer.
        let (client, bucket, time) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str(), 2),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str(), 2),
            "dp" => (&self.dp_client, self.dp_bucket.as_str(), 24),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };

        let link_expiry = Duration::from_secs(60 * 60 * time);

        let presigned_req = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(
                PresigningConfig::expires_in(link_expiry)
                    .map_err(|e| anyhow::anyhow!("Failed presigning config: {:?}", e))?,
            )
            .await?;

        Ok(presigned_req.uri().to_string())
    }

    /// Download an object's full bytes (used for admin file downloads).
    pub async fn get_object(&self, storage: &str, key: &str) -> anyhow::Result<Vec<u8>> {
        let (client, bucket) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str()),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str()),
            "dp" => (&self.dp_client, self.dp_bucket.as_str()),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };
        let resp = client.get_object().bucket(bucket).key(key).send().await?;
        let data = resp.body.collect().await?.into_bytes();
        Ok(data.to_vec())
    }

    /// Delete an object. Idempotent: deleting a missing key is not an error.
    pub async fn delete_object(&self, storage: &str, key: &str) -> anyhow::Result<()> {
        let (client, bucket) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str()),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str()),
            "dp" => (&self.dp_client, self.dp_bucket.as_str()),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };
        client.delete_object().bucket(bucket).key(key).send().await?;
        Ok(())
    }

    /// Calculate bucket usage (object count and total size)
    pub async fn get_bucket_usage(&self, storage: &str) -> anyhow::Result<(i64, i64)> {
        let (client, bucket, _) = match storage {
            "shadow" => (&self.shadow_client, self.shadow_bucket.as_str(), 24),
            "sanctum" => (&self.sanctum_client, self.sanctum_bucket.as_str(), 24),
            _ => return Err(anyhow::anyhow!("Invalid storage option")),
        };

        let mut total_objects = 0;
        let mut total_size = 0;
        let mut continuation_token = None;

        loop {
            let mut req = client.list_objects_v2().bucket(bucket);
            if let Some(token) = continuation_token {
                req = req.continuation_token(token);
            }

            let resp = req.send().await?;
            
            if let Some(contents) = resp.contents {
                for object in contents {
                    total_objects += 1;
                    total_size += object.size.unwrap_or(0);
                }
            }

            if resp.is_truncated.unwrap_or(false) {
                continuation_token = resp.next_continuation_token;
            } else {
                break;
            }
        }

        Ok((total_objects, total_size))
    }
}
