//! Private controller-scoped CLI; no image provisioning or general package path.
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use photara_store::package::v1_3::repeatable::disposable::{
        ControlledRequest, ControlledSession,
    };
    use std::io::{BufRead, Read, Write};
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: photara-controlled-session MANIFEST BINDING_JSON_FILE".into());
    }
    // Scope verification independently safe-walks manifest/image/owner. The binding
    // is only input to those checks; this file never grants admission by itself.
    let f = std::fs::File::open(&args[1])?;
    let mut binding = Vec::new();
    std::io::Read::take(f, 65537).read_to_end(&mut binding)?;
    if binding.len() > 65536 {
        return Err("binding bound".into());
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
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Controlled disposable admission is unavailable on this platform");
    std::process::exit(1);
}
