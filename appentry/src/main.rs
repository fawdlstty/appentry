/// Print the application version
///
/// # Arguments
///
/// This function takes no arguments
#[appentry::appentry(["-v", "--version"])]
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
#[appentry::appentry(["-p", "--plus"])]
fn plus(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    appentry::appentry_dispatch()?;

    Ok(())
}
