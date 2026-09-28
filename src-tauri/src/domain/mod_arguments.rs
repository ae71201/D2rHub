//! Mod names and Windows launch-argument transformations.
use std::path::{Component, Path};

pub(crate) fn plain_mod_name(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Mod 名称不能为空".to_string());
    }
    let path = Path::new(value);
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err("Mod 名称不能包含目录路径".to_string());
    }
    Ok(value)
}

pub(crate) fn generated_audio_mod_name(value: &str) -> Result<&str, String> {
    let value = plain_mod_name(value)?;
    if value.len() > 128 {
        return Err("Mod 名称不能超过 128 个字符".to_string());
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Mod 名称仅可使用英文字母、数字、短横线和下划线".to_string());
    }
    let uppercase = value.to_ascii_uppercase();
    if matches!(uppercase.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || uppercase
            .strip_prefix("COM")
            .or_else(|| uppercase.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
    {
        return Err("该名称是 Windows 保留名称，请换一个".to_string());
    }
    Ok(value)
}

pub(crate) fn active_mod_name(mod_args: &str) -> Result<Option<String>, String> {
    let args = parse_windows_command_line(mod_args)
        .map_err(|error| format!("无法解析账号启动参数: {error}"))?;
    let mut index = 0usize;
    while index < args.len() {
        let argument = &args[index];
        if argument.eq_ignore_ascii_case("-mod") {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "-mod 后缺少 Mod 名称".to_string())?;
            return Ok(Some(plain_mod_name(value)?.to_string()));
        }
        if argument
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("-mod="))
        {
            return Ok(Some(plain_mod_name(&argument[5..])?.to_string()));
        }
        index += 1;
    }
    Ok(None)
}

pub(crate) fn has_txt_argument(mod_args: &str) -> Result<bool, String> {
    Ok(parse_windows_command_line(mod_args)
        .map_err(|error| format!("无法解析账号启动参数: {error}"))?
        .iter()
        .any(|argument| argument.eq_ignore_ascii_case("-txt")))
}

fn quote_windows_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument
            .chars()
            .any(|character| character.is_whitespace() || character == '"')
    {
        return argument.to_string();
    }
    let mut output = String::from("\"");
    let mut slashes = 0usize;
    for character in argument.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        if character == '"' {
            output.push_str(&"\\".repeat(slashes * 2 + 1));
            output.push('"');
        } else {
            output.push_str(&"\\".repeat(slashes));
            output.push(character);
        }
        slashes = 0;
    }
    output.push_str(&"\\".repeat(slashes * 2));
    output.push('"');
    output
}

pub(crate) fn arguments_with_audio_mod(
    existing_arguments: &str,
    mod_name: &str,
) -> Result<String, String> {
    let mod_name = plain_mod_name(mod_name)?;
    let arguments = parse_windows_command_line(existing_arguments)
        .map_err(|error| format!("无法解析原启动参数: {error}"))?;
    let mut preserved = Vec::new();
    let mut index = 0usize;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument.eq_ignore_ascii_case("-mod") {
            index += 2;
            continue;
        }
        if argument
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("-mod="))
            || argument.eq_ignore_ascii_case("-txt")
        {
            index += 1;
            continue;
        }
        if argument.eq_ignore_ascii_case("-assettestmode") {
            index += 1;
            if arguments
                .get(index)
                .is_some_and(|value| !value.starts_with('-'))
            {
                index += 1;
            }
            continue;
        }
        if argument
            .get(..15)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("-assettestmode="))
        {
            index += 1;
            continue;
        }
        preserved.push(argument.clone());
        index += 1;
    }
    preserved.push("-mod".to_string());
    preserved.push(mod_name.to_string());
    preserved.push("-txt".to_string());
    preserved.push("-assettestmode".to_string());
    preserved.push("1".to_string());
    Ok(preserved
        .iter()
        .map(|argument| quote_windows_argument(argument))
        .collect::<Vec<_>>()
        .join(" "))
}

/// Parse a Windows command-line fragment into arguments without losing quoted spaces.
/// Implements the backslash-before-quote rules used by the Microsoft C runtime.
pub(crate) fn parse_windows_command_line(input: &str) -> Result<Vec<String>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut args = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index == chars.len() {
            break;
        }

        let mut argument = String::new();
        let mut in_quotes = false;
        let mut started = false;
        while index < chars.len() {
            let current = chars[index];
            if current.is_whitespace() && !in_quotes {
                break;
            }
            if current == '\\' {
                let slash_start = index;
                while index < chars.len() && chars[index] == '\\' {
                    index += 1;
                }
                let slash_count = index - slash_start;
                if index < chars.len() && chars[index] == '"' {
                    argument.extend(std::iter::repeat_n('\\', slash_count / 2));
                    if slash_count % 2 == 0 {
                        in_quotes = !in_quotes;
                    } else {
                        argument.push('"');
                    }
                    started = true;
                    index += 1;
                } else {
                    argument.extend(std::iter::repeat_n('\\', slash_count));
                    started = true;
                }
                continue;
            }
            if current == '"' {
                in_quotes = !in_quotes;
                started = true;
                index += 1;
                continue;
            }
            argument.push(current);
            started = true;
            index += 1;
        }

        if in_quotes {
            return Err("Mod 启动参数包含未闭合的双引号".to_string());
        }
        if started {
            args.push(argument);
        }
    }

    Ok(args)
}
