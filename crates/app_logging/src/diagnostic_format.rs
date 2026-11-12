use std::fmt::Write as _;
use std::{backtrace::Backtrace, fmt, panic::Location};

const SOURCE_COORDINATE_COMPONENTS: usize = 2;
const REGISTRY_PACKAGE_OFFSET: usize = 4;
const GIT_REPOSITORY_OFFSET: usize = 3;
const GIT_CHECKOUT_SOURCE_OFFSET: usize = 5;

pub fn safe_location<'a>(location: &'a Location<'a>) -> impl fmt::Display + 'a {
    SafeLocation(location)
}

pub fn safe_backtrace(backtrace: &Backtrace) -> impl fmt::Display + '_ {
    SafeBacktrace(backtrace)
}

pub fn safe_diagnostic(value: impl fmt::Display) -> impl fmt::Display {
    SafeDiagnostic(value)
}

struct SafeLocation<'a>(&'a Location<'a>);
impl fmt::Display for SafeLocation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            source_path(self.0.file()),
            self.0.line(),
            self.0.column()
        )
    }
}

struct SafeBacktrace<'a>(&'a Backtrace);
impl fmt::Display for SafeBacktrace<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&sanitize_text(&format!("{:#}", self.0)))
    }
}

struct SafeDiagnostic<T>(T);
impl<T: fmt::Display> fmt::Display for SafeDiagnostic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = String::new();
        write!(text, "{}", self.0)?;
        f.write_str(&sanitize_text(&text))
    }
}

fn source_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let parts: Vec<_> = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let Some(file) = parts.last().copied().filter(|file| {
        file.contains('.')
            && *file != "."
            && *file != ".."
            && !file.contains(':')
            && !file.chars().any(char::is_control)
    }) else {
        return "[source]".to_owned();
    };
    if normalized.ends_with('/') || parts.contains(&"..") {
        return if normalized.ends_with('/') {
            "[source]".to_owned()
        } else {
            file.to_owned()
        };
    }
    let relative = !normalized.starts_with('/') && !normalized.contains(':');
    if relative
        && (parts.first() == Some(&"library")
            || (parts.get(1) == Some(&"src")
                && parts.first().is_some_and(|name| {
                    name.contains('-')
                        && name.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                        })
                })))
    {
        return parts.join("/");
    }
    for (index, part) in parts.iter().enumerate() {
        let remaining = &parts[index..];
        if (*part == "gallery" && remaining.get(1) == Some(&"src"))
            || (*part == "crates" && remaining.get(2) == Some(&"src"))
        {
            return remaining.join("/");
        }
    }
    if let Some(index) = parts
        .windows(3)
        .position(|parts| parts == [".cargo", "registry", "src"])
        && let Some(suffix) = parts
            .get(index + REGISTRY_PACKAGE_OFFSET..)
            .filter(|suffix| suffix.len() >= 2)
    {
        return suffix.join("/");
    }
    if (normalized.starts_with("/rustc/") || parts.contains(&".rustup"))
        && let Some(index) = parts.iter().position(|part| *part == "library")
    {
        return parts[index..].join("/");
    }
    if let Some(index) = parts
        .windows(3)
        .position(|parts| parts == [".cargo", "git", "checkouts"])
        && let (Some(repository), Some(suffix)) = (
            parts.get(index + GIT_REPOSITORY_OFFSET),
            parts.get(index + GIT_CHECKOUT_SOURCE_OFFSET..),
        )
        && !suffix.is_empty()
    {
        return format!("{repository}/{}", suffix.join("/"));
    }
    file.to_owned()
}

pub(crate) fn sanitize_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        let remaining = &text[index..];
        if remaining.starts_with("at ") && source_marker(text, index) {
            output.push_str("at ");
            index += "at ".len();
            let remaining = &text[index..];
            let end = source_file_end(remaining).unwrap_or_else(|| logical_line_end(remaining));
            let (path, coordinates) = split_coordinates(&remaining[..end]);
            output.push_str(&source_path(path));
            output.push_str(coordinates);
            index += end;
            continue;
        }
        if path_root(text, index) {
            let prefix = &text[..index];
            let in_backtrace = debug_container_open(prefix, "Backtrace [", '[', ']');
            let in_location = debug_container_open(prefix, "Location {", '{', '}');
            let debug_file = prefix.trim_end_matches([' ', '"', '\\']).ends_with("file:")
                && (in_backtrace || in_location);
            if debug_file && let Some(end) = remaining.find('"') {
                let path = remaining[..end].trim_end_matches('\\');
                output.push_str(&source_path(path));
                output.push_str(&remaining[path.len()..end]);
                index += end;
                continue;
            }
            if let Some(end) = source_file_end(remaining) {
                let candidate = &remaining[..end];
                let (path, coordinates) = split_coordinates(candidate);
                let normalized = path
                    .replace('\\', "/")
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("/");
                let known_source = normalized.contains(".cargo/registry/src/")
                    || normalized.contains(".cargo/git/checkouts/")
                    || normalized.contains(".rustup/toolchains/")
                    || normalized.starts_with("rustc/");
                if !coordinates.is_empty() || known_source {
                    output.push_str(&source_path(path));
                    output.push_str(coordinates);
                    index += end;
                    continue;
                }
            }
        }
        if let Some(character) = remaining.chars().next() {
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
}

fn debug_container_open(prefix: &str, marker: &str, opening: char, closing: char) -> bool {
    let Some((_, tail)) = prefix.rsplit_once(marker) else {
        return false;
    };
    let encoded = tail
        .find('"')
        .is_some_and(|index| index != 0 && tail.as_bytes()[index - 1] == b'\\');
    let tail = if encoded {
        let mut decoded = String::with_capacity(tail.len());
        let mut characters = tail.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\\'
                && characters
                    .peek()
                    .is_some_and(|next| matches!(next, '\\' | '"'))
            {
                if let Some(next) = characters.next() {
                    decoded.push(next);
                }
            } else {
                decoded.push(character);
            }
        }
        std::borrow::Cow::Owned(decoded)
    } else {
        std::borrow::Cow::Borrowed(tail)
    };
    let mut depth = 1_usize;
    let mut quoted = false;
    let mut escaped = false;
    for character in tail.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else if character == '"' {
            quoted = true;
        } else if character == opening {
            depth += 1;
        } else if character == closing {
            depth -= 1;
            if depth == 0 {
                return false;
            }
        }
    }
    true
}

fn source_marker(text: &str, index: usize) -> bool {
    let prefix = &text[..index];
    let line = prefix.rsplit('\n').next().unwrap_or_default();
    let boundary = line.trim_matches([' ', '\t', '"']).is_empty()
        || line
            .rsplit_once(r"\n")
            .is_some_and(|(_, tail)| tail.trim().is_empty());
    if !boundary {
        return false;
    }
    let previous = prefix
        .trim_end_matches([' ', '\t'])
        .trim_end_matches(['\n', '\r']);
    let previous = previous.strip_suffix(r"\n").unwrap_or(previous);
    let previous = previous
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .rsplit(r"\n")
        .next()
        .unwrap_or_default();
    let frame = previous.trim().split_once(':').is_some_and(|(number, _)| {
        let number = number
            .rsplit('=')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"');
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    });
    let source = &text[index + "at ".len()..];
    frame
        || source_file_end(source)
            .is_some_and(|end| !split_coordinates(&source[..end]).1.is_empty())
}

fn path_root(text: &str, index: usize) -> bool {
    let remaining = &text.as_bytes()[index..];
    let boundary = index == 0
        || text.as_bytes()[index - 1].is_ascii_whitespace()
        || matches!(
            text.as_bytes()[index - 1],
            b'=' | b'"' | b'\'' | b'(' | b'['
        );
    boundary
        && (remaining.starts_with(b"\\\\")
            || remaining.starts_with(b"/")
            || (remaining.first().is_some_and(u8::is_ascii_alphabetic)
                && remaining.get(1) == Some(&b':')
                && remaining
                    .get(2)
                    .is_some_and(|byte| *byte == b'/' || *byte == b'\\')))
}

fn source_file_end(text: &str) -> Option<usize> {
    let limit = text
        .find(['\n', '\r', '"', '='])
        .unwrap_or(text.len())
        .min(logical_line_end(text));
    let limit = text[..limit]
        .char_indices()
        .skip(1)
        .find_map(|(index, _)| path_root(text, index).then_some(index))
        .unwrap_or(limit);
    let candidate = &text[..limit];
    for (index, _) in candidate.match_indices('.') {
        let extension = &candidate[index..];
        let Some(length) =
            [".rs", ".cpp", ".cc", ".c", ".hpp", ".h"]
                .iter()
                .find_map(|extension_name| {
                    extension
                        .starts_with(extension_name)
                        .then_some(extension_name.len())
                })
        else {
            continue;
        };
        let mut end = index + length;
        for _ in 0..SOURCE_COORDINATE_COMPONENTS {
            if candidate.as_bytes().get(end) != Some(&b':') {
                break;
            }
            let digits = candidate[end + 1..]
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            if digits == 0 {
                break;
            }
            end += 1 + digits;
        }
        if candidate.as_bytes().get(end).is_none_or(|byte| {
            byte.is_ascii_whitespace()
                || matches!(byte, b'\\' | b'\'' | b';' | b',' | b')' | b']' | b'}')
        }) {
            return Some(end);
        }
    }
    None
}

fn logical_line_end(text: &str) -> usize {
    let actual = text.find(['\n', '\r']).unwrap_or(text.len());
    let escaped = text
        .match_indices(r"\n")
        .find_map(|(index, _)| {
            let after = &text[index + r"\n".len()..];
            let boundary = after
                .chars()
                .next()
                .is_none_or(|c| c.is_ascii_whitespace() || c.is_ascii_digit() || c == '"');
            (boundary && (index == 0 || text.as_bytes()[index - 1] != b'\\')).then_some(index)
        })
        .unwrap_or(text.len());
    actual.min(escaped)
}

fn split_coordinates(text: &str) -> (&str, &str) {
    let mut end = text.len();
    for _ in 0..SOURCE_COORDINATE_COMPONENTS {
        let Some((prefix, number)) = text[..end].rsplit_once(':') else {
            break;
        };
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            break;
        }
        end = prefix.len();
    }
    (&text[..end], &text[end..])
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn source_paths_keep_only_confirmed_source_suffixes() {
        for (path, expected) in [
            (
                r"C:\Users\builder\.cargo\registry\src\index.crates.io-abcd\bevy_app-0.20.0\src\app.rs",
                "bevy_app-0.20.0/src/app.rs",
            ),
            (
                r"C:\Users\builder\.cargo\git\checkouts\bevy-brp-123\abcdef\crates\extras\src\lib.rs",
                "crates/extras/src/lib.rs",
            ),
            (
                r"C:\Users\builder\.cargo\git\checkouts\dep-123\abcdef\src\lib.rs",
                "dep-123/src/lib.rs",
            ),
            (
                r"C:\Users\builder\.rustup\toolchains\stable\lib\rustlib\src\rust\library\std\src\panicking.rs",
                "library/std/src/panicking.rs",
            ),
            (
                "/rustc/123456/library/std/src/thread/mod.rs",
                "library/std/src/thread/mod.rs",
            ),
            (
                r"D:\build-agent\work\gallery\src\diagnostics.rs",
                "gallery/src/diagnostics.rs",
            ),
            (
                r"\\?\D:\构建 目录/crates\app_logging\src\writer.rs",
                "crates/app_logging/src/writer.rs",
            ),
            (r"\\server\share\source tree\src\native.cpp", "native.cpp"),
            ("/tmp/cargo-install abcd/src/main.rs", "main.rs"),
            ("src/lib.rs", "lib.rs"),
            ("plain.rs", "plain.rs"),
            ("", "[source]"),
            (r"C:\", "[source]"),
            (r"D:\crates\private\src\..\..\secret.rs", "secret.rs"),
            ("unrecognized source location", "[source]"),
        ] {
            assert_eq!(source_path(path), expected, "{path}");
        }
    }

    #[test]
    fn backtrace_text_keeps_frames_symbols_addresses_and_coordinates() {
        let raw = concat!(
            "0: 0x123 app::load\n",
            "   at D:\\build agent\\gallery\\src\\diagnostics.rs:132:7\n",
            "1: 0x456 external::read\n",
            "   at \\\\server\\share\\folder with spaces\\foreign.cpp:42\n",
            "2: symbol_only\n",
            "   at unknown location\n",
            "3: 0x789 address_only\n",
            "   at /tmp/build/src/generated.rs\n",
        );
        let expected = concat!(
            "0: 0x123 app::load\n",
            "   at gallery/src/diagnostics.rs:132:7\n",
            "1: 0x456 external::read\n",
            "   at foreign.cpp:42\n",
            "2: symbol_only\n",
            "   at [source]\n",
            "3: 0x789 address_only\n",
            "   at generated.rs\n",
        );
        assert_eq!(sanitize_text(raw), expected);
        assert_eq!(sanitize_text(expected), expected);
        assert_eq!(
            sanitize_text(
                "details=0: app::read\n   at C:\\private\\generated.rs\n1: opaque\n   at unknown location\n"
            ),
            "details=0: app::read\n   at generated.rs\n1: opaque\n   at [source]\n"
        );
        assert_eq!(sanitize_text(&format!("{raw:?}")), format!("{expected:?}"));
        let cargo = "at C:\\builder\\.cargo\\registry\\src\\index\\dep-1.2.3\\src\\lib.rs:5\n   at /rustc/123/library/std/src/lib.rs:9\n";
        let safe = sanitize_text(cargo);
        assert_eq!(sanitize_text(&safe), safe);
    }

    #[test]
    fn nested_diagnostics_clean_source_shapes_and_keep_runtime_file_paths() {
        let raw = concat!(
            "error_id=abc error=failed, path=C:\\app data\\config\\config.ron ",
            "assets=D:/user/assets/image.png user_file=C:\\files\\recording.widx\n",
            "details=Backtrace [{ fn: \"external::read\", file: \"C:\\\\build agent\\\\src\\\\read.rs\", line: 19 }]; ",
            "source=C:\\Users\\builder\\.cargo\\registry\\src\\index\\dep-1.2.3\\src\\lib.rs:5:2; ",
            "other=C:/private/build/src/local.rs:21",
        );
        let output = sanitize_text(raw);
        assert!(
            output.contains(r"path=C:\app data\config\config.ron"),
            "{output}"
        );
        assert!(output.contains("assets=D:/user/assets/image.png"));
        assert!(output.contains(r"user_file=C:\files\recording.widx"));
        assert!(output.contains("error_id=abc error=failed"));
        assert!(output.contains("fn: \"external::read\""));
        assert!(output.contains("file: \"read.rs\", line: 19"), "{output}");
        assert!(output.contains("dep-1.2.3/src/lib.rs:5:2"));
        assert!(output.contains("other=local.rs:21"));
        assert!(!output.contains("builder"));
        assert!(!output.contains("private/build"));
        assert_eq!(
            sanitize_text("普通业务文本，没有源码位置"),
            "普通业务文本，没有源码位置"
        );
        assert_eq!(
            sanitize_text("at D:\\user data\\config.ron\npath=C:\\user\\code.rs"),
            "at D:\\user data\\config.ron\npath=C:\\user\\code.rs"
        );
        assert_eq!(
            sanitize_text(r#"business=File { file: "C:\user\code.rs" }"#),
            r#"business=File { file: "C:\user\code.rs" }"#
        );
        assert_eq!(
            sanitize_text(r#"Location { file: "C:\private\Unknown.RS", line: 7 }"#),
            r#"Location { file: "Unknown.RS", line: 7 }"#
        );
        assert_eq!(
            sanitize_text(
                r"error=C:\Program Files (x86)\builder\.cargo\registry\src\index\dep-1.2.3\src\lib.rs:7"
            ),
            "error=dep-1.2.3/src/lib.rs:7"
        );
        assert_eq!(
            sanitize_text(r"failed C:\user\config.ron while reading C:\builder\code.rs:7"),
            r"failed C:\user\config.ron while reading code.rs:7"
        );
    }

    #[test]
    fn borrowed_views_preserve_original_location_and_captured_backtrace() {
        let location = Location::caller();
        let original = location.to_string();
        let rendered = safe_location(location).to_string();
        assert!(rendered.ends_with(&format!(":{}:{}", location.line(), location.column())));
        assert_eq!(location.to_string(), original);
        let backtrace = Backtrace::force_capture();
        let original = format!("{backtrace:#}");
        let rendered = safe_backtrace(&backtrace).to_string();
        let frames = |text: &str| {
            text.lines()
                .filter(|line| {
                    line.trim_start()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit())
                })
                .count()
        };
        assert_eq!(frames(&rendered), frames(&original));
        assert_eq!(
            backtrace.status(),
            std::backtrace::BacktraceStatus::Captured
        );
        assert_eq!(format!("{backtrace:#}"), original);
        assert_eq!(
            safe_diagnostic("at C:\\private\\read.rs:7:2").to_string(),
            "at read.rs:7:2"
        );
    }

    #[test]
    fn debug_container_boundaries_ignore_quoted_symbols_and_source_paths() {
        let raw = r#"Backtrace [{ fn: "consumer::call::<[u8; 16]>", file: "C:\private [build]\src\read.rs", line: 19 }, { fn: "consumer::quoted\"symbol]", file: "D:\private\src\next.rs", line: 20 }] business=File { file: "D:\user\code.rs" }"#;
        let expected = r#"Backtrace [{ fn: "consumer::call::<[u8; 16]>", file: "read.rs", line: 19 }, { fn: "consumer::quoted\"symbol]", file: "next.rs", line: 20 }] business=File { file: "D:\user\code.rs" }"#;
        assert_eq!(sanitize_text(raw), expected);
        assert_eq!(sanitize_text(&format!("{raw:?}")), format!("{expected:?}"));
        assert_eq!(sanitize_text(expected), expected);
        assert_eq!(
            sanitize_text(
                r#"Location { file: "C:\private } root\read.rs", line: 1 } business=File { file: "D:\user\code.rs" }"#
            ),
            r#"Location { file: "read.rs", line: 1 } business=File { file: "D:\user\code.rs" }"#
        );
    }
}
