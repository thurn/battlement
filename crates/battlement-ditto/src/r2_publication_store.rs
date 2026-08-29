//! Authenticated R2 baseline writes through the S3-compatible API.

use std::{fs, path::Path, time::Duration};

use anyhow::{Context, Result, ensure};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
  Client,
  config::{Config, Region},
  error::SdkError,
  primitives::ByteStream,
};
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use tokio::runtime::{Builder, Runtime};

use crate::{
  baseline_publication::{
    ConditionalMutation, ConditionalObjectStore, R2Credentials, StoredObject, upload_immutable,
  },
  baseline_store::BaselineStore,
  r2_baseline_store::R2BaselineStore,
};

const MAXIMUM_OBJECT_BYTES: i64 = 64 * 1024 * 1024;

/// A read/write R2 store whose writes use bucket-scoped credentials.
pub struct R2PublicationStore {
  bucket: String,
  client: Client,
  reader: R2BaselineStore,
  runtime: Runtime,
}

impl R2PublicationStore {
  /// Creates a store for one R2 bucket and its public read endpoint.
  pub fn new(
    credentials: R2Credentials,
    public_base_url: String,
    timeout: Duration,
  ) -> Result<Self> {
    let endpoint = format!(
      "https://{}.r2.cloudflarestorage.com",
      credentials.account_id()
    );
    let config = Config::builder()
      .behavior_version_latest()
      .region(Region::new("auto"))
      .endpoint_url(endpoint)
      .force_path_style(true)
      .credentials_provider(Credentials::new(
        credentials.access_key_id(),
        credentials.secret_access_key(),
        None,
        None,
        "ditto-r2",
      ))
      .build();
    Ok(Self {
      bucket: credentials.bucket().to_owned(),
      client: Client::from_conf(config),
      reader: R2BaselineStore::new(public_base_url, timeout),
      runtime: Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("create R2 runtime")?,
    })
  }

  fn put(
    &self,
    key: &str,
    condition: PutCondition<'_>,
    bytes: &[u8],
  ) -> Result<ConditionalMutation> {
    let mut request = self
      .client
      .put_object()
      .bucket(&self.bucket)
      .key(key)
      .body(ByteStream::from(bytes.to_vec()));
    request = match condition {
      PutCondition::Absent => request.if_none_match("*"),
      PutCondition::Match(etag) => request.if_match(etag),
    };
    match self.runtime.block_on(request.send()) {
      Ok(_) => Ok(ConditionalMutation::Applied),
      Err(error) if is_precondition(&error) => Ok(ConditionalMutation::PreconditionFailed),
      Err(error) => Err(error).context("write R2 object"),
    }
  }
}

impl ConditionalObjectStore for R2PublicationStore {
  fn get(&self, key: &str) -> Result<Option<StoredObject>> {
    let output = match self.runtime.block_on(
      self
        .client
        .get_object()
        .bucket(&self.bucket)
        .key(key)
        .send(),
    ) {
      Ok(output) => output,
      Err(error) if is_missing(&error) => return Ok(None),
      Err(error) => return Err(error).context("read R2 object"),
    };
    ensure!(
      output.content_length().unwrap_or_default() <= MAXIMUM_OBJECT_BYTES,
      "R2 object exceeds the size limit"
    );
    let etag = output
      .e_tag()
      .context("R2 object response omitted ETag")?
      .to_owned();
    let bytes = self
      .runtime
      .block_on(output.body.collect())
      .context("read R2 object body")?
      .into_bytes()
      .to_vec();
    ensure!(
      bytes.len() as i64 <= MAXIMUM_OBJECT_BYTES,
      "R2 object exceeds the size limit"
    );
    Ok(Some(StoredObject { bytes, etag }))
  }

  fn confirm(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    match self.runtime.block_on(
      self
        .client
        .head_object()
        .bucket(&self.bucket)
        .key(key)
        .if_match(etag)
        .send(),
    ) {
      Ok(_) => Ok(ConditionalMutation::Applied),
      Err(error) if is_precondition(&error) || is_missing(&error) => {
        Ok(ConditionalMutation::PreconditionFailed)
      }
      Err(error) => Err(error).context("confirm R2 object"),
    }
  }

  fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    self.put(key, PutCondition::Absent, bytes)
  }

  fn put_if_match(&self, key: &str, etag: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    self.put(key, PutCondition::Match(etag), bytes)
  }

  fn delete_if_match(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    match self.runtime.block_on(
      self
        .client
        .delete_object()
        .bucket(&self.bucket)
        .key(key)
        .if_match(etag)
        .send(),
    ) {
      Ok(_) => Ok(ConditionalMutation::Applied),
      Err(error) if is_precondition(&error) || is_missing(&error) => {
        Ok(ConditionalMutation::PreconditionFailed)
      }
      Err(error) => Err(error).context("conditionally delete R2 object"),
    }
  }

  fn delete(&self, key: &str) -> Result<()> {
    self.runtime.block_on(
      self
        .client
        .delete_object()
        .bucket(&self.bucket)
        .key(key)
        .send(),
    )?;
    Ok(())
  }
}

impl BaselineStore for R2PublicationStore {
  fn hydrate(
    &self,
    namespace: &str,
    sha256: &str,
    cache_root: &Path,
  ) -> Result<std::path::PathBuf> {
    self.reader.hydrate(namespace, sha256, cache_root)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    upload_immutable(self, namespace, sha256, &fs::read(source)?)
  }
}

enum PutCondition<'a> {
  Absent,
  Match(&'a str),
}

fn is_missing<E>(error: &SdkError<E, HttpResponse>) -> bool {
  error
    .raw_response()
    .is_some_and(|response| response.status().as_u16() == 404)
}

fn is_precondition<E>(error: &SdkError<E, HttpResponse>) -> bool {
  error
    .raw_response()
    .is_some_and(|response| response.status().as_u16() == 412)
}
