pub mod android;
pub mod database;
pub mod mock_profiles;
pub mod pki;

mod jni;
mod utils;

use crate::database::IoError;
use crate::database::RawOperations;
use crate::database::Vcard;
use crate::pki::Certificate;
use crate::pki::CertificateId;
use crate::utils::ResultOption;
use futures::Stream;
use riko_runtime::HeapObject;
use sled::Db;
use std::path::Path;
use std::path::PathBuf;

/// The protagonist.
pub struct Client {
    database: Db,
    profile_path: PathBuf,
}

impl Client {
    /// Constructor.
    ///
    /// No need to explicitly start running the client. Once it is created, everything is functional
    /// until the whole object is dropped.
    pub fn new(profile_path: PathBuf) -> Result<Client, sled::Error> {
        let mut database_path = profile_path.clone();
        database_path.push("database");
        let database = Db::start_default(&database_path)?;

        Ok(Client {
            database,
            profile_path,
        })
    }
    pub fn profile_path(&self) -> &Path {
        &self.profile_path
    }
    pub fn vcard(
        &self,
        account_id: Option<&CertificateId>,
    ) -> impl Stream<Item = Result<Option<Vcard>, IoError>> {
        if let Some(id) = account_id {
            futures::stream::once(futures::future::ready(self.database.vcard(id)))
        } else {
            let vcard = match self.account_id() {
                Err(e) => Err(e.into()),
                Ok(None) => Ok(None),
                Ok(Some(id)) => self.database.vcard(&id),
            };
            futures::stream::once(futures::future::ready(vcard))
        }
    }
    pub fn account_id(&self) -> Result<Option<Vec<u8>>, sled::Error> {
        self.database
            .account_certificate()
            .map_deep(|cert| cert.id())
    }
}

impl HeapObject for Client {} // TODO: derive
