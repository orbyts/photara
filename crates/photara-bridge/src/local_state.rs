//! Safe D19 app-state initialization, separate from authored Project packages.
use crate::BridgeError;
use photara_library::gen2::{LocalLibraryStore, Timestamp};

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeLocalState {
    pub database_id: String,
    pub device_id: String,
    pub library_id: String,
    pub principal_id: String,
}
/// Initializes or verifies the explicit local `SQLite` path and default Library.
/// # Errors
/// Refuses unknown databases, unsafe paths and unsupported schema floors.
#[uniffi::export]
pub fn initialize_local_state(path: String) -> Result<BridgeLocalState, BridgeError> {
    let fail = || BridgeError::Store {
        message: "Local state initialization failed; existing data was not adopted.".into(),
    };
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| fail())?;
    let at = Timestamp::try_from(i64::try_from(elapsed.as_millis()).map_err(|_| fail())?)
        .map_err(|_| fail())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| fail())?;
    runtime.block_on(async {
        let (store, id) = LocalLibraryStore::open_app_state(path, at)
            .await
            .map_err(|error| BridgeError::Store {
                message: format!("Local state initialization failed: {error}"),
            })?;
        store.verify_integrity().await.map_err(|_| fail())?;
        store.close().await;
        Ok(BridgeLocalState {
            database_id: id.database_id.uuid().to_string(),
            device_id: id.device_id.uuid().to_string(),
            library_id: id.library_id.uuid().to_string(),
            principal_id: id.principal_id.to_string(),
        })
    })
}
