//! Explicit fresh disposable registrar utility; input/output outside DB directory.
use photara_library::disposable::{
    LocalDatabase, Pins, ProjectRegistration, Registration, bootstrap_ids,
};
use photara_store::package::v1_3::activation::v2::SelectionStore;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 4 {
        return Err("usage: disposable_authority create|open|register-project|create-library|rename-library DB REGISTRATION [PINS] [REQUEST]".into());
    }
    let reg: Registration = serde_json::from_slice(&fs::read(&args[3])?)?;
    let database = reg.database_id;
    let at = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let mut db = if args[1] == "create" {
        LocalDatabase::create_fresh(Path::new(&args[2]), reg, at)?
    } else {
        let pins: Pins = serde_json::from_slice(&fs::read(args.get(4).ok_or("pins required")?)?)?;
        LocalDatabase::reopen(Path::new(&args[2]), reg, pins)?
    };
    let request = || -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&fs::read(
            args.get(5).ok_or("request required")?,
        )?)?)
    };
    let result = match args[1].as_str() {
        "create" | "open" => {
            json!({"pins":db.pins(),"scope":SelectionStore::registration(&db),"bootstrap_library_id":bootstrap_ids(database).0,"initial_principal_id":bootstrap_ids(database).1,"settings":db.settings()?,"libraries":db.libraries()?,"snapshot":serde_json::from_slice::<Value>(&db.load()?)?})
        }
        "register-project" => {
            let p: ProjectRegistration = serde_json::from_value(request()?)?;
            db.register_project(&p, at)?;
            json!({"registered":p.project_id})
        }
        "create-library" => db.create(&request()?)?,
        "rename-library" => db.rename(&request()?)?,
        _ => return Err("unknown action".into()),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
