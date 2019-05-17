#[macro_use]
extern crate log;

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;

pub mod android;
pub mod pki;

/// Combination of an account ID and a device ID.
///
/// It is used to identify an entity a client can interact with. For example, specifying the destination of a message.
pub struct Address {
    pub account: Vec<u8>,
    pub device: Vec<u8>,
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let convert = |data: &[u8]| data_encoding::HEXUPPER.encode(data);
        write!(f, "{}/{}", convert(&self.account), convert(&self.device))
    }
}

/// Collection of files representing a device profile.
///
/// The majority of the data is structured in the form of filesystem tree instead of structured data formats such as
/// JSON. The reasons include faster random access, fewer conflicts during Git merges, avoiding overhead of byte-to-text
/// encoding such as Base64, etc..
///
/// # Directory structure
///
/// * `account.cert`: Account certificate.
/// * `account.key`: Private key to the account certificate.
/// * `blacklist/`: Accounts whose connections must be refused.
///   * `<account-id>`: Repeatable empty files.
/// * `device.cert`: Device certificate.
/// * `device.key`: Private key to the device certificate.
/// * `messages/`: All historical messages.
///   * `<chatroom-id>/`: Repeatable directories representing a chatroom.
///     * `<message-id>/`: Repeatable directories containing the history in a chatroom.
///       * `body`: Format depends on `type`.
///       * `filename`: (Optional) suggested filename if the message is the result of a file transfer.
///       * `sender`: Full address of the sender.
///       * `time`: Sent time.
///       * `type`: (Optional) IANA-registered media type, defaults to `text/plain`
/// * `roster/`: Trusted peer accounts.
///   * `<account-id>/`: Repeatable directories.
///     * `vCard/*`: Same as the account `vCard/*`.
/// * `unmanaged/`: Data that are not managed by Git.
/// * `vCard/`: Public information of an account.
///   * `avatar`: User photo, must be an WebP or an SVG.
///   * `description`: Additional description of the account.
///   * `devices/`: Linked devices.
///     * `<device-id>/`: Repeatable directories.
///       * `name`: Display name, only the first line is read.
///       * `network/`: Discovery networks the account has joined.
///         * `<network-name>`: Repeatable files containing the contact info in the network.
///   * `name`: Display name of the account, only the first line is read.
///   * `time`: Last time `vCard` was updated.
///
/// If not specified, the content of the file must be encoded in UTF-8. Other rules include:
///
/// * Certificates must conform to X.509 and encoded in ASN.1 DER.
/// * Cryptographic keys must be contained in RFC 5958 PKCS #8 encoded in ASN.1 DER.
/// * Timestamps must be encoded in ISO 8601 date+time+timezone format.
///
/// Git is used to synchronize all data in a device profile between devices, while the synchronization of vCard between
/// rosters simply relies on comparing the update time. More information about synchronization can be found at the
/// corresponding sections of the documents.
pub struct Profile {
    path: PathBuf,
}
