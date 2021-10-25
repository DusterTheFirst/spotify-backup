use lambda_runtime::{handler_fn, Context, Error};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(handler_fn(handler)).await?;
    Ok(())
}

async fn handler(event: Value, _ctx: Context) -> Result<Value, Error> {
    
    Ok(event)
}
