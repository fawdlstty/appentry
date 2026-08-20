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
    pub is_bare: bool,
    pub desc: Option<&'static str>,
    pub args: &'static [ArgInfo],
    pub method: AppEntryMethod,
}

impl FunctionInfo {
    pub const fn new(
        name: &'static str,
        is_default: bool,
        is_bare: bool,
        desc: Option<&'static str>,
        args: &'static [ArgInfo],
        method: AppEntryMethod,
    ) -> Self {
        Self {
            name,
            is_default,
            is_bare,
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
    let mut contains = false;
    let mut val = None;
    for name in names {
        if let Some(val1) = args.remove(*name) {
            val = val1;
            contains = true;
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
        None => match is_bool && contains {
            true => "true".parse::<T>().unwrap_or_default(),
            false => T::default(),
        },
    }
}

pub fn get_vec_arg_from_name(
    args: &mut HashMap<String, Option<String>>,
    names: &[&str],
) -> Vec<String> {
    for name in names {
        if let Some(Some(val)) = args.remove(*name) {
            if val.is_empty() {
                return Vec::new();
            }
            return val.split('\0').map(|arg| arg.to_string()).collect();
        }
    }
    Vec::new()
}

pub fn appentry_help(arg0: &str, methods: &Vec<&FunctionInfo>, enable_short: bool) -> ! {
    print_appentry_help(arg0, methods, enable_short);
    std::process::exit(0);
}

fn print_appentry_help(arg0: &str, methods: &Vec<&FunctionInfo>, enable_short: bool) {
    let arg0 = match (arg0.rfind('/'), arg0.rfind('\\')) {
        (Some(a), None) => &arg0[a + 1..],
        (None, Some(b)) => &arg0[b + 1..],
        (Some(a), Some(b)) => &arg0[a.max(b) + 1..],
        (None, None) => arg0,
    };
    let width = methods
        .iter()
        .filter(|m| !m.is_bare)
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
        if method.is_bare {
            let method_name = match method.is_default {
                true => format!("[{}]", method.name),
                false => method.name.to_string(),
            };
            let args = method
                .args
                .iter()
                .map(|arg| {
                    if is_vec_string_arg(arg) {
                        format!("<{}...>", arg.name.to_lowercase())
                    } else {
                        format!(
                            "<{}:{}>",
                            arg.name.to_lowercase(),
                            arg.type_name.to_lowercase()
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if args.is_empty() {
                println!("Usage: {arg0} {method_name}");
            } else {
                println!("Usage: {arg0} {method_name} {args}");
            }
        } else {
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
        }
        println!();
    }
}

fn parse_method_args(
    method: &FunctionInfo,
    raw_args: Vec<String>,
) -> anyhow::Result<HashMap<String, Option<String>>> {
    if method.is_bare {
        parse_bare_method_args(method, raw_args)
    } else {
        Ok(parse_flag_method_args(raw_args))
    }
}

fn parse_bare_method_args(
    method: &FunctionInfo,
    raw_args: Vec<String>,
) -> anyhow::Result<HashMap<String, Option<String>>> {
    if raw_args.iter().any(|arg| arg.starts_with('-')) {
        anyhow::bail!(
            "bare method '{}' does not accept flag arguments",
            method.name
        );
    }
    if let Some(varargs_index) = method.args.iter().position(is_vec_string_arg) {
        if varargs_index + 1 != method.args.len() {
            anyhow::bail!("Vec<String> must be the last bare argument");
        }
        if raw_args.len() < varargs_index {
            anyhow::bail!(
                "bare method '{}' expects at least {} arguments, got {}",
                method.name,
                varargs_index,
                raw_args.len()
            );
        }

        let mut arg_vals = HashMap::new();
        for (arg, value) in method.args[..varargs_index]
            .iter()
            .zip(raw_args.iter().take(varargs_index))
        {
            arg_vals.insert(format!("--{}", arg.name), Some(value.clone()));
        }
        let rest = raw_args
            .into_iter()
            .skip(varargs_index)
            .collect::<Vec<_>>()
            .join("\0");
        arg_vals.insert(format!("--{}", method.args[varargs_index].name), Some(rest));
        return Ok(arg_vals);
    }
    if raw_args.len() != method.args.len() {
        anyhow::bail!(
            "bare method '{}' expects {} arguments, got {}",
            method.name,
            method.args.len(),
            raw_args.len()
        );
    }

    let mut arg_vals = HashMap::new();
    for (arg, value) in method.args.iter().zip(raw_args) {
        arg_vals.insert(format!("--{}", arg.name), Some(value));
    }
    Ok(arg_vals)
}

fn is_vec_string_arg(arg: &ArgInfo) -> bool {
    matches!(
        arg.type_name.replace(' ', "").as_str(),
        "Vec<String>" | "std::vec::Vec<String>" | "::std::vec::Vec<String>"
    )
}

fn parse_flag_method_args(mut raw_args: Vec<String>) -> HashMap<String, Option<String>> {
    let mut arg_vals = HashMap::new();
    while !raw_args.is_empty() {
        let arg = raw_args.remove(0);
        let arg_val = match raw_args.first() {
            Some(v) if !v.starts_with('-') => Some(raw_args.remove(0)),
            _ => None,
        };
        arg_vals.insert(arg, arg_val);
    }
    arg_vals
}

fn print_appentry_error<T>(
    arg0: &str,
    methods: &Vec<&FunctionInfo>,
    enable_short: bool,
    err: anyhow::Error,
) -> anyhow::Result<T>
where
    T: Sized,
{
    eprintln!("{err}");
    print_appentry_help(arg0, methods, enable_short);
    Err(err)
}

fn dispatch_parts(
    enable_short: bool,
) -> anyhow::Result<(&'static FunctionInfo, HashMap<String, Option<String>>)> {
    let methods: Vec<_> = inventory::iter::<FunctionInfo>.into_iter().collect();
    if methods.is_empty() {
        panic!("You should define #[appentry] macro in entry function");
    }

    let mut args: Vec<String> = std::env::args().collect();
    let arg0 = args.remove(0);
    dispatch_parts_from_methods(enable_short, arg0, args, &methods)
}

fn dispatch_parts_from_methods(
    enable_short: bool,
    arg0: String,
    mut args: Vec<String>,
    methods: &Vec<&'static FunctionInfo>,
) -> anyhow::Result<(&'static FunctionInfo, HashMap<String, Option<String>>)> {
    if let Some(arg) = args.first()
        && arg == "--help"
    {
        appentry_help(&arg0, &methods, enable_short);
    }

    let method = match methods.get_method(enable_short, &mut args) {
        Ok(method) => method,
        Err(err) => return print_appentry_error(&arg0, &methods, enable_short, err),
    };
    let arg_vals = match parse_method_args(method, args) {
        Ok(arg_vals) => arg_vals,
        Err(err) => return print_appentry_error(&arg0, &methods, enable_short, err),
    };

    Ok((method, arg_vals))
}

pub fn dispatch(enable_short: bool) -> anyhow::Result<()> {
    let (method, mut arg_vals) = dispatch_parts(enable_short)?;
    method.invoke(&mut arg_vals)?;
    Ok(())
}

pub async fn dispatch_async(enable_short: bool) -> anyhow::Result<()> {
    let (method, mut arg_vals) = dispatch_parts(enable_short)?;
    method.invoke_async(&mut arg_vals).await?;
    Ok(())
}

pub trait VecFunctionInfoExt {
    fn get_method(
        &self,
        enable_short: bool,
        args: &mut Vec<String>,
    ) -> anyhow::Result<&'static FunctionInfo>;
}
impl VecFunctionInfoExt for Vec<&'static FunctionInfo> {
    fn get_method(
        &self,
        enable_short: bool,
        args: &mut Vec<String>,
    ) -> anyhow::Result<&'static FunctionInfo> {
        if let Some(name) = args.first() {
            let mut wrong_mode_match = false;
            for method in self.iter() {
                if method.is_bare {
                    if !method.is_default && name == method.name {
                        args.remove(0);
                        return Ok(method);
                    }
                    if method_matches_flag(method, enable_short, name) {
                        wrong_mode_match = true;
                    }
                } else {
                    if method_matches_flag(method, enable_short, name) {
                        args.remove(0);
                        return Ok(method);
                    }
                    if name == method.name {
                        wrong_mode_match = true;
                    }
                }
            }
            if wrong_mode_match {
                anyhow::bail!("invalid command form: {name}");
            }
        }
        let mut defaults = vec![];
        for method in self.iter() {
            if method.is_default {
                defaults.push(method);
            }
        }
        match defaults.len() {
            0 => anyhow::bail!("missing command"),
            1 => Ok(defaults.remove(0)),
            _ => panic!("Multiple default methods is not allowed"),
        }
    }
}

fn method_matches_flag(method: &FunctionInfo, enable_short: bool, name: &str) -> bool {
    if let Some(long_name) = name.strip_prefix("--") {
        return long_name == method.name;
    }
    if enable_short && let Some(short_name) = name.strip_prefix('-') {
        return method
            .name
            .chars()
            .next()
            .is_some_and(|ch| short_name == ch.to_string());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_: &mut HashMap<String, Option<String>>) -> anyhow::Result<()> {
        Ok(())
    }

    static BARE_ARGS: [ArgInfo; 2] = [ArgInfo::new("x", "i32"), ArgInfo::new("y", "i32")];
    static VARARGS_ARGS: [ArgInfo; 2] = [
        ArgInfo::new("command", "String"),
        ArgInfo::new("args", "Vec < String >"),
    ];
    static BARE_ADD: FunctionInfo = FunctionInfo::new(
        "add",
        false,
        true,
        None,
        &BARE_ARGS,
        AppEntryMethod::Sync(noop),
    );
    static FLAG_ADD: FunctionInfo = FunctionInfo::new(
        "add",
        false,
        false,
        None,
        &BARE_ARGS,
        AppEntryMethod::Sync(noop),
    );
    static DEFAULT: FunctionInfo =
        FunctionInfo::new("default", true, true, None, &[], AppEntryMethod::Sync(noop));
    static VARARGS: FunctionInfo = FunctionInfo::new(
        "run",
        false,
        true,
        None,
        &VARARGS_ARGS,
        AppEntryMethod::Sync(noop),
    );

    #[test]
    fn parse_bare_method_args_maps_positionals_to_long_names() {
        let args =
            parse_bare_method_args(&BARE_ADD, vec!["1".to_string(), "2".to_string()]).unwrap();

        assert_eq!(args.get("--x"), Some(&Some("1".to_string())));
        assert_eq!(args.get("--y"), Some(&Some("2".to_string())));
    }

    #[test]
    fn parse_bare_method_args_rejects_flags_and_wrong_counts() {
        assert!(
            parse_bare_method_args(&BARE_ADD, vec!["-x".to_string(), "1".to_string()]).is_err()
        );
        assert!(parse_bare_method_args(&BARE_ADD, vec!["1".to_string()]).is_err());
        assert!(
            parse_bare_method_args(
                &BARE_ADD,
                vec!["1".to_string(), "2".to_string(), "3".to_string()]
            )
            .is_err()
        );
    }

    #[test]
    fn parse_bare_method_args_captures_vec_string_tail() {
        let args = parse_bare_method_args(
            &VARARGS,
            vec!["show".to_string(), "gzgy".to_string(), "1~2".to_string()],
        )
        .unwrap();

        assert_eq!(args.get("--command"), Some(&Some("show".to_string())));
        assert_eq!(
            get_vec_arg_from_name(&mut args.clone(), &["--args"]),
            vec!["gzgy".to_string(), "1~2".to_string()]
        );
    }

    #[test]
    fn get_method_matches_bare_and_flag_forms_by_method_mode() {
        let methods = vec![&BARE_ADD, &FLAG_ADD];

        let mut args = vec!["add".to_string(), "1".to_string(), "2".to_string()];
        assert_eq!(methods.get_method(true, &mut args).unwrap().name, "add");
        assert_eq!(args, vec!["1".to_string(), "2".to_string()]);

        let mut args = vec!["--add".to_string(), "-x".to_string(), "1".to_string()];
        assert_eq!(methods.get_method(true, &mut args).unwrap().name, "add");
        assert_eq!(args, vec!["-x".to_string(), "1".to_string()]);
    }

    #[test]
    fn get_method_rejects_wrong_form_instead_of_falling_back_to_default() {
        let bare_methods = vec![&BARE_ADD, &DEFAULT];
        let mut args = vec!["--add".to_string(), "1".to_string(), "2".to_string()];
        assert!(bare_methods.get_method(true, &mut args).is_err());

        let flag_methods = vec![&FLAG_ADD, &DEFAULT];
        let mut args = vec!["add".to_string(), "1".to_string(), "2".to_string()];
        assert!(flag_methods.get_method(true, &mut args).is_err());
    }

    #[test]
    fn get_method_does_not_consume_default_bare_function_name() {
        let methods = vec![&DEFAULT];
        let mut args = vec!["default".to_string()];

        assert_eq!(methods.get_method(true, &mut args).unwrap().name, "default");
        assert_eq!(args, vec!["default".to_string()]);
    }
}
