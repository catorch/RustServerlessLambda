use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: String,
    user_id: String,
    document_name: String,
    
}
/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let who = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("name"))
        .unwrap_or("world");
    let message = format!("Hello {who}, this is an AWS Lambda HTTP request");

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

async fn fetch_documents_from_signnow(template_id: &str, access_token: &str) -> Result<Vec<Document>, Error> {
    let url = format!("https://api.signnow.com/v2/templates/{}/copies", template_id);
    let params = [("sort_by", "created"), ("sort_order", "desc"), ("per_page", "20")];

    let client = reqwest::Client::new();
    let response = client.get(url)
        .bearer_auth(access_token)
        .query(&params)
        .send()
        .await?;

    let documents: Vec<Document> = response.json().await.map_err(Box::new)?;
    Ok(documents)
}

async fn lambda_handler(event: Request) -> Result<Response<Body>, Error> {
    // temporal template_id and access_token
    let template_id = "template_id";
    let access_token = "access_token";

    match fetch_documents_from_signnow(template_id, access_token).await {
        Ok(documents) => {
            let body = json!(documents).to_string();
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(body.into())
                .expect("Failed to render response"))
        },
        Err(error) => {
            let error_message = format!("Error fetching documents: {}", error);
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