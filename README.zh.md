# appentry

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Fappentry%2Fmain%2F/appentry/Cargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/appentry/rust.yml)

[English](README.md) | 简体中文

极简的命令行参数解析库

# 文档

入门示例:

```rust
#[appentry::appentry]
fn numplus(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    appentry::appentry_dispatch()?;
    Ok(())
}

/*
$ myapp --help
Usage: target\debug\appentry.exe [Options]
Options:
    -x, --x <i32>
    -y, --y <i32>

$ myapp -x 1 -y 2
3
*/
```

多方法及文档说明示例:

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
#[appentry::appentry(["-p", "--plus"])]
fn plus(x: i32, y: i32) -> anyhow::Result<()> {
    println!("{}", x + y);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    appentry::appentry_dispatch()?;
    Ok(())
}

/*
$ myapp --help
Desc:  Add two numbers
Usage: target\debug\appentry.exe -p/--plus [Options]
Options:
    -x, --x <i32> The first number to add
    -y, --y <i32>

Desc:  Print the application version
Usage: target\debug\appentry.exe -v/--version

$ myapp --version
myapp 0.1.0
*/
```
