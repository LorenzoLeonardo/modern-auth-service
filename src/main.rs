mod oauth2;
mod shared_object;

use oauth2::error::OAuth2Result;

#[tokio::main(flavor = "current_thread")]
async fn main() -> OAuth2Result<()> {
    Ok(())
}
