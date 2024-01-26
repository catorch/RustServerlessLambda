use aws_config::meta::region::RegionProviderChain;
use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use aws_sdk_s3::{Client};
use aws_sdk_s3::primitives::ByteStream;
use reqwest::{Client as HttpClient, Error as ReqwestError};

#[derive(Serialize, Deserialize, Debug)]
struct SignNowDocumentsResponse {
    data: Vec<DocumentData>,    
}

#[derive(Serialize, Deserialize, Debug)]
struct DocumentData {
    id: String,
}

async fn upload_to_s3(bucket: &str, key: &str, data: Vec<u8>) -> Result<(), Error> {
    let region_provider = RegionProviderChain::default_provider()
        .or_else("us-west-1"); 

    let config = aws_config::from_env()
        .region(region_provider)
        .load()
        .await;

    let s3_client = Client::new(&config);

    let data_stream = ByteStream::from(data);
    s3_client.put_object()
        .bucket(bucket)
        .key(key)
        .body(data_stream)
        .send()
        .await?;

    Ok(())
}


async fn fetch_documents_from_signnow(template_id: &str, access_token: &str) -> Result<Vec<String>, Error> {
    let url = format!("https://stoplight.io/mocks/airslate/signnow/60489437/v2/templates/{}/copies", template_id);
    let params = [("sort_by", "created"), ("sort_order", "desc"), ("per_page", "20")];

    let client = reqwest::Client::new();
    let response = client.get(url)
        .bearer_auth(access_token)
        .query(&params)
        .send()
        .await?;

    let parsed_response: SignNowDocumentsResponse = response.json().await?;

    // Extract only the IDs from the document data
    let document_ids = parsed_response.data.into_iter().map(|doc| doc.id).collect();
    
    Ok(document_ids)
}


async fn download_document_from_signnow(document_id: &str, access_token: &str) -> Result<Vec<u8>, ReqwestError> {
    let download_url = format!("https://stoplight.io/mocks/airslate/signnow/60489437/document/{}/download?type=collapsed", document_id);

    let client = HttpClient::new();
    let response = client.get(download_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    let document_content = response.bytes().await?.to_vec();
    Ok(document_content)
}



async fn lambda_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract template_id and access_token from event or environment
    let template_id = "a9dcd5bf8b494bc2ba4ac4ba13a36157c677fc2f";
    let access_token = "MjdiNWM5NTE0NWE1NDAxMWQ0YTMwMTJlNGQ1MTA2ZDA6NmJmZjE0MDEwNTZiYjg4ZjFlNTA5ZjQ2OGFjYTRkYzY=";
    let bucket_name = "rust-test-catorch";

    match fetch_documents_from_signnow(template_id, access_token).await {
        Ok(document_ids) => {
            for document_id in document_ids {
                // Download each document
                match download_document_from_signnow(&document_id, access_token).await {
                    Ok(document_content) => {
                        // Construct a key for S3 
                        let s3_key = format!("documents/{}.pdf", document_id);

                        // Upload to S3
                        upload_to_s3(bucket_name, &s3_key, document_content).await?;
                    },
                    Err(error) => {
                        eprintln!("Failed to download document {}: {}", document_id, error);
                    }
                }
            }
            // Return a success response
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body("{\"message\": \"Documents processed\"}".into())
                .expect("Failed to render response"))
        },
        Err(error) => {
            let error_message = format!("Error fetching document IDs: {}", error);
            Ok(Response::builder()
                .status(500)
                .header("content-type", "text/plain")
                .body(error_message.into())
                .expect("Failed to render response"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(lambda_handler)).await
}