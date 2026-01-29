pub use appentry_derive::appentry;
pub use inventory;

use std::collections::HashMap;

#[derive(Copy, Clone)]
pub struct FunctionInfo {
    pub name: &'static str,
    pub alias: &'static [&'static str],
    pub desc: Option<&'static str>,
    pub args: &'static [ArgInfo],
    pub method: fn(&mut HashMap<String, Option<String>>) -> anyhow::Result<()>,
}

impl FunctionInfo {
    pub const fn new(
        name: &'static str,
        alias: &'static [&'static str],
        args: &'static [ArgInfo],
        method: fn(&mut HashMap<String, Option<String>>) -> anyhow::Result<()>,
    ) -> Self {
        Self {
            name,
            alias,
            desc: None,
            args,
            method,
        }
    }

    pub const fn new_with_desc(
        name: &'static str,
        alias: &'static [&'static str],
        desc: Option<&'static str>,
        args: &'static [ArgInfo],
        method: fn(&mut HashMap<String, Option<String>>) -> anyhow::Result<()>,
    ) -> Self {
        Self {
            name,
            alias,
            desc,
            args,
            method,
        }
    }

    pub fn invoke(&self, args: &mut HashMap<String, Option<String>>) -> anyhow::Result<()> {
        (self.method)(args)
    }
}

/// A struct to hold argument metadata
#[derive(Copy, Clone)]
pub struct ArgInfo {
    pub name: &'static str,
    pub type_name: &'static str,
    pub desc: Option<&'static str>,
}

impl ArgInfo {
    pub const fn new(name: &'static str, type_name: &'static str) -> Self {
        Self {
            name,
            type_name,
            desc: None,
        }
    }

    pub const fn new_with_desc(
        name: &'static str,
        type_name: &'static str,
        desc: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            type_name,
            desc,
        }
    }
}

inventory::collect!(FunctionInfo);

pub fn get_arg_from_name<T: std::str::FromStr + Default + 'static>(
    args: &mut HashMap<String, Option<String>>,
    names: &[&str],
) -> T {
    let mut val = None;
    for name in names {
        if let Some(val1) = args.remove(*name) {
            val = val1;
        }
    }
    let is_bool = std::any::TypeId::of::<T>() == std::any::TypeId::of::<bool>();
    match val {
        Some(val) => match val.parse::<T>() {
            Ok(arg) => return arg,
            Err(_) => match is_bool {
                true => "true".parse::<T>().unwrap_or_default(),
                false => T::default(),
            },
        },
        None => match is_bool {
            true => "true".parse::<T>().unwrap_or_default(),
            false => T::default(),
        },
    }
}

pub fn appentry_help(arg0: &str, methods: &Vec<&FunctionInfo>) {
    let arg0 = match (arg0.rfind('/'), arg0.rfind('\\')) {
        (Some(a), None) => &arg0[a + 1..],
        (None, Some(b)) => &arg0[b + 1..],
        (Some(a), Some(b)) => &arg0[a.max(b) + 1..],
        (None, None) => arg0,
    };
    let width = methods
        .iter()
        .map(|m| {
            m.args
                .iter()
                .map(|arg| {
                    let lcname = arg.name.to_lowercase();
                    let lcch = arg.name.chars().next().unwrap();
                    let lctyname = arg.type_name.to_lowercase();
                    format!("    -{lcch}, --{lcname} <{lctyname}>").len()
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    match methods.len() {
        0 => panic!("You should define #[appentry] macro in entry function"),
        1 => {
            let method = methods[0];
            if let Some(desc) = method.desc {
                println!("Desc:  {desc}");
            }
            if method.args.is_empty() {
                println!("Usage: {arg0}");
            } else {
                println!("Usage: {arg0} [Options]");
                println!("Options:");
                for arg in method.args.iter() {
                    let lcname = arg.name.to_lowercase();
                    let lcch = arg.name.chars().next().unwrap();
                    let lctyname = arg.type_name.to_lowercase();
                    let base = format!("    -{lcch}, --{lcname} <{lctyname}>");
                    if let Some(desc) = arg.desc {
                        println!("{base:width$} {desc}");
                    } else {
                        println!("{base}");
                    }
                }
            }
            println!();
        }
        _ => {
            for method in methods {
                let name = match method.alias.is_empty() {
                    true => method.name.to_string(),
                    false => method.alias.join("/"),
                };
                if let Some(desc) = method.desc {
                    println!("Desc:  {desc}");
                }
                if method.args.is_empty() {
                    println!("Usage: {arg0} {name}");
                } else {
                    println!("Usage: {arg0} {name} [Options]");
                    println!("Options:");
                    for arg in method.args.iter() {
                        let lcname = arg.name.to_lowercase();
                        let lcch = arg.name.chars().next().unwrap();
                        let lctyname = arg.type_name.to_lowercase();
                        let base = format!("    -{lcch}, --{lcname} <{lctyname}>");
                        if let Some(desc) = arg.desc {
                            println!("{base:width$} {desc}");
                        } else {
                            println!("{base}");
                        }
                    }
                }
                println!();
            }
        }
    }
}

pub fn dispatch() -> anyhow::Result<()> {
    let mut methods: Vec<_> = inventory::iter::<FunctionInfo>.into_iter().collect();
    let mut args: Vec<String> = std::env::args().collect();
    let arg0 = args.remove(0);
    if let Some(arg) = args.first()
        && arg == "--help"
    {
        appentry_help(&arg0, &methods);
        return Ok(());
    }

    let method = match methods.len() {
        0 => panic!("You should define #[appentry] macro in entry function"),
        1 => methods.remove(0),
        _ => {
            let name = args.remove(0);
            let mut item = None;
            for method in methods.iter() {
                if method.alias.contains(&name.as_str())
                    || (method.alias.is_empty() && method.name == name)
                {
                    item = Some(method);
                    break;
                }
            }
            match item {
                Some(item) => item,
                None => {
                    appentry_help(&arg0, &methods);
                    return Ok(());
                }
            }
        }
    };

    let mut arg_vals = HashMap::new();
    while !args.is_empty() {
        let arg = args.remove(0);
        let arg_val = match args.first() {
            Some(v) if !v.starts_with('-') => Some(args.remove(0)),
            _ => None,
        };
        arg_vals.insert(arg, arg_val);
    }

    method.invoke(&mut arg_vals)?;

    Ok(())
}
