use std::fs::File;
use std::io::Write;
use std::path::{Path};
use aws_sdk_s3::config::{Credentials, Region};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::settings::file_credentials::FileCredential;
use aws_smithy_runtime_api::http::Request;
use std::{
    convert::Infallible,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    primitives::{ByteStream, SdkBody},
    Client,
};
use bytes::Bytes;
use http_body::{Body, SizeHint};
use crate::model::progress_item::ProgressItem;

#[derive(Clone)]
pub struct S3DataFetcher {
    default_region: String,
    credentials: Credentials,
}

struct ProgressTracker {
    bytes_written: u64,
    content_length: u64,
    progress_sender: UnboundedSender<ProgressItem>,
    uri: String,
}

impl ProgressTracker {
    fn track(&mut self, len: u64) {
        self.bytes_written += len;
        let progress = self.bytes_written as f64 / self.content_length as f64;
        let progress_item = ProgressItem {
            progress: progress * 100.0,
            uri: self.uri.clone(),
        };
        let _ = self.progress_sender.send(progress_item);
    }
}

#[pin_project::pin_project]
pub struct ProgressBody<InnerBody> {
    #[pin]
    inner: InnerBody,
    // progress_tracker is a separate field, so it can be accessed as &mut.
    progress_tracker: ProgressTracker,
}

// For an SdkBody specifically, the ProgressTracker swap itself in-place while customizing the SDK operation.
impl ProgressBody<SdkBody> {
    // Wrap a Requests SdkBody with a new ProgressBody, and replace it on the fly.
    // This is specialized for SdkBody specifically, as SdkBody provides ::taken() to
    // swap out the current body for a fresh, empty body and then provides ::from_dyn()
    // to get an SdkBody back from the ProgressBody it created. http::Body does not have
    // this "change the wheels on the fly" utility.
    pub fn replace(value: Request<SdkBody>, tx: UnboundedSender<ProgressItem>) -> Result<Request<SdkBody>, Infallible> {
        let uri = value.uri().to_string();
        let value = value.map(|body| {
            let len = body.content_length().expect("upload body sized");
            let cloned_uri = uri.clone();
            let body = ProgressBody::new(body, len, cloned_uri, tx.clone());
            SdkBody::from_body_0_4(body)
        });
        Ok(value)
    }
}

impl<InnerBody> ProgressBody<InnerBody>
    where
        InnerBody: Body<Data=Bytes, Error=aws_smithy_types::body::Error>,
{
    pub fn new(body: InnerBody, content_length: u64, uri: String, tx: UnboundedSender<ProgressItem>) -> Self {
        Self {
            inner: body,
            progress_tracker: ProgressTracker {
                bytes_written: 0,
                content_length,
                progress_sender: tx,
                uri: uri.to_string(),
            },
        }
    }
}

impl<InnerBody> Body for ProgressBody<InnerBody>
    where
        InnerBody: Body<Data=Bytes, Error=aws_smithy_types::body::Error>,
{
    type Data = Bytes;

    type Error = aws_smithy_types::body::Error;

    // Our poll_data delegates to the inner poll_data, but needs a project() to
    // get there. When the poll has data, it updates the progress_tracker.
    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.project();
        match this.inner.poll_data(cx) {
            Poll::Ready(Some(Ok(data))) => {
                this.progress_tracker.track(data.len() as u64);
                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(None) => {
                tracing::debug!("done");
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    // Delegate utilities to inner and progress_tracker.
    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        self.project().inner.poll_trailers(cx)
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.progress_tracker.content_length)
    }
}
/*
- Handle buckets from different regions
- fix upload/download functions to handled dirs/buckets
- report progress on upload/download operations
- add create/delete buckets
- add copy files within a bucket
 */

impl S3DataFetcher {
    pub fn new(creds: FileCredential) -> Self {
        let access_key = creds.access_key;
        let secret_access_key = creds.secret_key;
        let default_region = creds.default_region;
        let credentials = Credentials::new(
            access_key,
            secret_access_key,
            None, // Token, if using temporary credentials (like STS)
            None, // Expiry time, if applicable
            "manual", // Source, just a label for debugging
        );
        S3DataFetcher { default_region, credentials }
    }

    /*
    this function handles only simple files as of now.
    - not tested with larger ones,
    - not sure when and if necessary to use multipart uploads,
    - no progress reported to transfers page
    - no directory handling
     */
    pub async fn upload_item(&self, item: LocalSelectedItem, upload_tx: UnboundedSender<ProgressItem>) -> anyhow::Result<bool> {
        let client = self.get_s3_client(Some(item.s3_creds)).await;
        let body = ByteStream::read_from()
            .path(item.path)
            // https://github.com/awslabs/aws-sdk-rust/blob/main/examples/examples/s3/src/bin/put-object-progress.rs
            // Artificially limit the buffer size to ensure the file has multiple
            // progress steps.
            // .buffer_size(2048)
            .build()
            .await?;

        let request = client
            .put_object()
            .bucket(item.destination_bucket)
            .key(item.name)
            .body(body);

        let customized = request
            .customize()
            .map_request(move |req| ProgressBody::<SdkBody>::replace(req, upload_tx.clone()));

        let _out = customized.send().await?;

        Ok(true)
    }

    /*
    this function handles only simple files as of now.
    - not tested with larger files,
    - no progress reported to transfers page
    - no directory or full bucket handling
    */
    pub async fn download_item(&self, item: S3SelectedItem) -> anyhow::Result<bool> {
        let client = self.get_s3_client(Some(item.s3_creds)).await;
        let mut path = PathBuf::from(item.destination_dir);
        path.push(item.name.clone());
        let mut file = File::create(&path)?;

        let mut object = client
            .get_object()
            .bucket(item.bucket.expect("bucket must be defined").clone())
            .key(item.name.clone())
            .send()
            .await?;

        let mut _byte_count = 0_usize;
        while let Some(bytes) = object.body.try_next().await? {
            let bytes_len = bytes.len();
            file.write_all(&bytes)?;
            // trace!("Intermediate write of {bytes_len}");
            _byte_count += bytes_len;
        }
        Ok(true)
    }

    pub async fn list_current_location(&self, bucket: Option<String>, prefix: Option<String>) -> anyhow::Result<Vec<S3DataItem>> {
        match (bucket, prefix) {
            (None, None) => self.list_buckets().await,
            (Some(bucket), None) => self.list_objects(bucket.as_str(), None).await,
            (Some(bucket), Some(prefix)) => self.list_objects(bucket.as_str(), Some(prefix)).await,
            _ => self.list_buckets().await
        }
    }
    async fn list_objects(&self, bucket: &str, prefix: Option<String>) -> anyhow::Result<Vec<S3DataItem>> {
        let client = self.get_s3_client(None).await;
        let mut all_objects = Vec::new();
        let mut response = client
            .list_objects_v2()
            .delimiter("/")
            .set_prefix(prefix)
            .bucket(bucket.to_owned())
            .max_keys(10) // In this example, go 10 at a time.
            .into_paginator()
            .send();

        while let Some(result) = response.next().await {
            match result {
                Ok(output) => {
                    for object in output.contents() {
                        let key = object.key().unwrap_or_default();
                        //todo: get size of the file
                        // all_objects.push(vec![key.to_string()]);
                        let size = object.size().map_or(String::new(), |value| value.to_string());
                        let path = Path::new(key);
                        let file_extension = path.extension()
                            .and_then(|ext| ext.to_str()) // Convert the OsStr to a &str
                            .unwrap_or("");
                        all_objects.push(S3DataItem::init(Some(bucket.to_string()), key.to_string(), size, file_extension, "", false, false));
                    }
                    for object in output.common_prefixes() {
                        let key = object.prefix().unwrap_or_default();
                        all_objects.push(S3DataItem::init(Some(bucket.to_string()), key.to_string(), "".to_string(), "Dir", key, true, false));
                    }
                }
                Err(err) => {
                    eprintln!("Err: {:?}", err) // Return the error immediately if encountered
                }
            }
        }

        Ok(all_objects)
    }

    // Example async method to fetch data from an external service
    async fn list_buckets(&self) -> anyhow::Result<Vec<S3DataItem>> {
        let client = self.get_s3_client(None).await;
        let mut fetched_data: Vec<S3DataItem> = vec![];
        if let Ok(res) = client.list_buckets().send().await {
            fetched_data = res.buckets.as_ref().map_or_else(
                Vec::new, // In case there is no buckets field (it's None), return an empty Vec
                |buckets| {
                    buckets.iter().filter_map(|bucket| {
                        // Filter out buckets where name is None, and map those with a name to a Vec<String>
                        //vec![name.clone()]
                        bucket.name.as_ref().map(|name| S3DataItem::init(None, name.clone(), "".to_string(), "Bucket", name, false, true))
                    }).collect()
                },
            )
        }
        Ok(fetched_data)
    }

    async fn get_s3_client(&self, creds: Option<FileCredential>) -> Client {
        let credentials: Credentials;
        let default_region: String;
        if let Some(crd) = creds {
            let access_key = crd.access_key;
            let secret_access_key = crd.secret_key;
            default_region = crd.default_region;
            credentials = Credentials::new(
                access_key,
                secret_access_key,
                None, // Token, if using temporary credentials (like STS)
                None, // Expiry time, if applicable
                "manual", // Source, just a label for debugging
            );
        } else {
            credentials = self.credentials.clone();
            default_region = self.default_region.clone();
        }
        let region_provider = RegionProviderChain::first_try(Region::new(default_region))
            .or_default_provider()
            .or_else(Region::new("eu-north-1"));
        let shared_config = aws_config::from_env().credentials_provider(credentials).region(region_provider).load().await;
        Client::new(&shared_config)
    }
}
