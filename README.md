# appentry

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Fappentry%2Fmain%2F/appentry/Cargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/appentry/rust.yml)

A minimalist command-line argument parsing library

# Examples

Hello World:

```rust
#[appentry::appentry]
fn add(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    appentry::dispatch()?;
    Ok(())
}

/*
$ myapp --help
Usage: appentry.exe [Options]
Options:
    -x, --x <i32>
    -y, --y <i32>

$ myapp -x 1 -y 2
3
*/
```

Multiple methods and docs:

```rust
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
#[appentry::appentry(["-a", "--add"])]
fn add(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    appentry::dispatch()?;
    Ok(())
}

/*
$ myapp --help
Desc:  Add two numbers
Usage: appentry.exe -a/--add [Options]
Options:
    -x, --x <i32> The first number to add
    -y, --y <i32>

Desc:  Print the application version
Usage: appentry.exe -v/--version

$ myapp --version
myapp 0.1.0
*/
```
