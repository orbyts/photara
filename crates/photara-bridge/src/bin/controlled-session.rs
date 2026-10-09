//! Private controller-scoped CLI; no image provisioning or general package path.
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use photara_store::package::v1_3::repeatable::disposable::{
        ControlledRequest, ControlledSession,
    };
    use std::io::{BufRead, Read, Write};
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(2..=4).contains(&args.len()) {
        return Err("usage: photara-controlled-session MANIFEST BINDING_JSON_FILE [TARGET_BINDINGS_JSON_FILE [DATABASE_REGISTRATION]]".into());
    }
    // Scope verification independently safe-walks manifest/image/owner. The binding
    // is only input to those checks; this file never grants admission by itself.
    let f = std::fs::File::open(&args[1])?;
    let mut binding = Vec::new();
    std::io::Read::take(f, 65537).read_to_end(&mut binding)?;
    if binding.len() > 65536 {
        return Err("binding bound".into());
    }
    if args.len() == 4 {
        return local_workspace(&args, serde_json::from_slice(&binding)?);
    }
    if args.len() == 3 {
        return workspace(&args, serde_json::from_slice(&binding)?);
    }
    let mut session = ControlledSession::open(
        std::path::Path::new(&args[0]),
        serde_json::from_slice(&binding)?,
    )?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut bytes = Vec::new();
        let n = std::io::Read::by_ref(&mut input)
            .take(1_048_577)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            return Err("interrupted session: explicit verified Close required".into());
        }
        if bytes.len() > 1_048_576 || !bytes.ends_with(b"\n") {
            return Err("request bound".into());
        }
        let request: ControlledRequest = serde_json::from_slice(&bytes)?;
        let response = session.execute(request);
        println!("{response}");
        std::io::stdout().flush()?;
        if response["snapshot"]["closed"] == true && response["error"].is_null() {
            break;
        }
    }
    Ok(())
}
#[cfg(target_os = "macos")]
fn workspace(
    args: &[String],
    binding: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use photara_store::package::v1_3::repeatable::disposable::{
        ControlledRequest, ControlledWorkspace,
    };
    use std::io::{BufRead, Read, Write};
    let f = std::fs::File::open(&args[2])?;
    let mut targets = Vec::new();
    f.take(65537).read_to_end(&mut targets)?;
    if targets.len() > 65536 {
        return Err("target bindings bound".into());
    }
    let mut w = ControlledWorkspace::open(
        std::path::Path::new(&args[0]),
        binding,
        serde_json::from_slice(&targets)?,
    )?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut bytes = Vec::new();
        let n = std::io::Read::by_ref(&mut input)
            .take(1_048_577)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            return Err("interrupted workspace: explicit verified Close required".into());
        }
        if bytes.len() > 1_048_576 || !bytes.ends_with(b"\n") {
            return Err("request bound".into());
        }
        let r: serde_json::Value = serde_json::from_slice(&bytes)?;
        let id = r["activation_id"].as_str().unwrap_or("");
        let value = match r["command"].as_str() {
            Some("workspace-state") => w.state(),
            Some("prepare") => w.prepare(
                id,
                r["target_project_id"].as_str().unwrap_or(""),
                r["view"].clone(),
                r["expected_owner_epoch"].as_str(),
                r["expected_attachment_generation"].as_u64(),
            ),
            Some("confirm") => w.confirm(id),
            Some("cancel") => w.cancel(id),
            Some("activation-retry") => w.retry(id),
            Some("faults") => {
                let points: Vec<String> = serde_json::from_value(r["points"].clone())?;
                w.inject_faults(&points)?;
                w.state()
            }
            _ => {
                let request: ControlledRequest = serde_json::from_value(r)?;
                w.execute(request)
            }
        };
        println!("{value}");
        std::io::stdout().flush()?;
        if value["snapshot"]["closed"] == true && value["error"].is_null() {
            break;
        }
    }
    Ok(())
}
#[cfg(target_os = "macos")]
fn local_workspace(
    args: &[String],
    binding: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use photara_store::package::v1_3::repeatable::disposable::ControlledRequest;
    use std::io::{BufRead, Read, Write};
    let mut raw = Vec::new();
    std::fs::File::open(&args[2])?
        .take(65537)
        .read_to_end(&mut raw)?;
    if raw.len() > 65536 {
        return Err("target bound".into());
    }
    let mut w = photara_bridge::open_controlled_local_workspace(
        std::path::Path::new(&args[0]),
        binding,
        serde_json::from_slice(&raw)?,
        std::path::Path::new(&args[3]),
    )?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut bytes = Vec::new();
        let n = std::io::Read::by_ref(&mut input)
            .take(1_048_577)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            return Err("interrupted workspace: explicit verified Close required".into());
        }
        if bytes.len() > 1_048_576 || !bytes.ends_with(b"\n") {
            return Err("request bound".into());
        }
        let r: serde_json::Value = serde_json::from_slice(&bytes)?;
        let id = r["activation_id"].as_str().unwrap_or("");
        let value = match r["command"].as_str() {
            Some("workspace-state") => w.state(),
            Some("prepare") => w.prepare(
                id,
                r["target_project_id"].as_str().unwrap_or(""),
                r["view"].clone(),
                r["expected_owner_epoch"].as_str(),
                r["expected_attachment_generation"].as_u64(),
            ),
            Some("close-project") => w.close_project(
                id,
                r["view"].clone(),
                r["expected_owner_epoch"].as_str(),
                r["expected_attachment_generation"].as_u64(),
            ),
            Some("select-library") => w.select_library(
                id,
                r["library_id"].as_str().unwrap_or(""),
                r["view"].clone(),
                r["expected_owner_epoch"].as_str(),
                r["expected_attachment_generation"].as_u64(),
            ),
            Some("create-library" | "rename-library") => w.library_command(
                r["operation_id"].as_str().unwrap_or(""),
                r["library_id"].as_str().unwrap_or(""),
                r["name"].as_str().unwrap_or(""),
                if r["command"] == "rename-library" {
                    Some(
                        r["expected_revision"]
                            .as_u64()
                            .ok_or("expected revision required")?,
                    )
                } else {
                    None
                },
            ),
            Some("lifecycle") => w.lifecycle(r["request"].clone()),
            Some("confirm") => w.confirm(id),
            Some("cancel") => w.cancel(id),
            Some("activation-retry") => w.retry(id),
            Some("faults") => {
                w.inject_fault(r["point"].as_str().ok_or("point required")?)?;
                w.state()
            }
            _ => w.execute(serde_json::from_value::<ControlledRequest>(r)?),
        };
        println!("{value}");
        std::io::stdout().flush()?;
        if value["error"].is_null()
            && (value["snapshot"]["closed"] == true || value["result"]["context_closed"] == true)
        {
            return Ok(());
        }
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Controlled disposable admission is unavailable on this platform");
    std::process::exit(1);
}
