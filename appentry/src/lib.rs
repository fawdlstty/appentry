pub use appentry_derive::appentry;
pub use inventory;

use core::panic;
use std::{collections::HashMap, pin::Pin};

pub type SyncMethod = fn(&mut HashMap<String, Option<String>>) -> anyhow::Result<()>;
pub type AsyncMethod =
    fn(&mut HashMap<String, Option<String>>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>>;

#[derive(Copy, Clone)]
pub enum AppEntryMethod {
    Sync(SyncMethod),
    Async(AsyncMethod),
}

#[derive(Copy, Clone)]
pub struct FunctionInfo {
    pub name: &'static str,
    pub is_default: bool,
    pub desc: Option<&'static str>,
    pub args: &'static [ArgInfo],
    pub method: AppEntryMethod,
}

impl FunctionInfo {
    pub const fn new(
        name: &'static str,
        is_default: bool,
        desc: Option<&'static str>,
        args: &'static [ArgInfo],
        method: AppEntryMethod,
    ) -> Self {
        Self {
            name,
            is_default,
            desc,
            args,
            method,
        }
    }

    pub fn invoke(&self, args: &mut HashMap<String, Option<String>>) -> anyhow::Result<()> {
        match self.method {
            AppEntryMethod::Sync(method) => method(args),
            AppEntryMethod::Async(_) => panic!("Cannot invoke async method in sync context"),
        }
    }

    pub async fn invoke_async(
        &self,
        args: &mut HashMap<String, Option<String>>,
    ) -> anyhow::Result<()> {
        match self.method {
            AppEntryMethod::Sync(method) => method(args),
            AppEntryMethod::Async(method) => method(args).await,
        }
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

pub fn appentry_help(arg0: &str, methods: &Vec<&FunctionInfo>, enable_short: bool) -> ! {
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
                    let lctyname = arg.type_name.to_lowercase();
                    let lcch = arg.name.chars().next().unwrap();
                    match enable_short {
                        true => format!("    -{lcch}|--{lcname} <{lctyname}>").len(),
                        false => format!("    --{lcname} <{lctyname}>").len(),
                    }
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    for method in methods {
        if let Some(desc) = method.desc {
            println!("Desc:  {desc}");
        }
        let lcch = method.name.chars().next().unwrap();
        let method_name = {
            let method_name = match enable_short {
                true => format!("-{lcch}|--{}", method.name),
                false => format!("--{}", method.name),
            };
            match method.is_default {
                true => format!("[{method_name}]"),
                false => method_name,
            }
        };
        if method.args.is_empty() {
            println!("Usage: {arg0} {method_name}");
        } else {
            println!("Usage: {arg0} {method_name} [Options]");
            println!("Options:");
            for arg in method.args.iter() {
                let lcname = arg.name.to_lowercase();
                let lcch = arg.name.chars().next().unwrap();
                let lctyname = arg.type_name.to_lowercase();
                let base = match enable_short {
                    true => format!("    -{lcch}|--{lcname} <{lctyname}>"),
                    false => format!("    --{lcname} <{lctyname}>"),
                };
                if let Some(desc) = arg.desc {
                    println!("{base:width$} {desc}");
                } else {
                    println!("{base}");
                }
            }
        }
        println!();
    }
    std::process::exit(0);
}

pub fn dispatch(enable_short: bool) -> anyhow::Result<()> {
    let methods: Vec<_> = inventory::iter::<FunctionInfo>.into_iter().collect();
    if methods.is_empty() {
        panic!("You should define #[appentry] macro in entry function");
    }

    let mut args: Vec<String> = std::env::args().collect();
    let arg0 = args.remove(0);
    if let Some(arg) = args.first()
        && arg == "--help"
    {
        appentry_help(&arg0, &methods, enable_short);
    }

    let method = methods.get_method(enable_short, &arg0, &mut args);

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

pub async fn dispatch_async(enable_short: bool) -> anyhow::Result<()> {
    let methods: Vec<_> = inventory::iter::<FunctionInfo>.into_iter().collect();
    if methods.is_empty() {
        panic!("You should define #[appentry] macro in entry function");
    }

    let mut args: Vec<String> = std::env::args().collect();
    let arg0 = args.remove(0);
    if let Some(arg) = args.first()
        && arg == "--help"
    {
        appentry_help(&arg0, &methods, enable_short);
    }

    let method = methods.get_method(enable_short, &arg0, &mut args);
    let mut arg_vals = HashMap::new();
    while !args.is_empty() {
        let arg = args.remove(0);
        let arg_val = match args.first() {
            Some(v) if !v.starts_with('-') => Some(args.remove(0)),
            _ => None,
        };
        arg_vals.insert(arg, arg_val);
    }

    method.invoke_async(&mut arg_vals).await?;
    Ok(())
}

pub trait VecFunctionInfoExt {
    fn get_method(&self, enable_short: bool, arg0: &str, args: &mut Vec<String>) -> &FunctionInfo;
}
impl VecFunctionInfoExt for Vec<&FunctionInfo> {
    fn get_method(&self, enable_short: bool, arg0: &str, args: &mut Vec<String>) -> &FunctionInfo {
        if let Some(name) = args.first() {
            let mut item = None;
            for method in self.iter() {
                if (&name[..2] == "--" && &name[2..] == method.name)
                    || (enable_short && &name[..1] == "-" && &name[1..] == &method.name[..1])
                {
                    item = Some(method);
                    break;
                }
            }
            if let Some(item) = item {
                args.remove(0);
                return item;
            }
        }
        let mut defaults = vec![];
        for method in self.iter() {
            if method.is_default {
                defaults.push(method);
            }
        }
        match defaults.len() {
            0 => appentry_help(&arg0, &self, enable_short),
            1 => defaults.remove(0),
            _ => panic!("Multiple default methods is not allowed"),
        }
    }
}
