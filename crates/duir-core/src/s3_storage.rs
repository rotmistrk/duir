use crate::error::{OmelaError, Result};
use crate::model::TodoFile;
use crate::storage::TodoStorage;

/// S3 path: `s3://bucket/prefix/name`
#[derive(Debug, Clone)]
pub struct S3Path {
    pub bucket: String,
    pub key: String,
}

impl S3Path {
    /// Parse an `s3://bucket/key` path.
    #[must_use]
    pub fn parse(path: &str) -> Option<Self> {
        let rest = path.strip_prefix("s3://")?;
        let (bucket, key) = rest.split_once('/')?;
        if bucket.is_empty() {
            return None;
        }
        Some(Self {
            bucket: bucket.to_owned(),
            key: key.to_owned(),
        })
    }

    /// Check if a string is an S3 path.
    #[must_use]
    pub fn is_s3(path: &str) -> bool {
        path.starts_with("s3://")
    }
}

/// S3-backed storage. Each todo file is an S3 object.
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    rt: tokio::runtime::Runtime,
}

impl S3Storage {
    /// Create a new S3 storage client using default AWS credentials.
    ///
    /// # Errors
    /// Returns an error if the tokio runtime cannot be created.
    pub fn new() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| OmelaError::Other(format!("Tokio runtime: {e}")))?;

        let config = rt.block_on(aws_config::load_defaults(aws_config::BehaviorVersion::latest()));
        let client = aws_sdk_s3::Client::new(&config);

        Ok(Self { client, rt })
    }

    /// Read bytes from S3.
    ///
    /// # Errors
    /// Returns an error if the object cannot be read.
    pub fn read_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        self.rt.block_on(async {
            let resp = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 get {bucket}/{key}: {e}")))?;

            let bytes = resp
                .body
                .collect()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 read body: {e}")))?
                .into_bytes();

            Ok(bytes.to_vec())
        })
    }

    /// Write bytes to S3.
    ///
    /// # Errors
    /// Returns an error if the object cannot be written.
    pub fn write_bytes(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<()> {
        self.rt.block_on(async {
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(data.into())
                .send()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 put {bucket}/{key}: {e}")))?;
            Ok(())
        })
    }

    /// List objects under a prefix.
    ///
    /// # Errors
    /// Returns an error if the listing fails.
    pub fn list_objects(&self, bucket: &str, prefix: &str) -> Result<Vec<String>> {
        self.rt.block_on(async {
            let resp = self
                .client
                .list_objects_v2()
                .bucket(bucket)
                .prefix(prefix)
                .delimiter("/")
                .send()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 list {bucket}/{prefix}: {e}")))?;

            let mut results = Vec::new();

            // Common prefixes (directories)
            for p in resp.common_prefixes() {
                if let Some(prefix_str) = p.prefix() {
                    results.push(format!("s3://{bucket}/{prefix_str}"));
                }
            }

            // Objects
            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    results.push(format!("s3://{bucket}/{key}"));
                }
            }

            Ok(results)
        })
    }

    /// List buckets.
    ///
    /// # Errors
    /// Returns an error if the listing fails.
    pub fn list_buckets(&self) -> Result<Vec<String>> {
        self.rt.block_on(async {
            let resp = self
                .client
                .list_buckets()
                .send()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 list buckets: {e}")))?;

            let names: Vec<String> = resp
                .buckets()
                .iter()
                .filter_map(|b| b.name().map(|n| format!("s3://{n}/")))
                .collect();

            Ok(names)
        })
    }

    /// Load a todo file from S3.
    ///
    /// # Errors
    /// Returns an error if the object cannot be read or parsed.
    pub fn load_todo(&self, bucket: &str, key: &str) -> Result<TodoFile> {
        let bytes = self.read_bytes(bucket, key)?;
        let content = String::from_utf8(bytes).map_err(|e| OmelaError::Other(format!("UTF-8: {e}")))?;
        crate::file_storage::from_auto(&content)
    }

    /// Save a todo file to S3.
    ///
    /// # Errors
    /// Returns an error if serialization or upload fails.
    pub fn save_todo(&self, bucket: &str, key: &str, file: &TodoFile) -> Result<()> {
        let json = serde_json::to_string_pretty(file)?;
        self.write_bytes(bucket, key, json.into_bytes())
    }
}

/// Implement `TodoStorage` for an S3 bucket + prefix.
pub struct S3BucketStorage {
    s3: S3Storage,
    bucket: String,
    prefix: String,
}

const S3_EXT: &str = ".todo.json";

impl S3BucketStorage {
    /// Create storage for a specific bucket and prefix.
    ///
    /// # Errors
    /// Returns an error if the S3 client cannot be created.
    pub fn new(bucket: &str, prefix: &str) -> Result<Self> {
        Ok(Self {
            s3: S3Storage::new()?,
            bucket: bucket.to_owned(),
            prefix: prefix.to_owned(),
        })
    }

    fn full_key(&self, name: &str) -> String {
        format!("{}{name}{S3_EXT}", self.prefix)
    }
}

impl TodoStorage for S3BucketStorage {
    fn list(&self) -> Result<Vec<String>> {
        let objects = self.s3.list_objects(&self.bucket, &self.prefix)?;
        let names: Vec<String> = objects
            .into_iter()
            .filter_map(|path| {
                let key = path.strip_prefix(&format!("s3://{}/", self.bucket))?;
                let name = key.strip_prefix(&self.prefix)?.strip_suffix(S3_EXT)?;
                Some(name.to_owned())
            })
            .collect();
        Ok(names)
    }

    fn load(&self, name: &str) -> Result<TodoFile> {
        self.s3.load_todo(&self.bucket, &self.full_key(name))
    }

    fn save(&self, name: &str, file: &TodoFile) -> Result<()> {
        self.s3.save_todo(&self.bucket, &self.full_key(name), file)
    }

    fn delete(&self, name: &str) -> Result<()> {
        self.s3.rt.block_on(async {
            self.s3
                .client
                .delete_object()
                .bucket(&self.bucket)
                .key(self.full_key(name))
                .send()
                .await
                .map_err(|e| OmelaError::Other(format!("S3 delete: {e}")))?;
            Ok(())
        })
    }

    fn exists(&self, name: &str) -> Result<bool> {
        self.s3.rt.block_on(async {
            match self
                .s3
                .client
                .head_object()
                .bucket(&self.bucket)
                .key(self.full_key(name))
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        })
    }
}
