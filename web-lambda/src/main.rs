use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    Request, RequestExt,
};

use serde_json::json;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(handler(handle_request)).await?;

    Ok(())
}

async fn handle_request(request: Request, _: Context) -> Result<Value, Error> {
    Ok(json!(format!(
        "hello {}",
        request
            .query_string_parameters()
            .get("name")
            .unwrap_or_else(|| "stranger")
    )))
}
