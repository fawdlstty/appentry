/// Print the application version
///
/// # Arguments
///
/// This function takes no arguments
#[appentry::appentry]
fn version() {
    println!(
        "{} {}",
        std::env!("CARGO_PKG_NAME"),
        std::env!("CARGO_PKG_VERSION")
    );
}

/// Add two numbers
///
/// # Arguments
///
/// * `x` - The first number to add
/// * `y` -
#[appentry::appentry(default)]
async fn add(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    appentry::dispatch_async(true).await?;
    Ok(())
}
