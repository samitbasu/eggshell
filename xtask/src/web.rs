//! `cargo xtask web NAME`: build an example for the browser and serve it.
//!
//! The build is `cargo build --target wasm32-unknown-unknown --example NAME`
//! followed by the `wasm-bindgen` CLI (`--target web`), whose output plus a
//! generated `index.html` land in `target/web/NAME/`. A std-only static
//! server, in this process, serves that directory until Ctrl+C. Nothing is
//! spawned that outlives xtask, and the build children run in process groups
//! it ends itself.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{Context, Result, bail};

use crate::process;
use crate::steps::{Profile, Step, WASM_TARGET};

const CANVAS_ID: &str = "bullseye_canvas";

pub struct Plan {
    pub example: String,
    pub profile: Profile,
    pub port: u16,
}

pub fn run(plan: &Plan) -> Result<()> {
    let root = process::workspace_root();
    ensure_example_exists(&root, &plan.example)?;
    ensure_wasm_bindgen(&root)?;
    let out_dir = root.join("target/web").join(&plan.example);
    build(&root, plan, &out_dir)?;
    serve(&out_dir, plan.port)
}

fn ensure_example_exists(root: &Path, name: &str) -> Result<()> {
    let known = crate::examples::list(&root.join("examples"))?;
    if known.iter().any(|example| example.name == name) {
        return Ok(());
    }
    let names: Vec<_> = known.iter().map(|e| e.name.as_str()).collect();
    bail!("no example `{name}`; known: {}", names.join(", "))
}

/// The version of `wasm-bindgen` that `Cargo.lock` pins; the CLI must match
/// it exactly.
pub fn locked_wasm_bindgen_version(lock: &str) -> Option<String> {
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "name = \"wasm-bindgen\"" {
            return lines
                .next()?
                .trim()
                .strip_prefix("version = \"")?
                .strip_suffix('"')
                .map(str::to_owned);
        }
    }
    None
}

fn ensure_wasm_bindgen(root: &Path) -> Result<()> {
    let lock = fs::read_to_string(root.join("Cargo.lock")).context("reading Cargo.lock")?;
    let wanted = locked_wasm_bindgen_version(&lock)
        .context("Cargo.lock does not pin `wasm-bindgen`; build the example for wasm32 first")?;
    let install = format!("cargo install wasm-bindgen-cli --version {wanted} --locked");
    let Some(found) = process::tool_version("wasm-bindgen") else {
        bail!("`wasm-bindgen` is not installed; install it with: {install}");
    };
    if found.split_whitespace().last() != Some(wanted.as_str()) {
        bail!(
            "`{found}` does not match the {wanted} that Cargo.lock pins; install it with: {install}"
        );
    }
    Ok(())
}

fn build(root: &Path, plan: &Plan, out_dir: &Path) -> Result<()> {
    let (profile_flag, profile_dir) = match plan.profile {
        Profile::Debug => (None, "debug"),
        Profile::Release => (Some("--release"), "release"),
    };
    let mut args = vec!["build", "--target", WASM_TARGET, "--example", &plan.example];
    args.extend(profile_flag);
    Step::cargo("wasm build", &args).run()?;

    let wasm = root
        .join("target")
        .join(WASM_TARGET)
        .join(profile_dir)
        .join("examples")
        .join(format!("{}.wasm", plan.example));
    let _ = fs::remove_dir_all(out_dir);
    fs::create_dir_all(out_dir)?;
    let bindgen_args: Vec<String> = [
        "--target".as_ref(),
        "web".as_ref(),
        "--no-typescript".as_ref(),
        "--out-name".as_ref(),
        "app".as_ref(),
        "--out-dir".as_ref(),
        out_dir.as_os_str(),
        wasm.as_os_str(),
    ]
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
    eprintln!("==> wasm-bindgen: wasm-bindgen {}", bindgen_args.join(" "));
    let (status, _) = process::run_program("wasm-bindgen", &bindgen_args, &[])?;
    if !status.success() {
        bail!("wasm-bindgen failed ({status})");
    }
    fs::write(out_dir.join("index.html"), index_html(&plan.example))?;
    Ok(())
}

pub fn index_html(example: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>bullseye: {example}</title>
<style>
html, body {{ margin: 0; height: 100%; overflow: hidden; }}
canvas {{ display: block; width: 100%; height: 100%; }}
</style>
</head>
<body>
<canvas id="{CANVAS_ID}"></canvas>
<script type="module">
import init from "./app.js";
await init();
</script>
</body>
</html>
"#
    )
}

fn serve(dir: &Path, port: u16) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("binding 127.0.0.1:{port}"))?;
    let address = listener.local_addr()?;
    println!(
        "serving {} at http://{address}/  (Ctrl+C to stop)",
        dir.display()
    );
    let dir = dir.to_path_buf();
    for stream in listener.incoming().flatten() {
        let dir = dir.clone();
        thread::spawn(move || {
            let _ = respond(stream, &dir);
        });
    }
    Ok(())
}

/// Maps a request target to a file under `dir`, refusing anything that
/// leaves it.
pub fn resolve(dir: &Path, target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next()?.trim_start_matches('/');
    let mut resolved = dir.to_path_buf();
    for part in path.split('/').filter(|part| !part.is_empty()) {
        if part == ".." || part.contains(['\\', '\0']) {
            return None;
        }
        resolved.push(part);
    }
    if path.is_empty() || path.ends_with('/') {
        resolved.push("index.html");
    }
    Some(resolved)
}

pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn respond(mut stream: TcpStream, dir: &Path) -> std::io::Result<()> {
    let mut request_line = String::new();
    BufReader::new(&stream).read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next(), parts.next());
    let file = match (method, target) {
        (Some("GET"), Some(target)) => {
            resolve(dir, target).and_then(|p| fs::read(&p).ok().map(|b| (p, b)))
        }
        _ => None,
    };
    match file {
        Some((path, body)) => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                content_type(&path),
                body.len()
            )?;
            stream.write_all(&body)
        }
        None => stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn finds_the_locked_version() {
        let lock = "[[package]]\nname = \"other\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.130\"\n";
        assert_eq!(
            locked_wasm_bindgen_version(lock).as_deref(),
            Some("0.2.130")
        );
        assert_eq!(locked_wasm_bindgen_version("name = \"x\"\n"), None);
    }

    #[test]
    fn resolve_stays_inside_the_directory() {
        let dir = Path::new("/srv");
        assert_eq!(resolve(dir, "/"), Some(dir.join("index.html")));
        assert_eq!(resolve(dir, "/app.js?v=1"), Some(dir.join("app.js")));
        assert_eq!(resolve(dir, "/a/../../etc/passwd"), None);
    }

    #[test]
    fn wasm_is_served_with_its_mime_type() {
        assert_eq!(content_type(Path::new("app_bg.wasm")), "application/wasm");
        assert_eq!(
            content_type(Path::new("app.js")),
            "text/javascript; charset=utf-8"
        );
    }

    #[test]
    fn index_names_the_canvas_the_examples_look_for() {
        let canvas_id = include_str!("../../examples/common/mod.rs")
            .lines()
            .find_map(|l| l.strip_prefix("pub const WEB_CANVAS_ID: &str = \""))
            .and_then(|l| l.strip_suffix("\";"))
            .unwrap();
        assert!(index_html("hello").contains(&format!("id=\"{canvas_id}\"")));
    }

    #[test]
    fn serves_files_and_refuses_unknown_ones() {
        let tmp = TempDir::new("web");
        fs::write(tmp.path().join("index.html"), "hi").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let dir = tmp.path().to_path_buf();
        let server = thread::spawn(move || {
            for stream in listener.incoming().take(2).flatten() {
                respond(stream, &dir).unwrap();
            }
        });
        let get = |target: &str| {
            let mut stream = TcpStream::connect(address).unwrap();
            write!(stream, "GET {target} HTTP/1.1\r\n\r\n").unwrap();
            let mut reply = String::new();
            stream.read_to_string(&mut reply).unwrap();
            reply
        };
        let index = get("/");
        let missing = get("/nope.js");
        server.join().unwrap();
        assert!(index.starts_with("HTTP/1.1 200") && index.ends_with("hi"));
        assert!(missing.starts_with("HTTP/1.1 404"));
    }
}
