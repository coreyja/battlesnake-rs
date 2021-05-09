use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    Handler, IntoResponse, Request, RequestExt,
};

use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let state = std::sync::Arc::new("something".to_owned());

    lambda_runtime::run(handler(move |request: Request, _: Context| {
        let path_parts: Vec<&str> = request
            .uri()
            .path()
            .split("/")
            .filter(|x| x != &"")
            .collect();
        let snake_name = path_parts.get(0);

        let mine = state.clone();
        let v = json!({ "state": mine.as_ref(), "msg":
                   format!(
                       "hello {} you are asking for {}",
                       request
                           .query_string_parameters()
                           .get("name")
                           .unwrap_or_else(|| "stranger"),
                       snake_name.unwrap_or_else(|| &"404")
                   )
        });
        async { Ok(v) }
    }))
    .await?;

    Ok(())
}
