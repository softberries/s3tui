use std::path::Path;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region};
use crate::model::s3_data_item::S3DataItem;

#[derive(Clone)]
pub struct S3DataFetcher {
    credentials: Credentials,
}

impl S3DataFetcher {
    pub fn new() -> Self {
        let access_key = "YOUR_ACCESS_KEY";
        let secret_access_key = "YOUR_SECRET_KEY";
        let credentials = Credentials::new(
            access_key,
            secret_access_key,
            None, // Token, if using temporary credentials (like STS)
            None, // Expiry time, if applicable
            "manual", // Source, just a label for debugging
        );
        S3DataFetcher { credentials }
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
        let client = self.get_s3_client().await;
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
                        all_objects.push(S3DataItem::init(key.to_string(), size, file_extension, "", false, false));
                    }
                    for object in output.common_prefixes() {
                        let key = object.prefix().unwrap_or_default();
                        all_objects.push(S3DataItem::init(key.to_string(), "".to_string(), "Dir", key, true, false));
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
        let client = self.get_s3_client().await;
        let res = client.list_buckets().send().await;
        let fetched_data: Vec<S3DataItem>;
        match res {
            Ok(res) => {
                fetched_data = res.buckets.as_ref().map_or_else(
                    Vec::new, // In case there is no buckets field (it's None), return an empty Vec
                    |buckets| {
                        buckets.iter().filter_map(|bucket| {
                            // Filter out buckets where name is None, and map those with a name to a Vec<String>
                            //vec![name.clone()]
                            bucket.name.as_ref().map(|name| S3DataItem::init(name.clone(), "".to_string(), "Bucket", name, false, true))
                        }).collect()
                    },
                )
            }
            _ => {
                fetched_data = vec![]
            }
        }
        Ok(fetched_data)
    }

    async fn get_s3_client(&self) -> Client {
        let credentials = self.credentials.clone();
        let region_provider = RegionProviderChain::first_try(Region::new("eu-west-1"))
            .or_default_provider()
            .or_else(Region::new("eu-north-1"));
        let shared_config = aws_config::from_env().credentials_provider(credentials).region(region_provider).load().await;
        let client = Client::new(&shared_config);
        client
    }
}
