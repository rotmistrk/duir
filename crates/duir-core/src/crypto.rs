use std::io::{Read, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::error::{OmelaError, Result};
use crate::model::TodoItem;

/// Payload that gets encrypted: the children and note of a node.
#[derive(serde::Serialize, serde::Deserialize)]
struct EncryptedPayload {
    note: String,
    items: Vec<TodoItem>,
}

/// Encrypt a node's children and note with a passphrase.
/// Sets `cipher` and clears `items`/`note` on the item.
///
/// # Errors
/// Returns an error if encryption fails.
pub fn encrypt_item(item: &mut TodoItem, passphrase: &str) -> Result<()> {
    let payload = EncryptedPayload {
        note: item.note.clone(),
        items: item.items.clone(),
    };
    let json = serde_json::to_string(&payload)?;

    let recipient = age::scrypt::Recipient::new(age::secrecy::SecretString::from(passphrase.to_owned()));

    let recipients: Vec<Box<dyn age::Recipient>> = vec![Box::new(recipient)];
    let encryptor = age::Encryptor::with_recipients(recipients.iter().map(std::convert::AsRef::as_ref))
        .map_err(|e| OmelaError::Other(format!("Encryption setup error: {e}")))?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| OmelaError::Other(format!("Encryption error: {e}")))?;

    writer
        .write_all(json.as_bytes())
        .map_err(|e| OmelaError::Other(format!("Encryption write error: {e}")))?;
    writer
        .finish()
        .map_err(|e| OmelaError::Other(format!("Encryption finish error: {e}")))?;

    item.cipher = Some(BASE64.encode(&encrypted));
    item.items.clear();
    item.note.clear();
    item.unlocked = false;
    item.folded = true;

    Ok(())
}

/// Decrypt a node's cipher text with a passphrase.
/// Restores `items` and `note`, marks as unlocked.
///
/// # Errors
/// Returns an error if decryption fails (wrong password or corrupt data).
pub fn decrypt_item(item: &mut TodoItem, passphrase: &str) -> Result<()> {
    let cipher_b64 = item
        .cipher
        .as_ref()
        .ok_or_else(|| OmelaError::Other("Node is not encrypted".to_owned()))?;

    let encrypted = BASE64
        .decode(cipher_b64)
        .map_err(|e| OmelaError::Other(format!("Base64 decode error: {e}")))?;

    let decryptor =
        age::Decryptor::new(encrypted.as_slice()).map_err(|e| OmelaError::Other(format!("Decryption error: {e}")))?;

    let identity = age::scrypt::Identity::new(age::secrecy::SecretString::from(passphrase.to_owned()));

    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| OmelaError::Other("Wrong password".to_owned()))?;

    let mut json = String::new();
    reader
        .read_to_string(&mut json)
        .map_err(|e| OmelaError::Other(format!("Decryption read error: {e}")))?;

    let payload: EncryptedPayload = serde_json::from_str(&json)?;

    item.note = payload.note;
    item.items = payload.items;
    item.unlocked = true;
    item.folded = false;

    Ok(())
}

/// Remove encryption from a node permanently.
pub fn strip_encryption(item: &mut TodoItem) {
    item.cipher = None;
    item.unlocked = false;
}

/// Check if any node in a subtree is encrypted.
#[must_use]
pub fn has_encrypted_in_subtree(item: &TodoItem) -> bool {
    if item.is_encrypted() {
        return true;
    }
    item.items.iter().any(has_encrypted_in_subtree)
}

/// Saved plaintext state of a node: (path, note, children).
pub type SavedNodeState = (Vec<usize>, String, Vec<TodoItem>);

/// Re-encrypt all unlocked nodes in a tree before saving.
/// Returns the items that were re-encrypted so they can be restored after save.
///
/// # Errors
/// Returns an error if encryption fails.
pub fn lock_for_save<S: std::hash::BuildHasher>(
    items: &mut [TodoItem],
    passwords: &std::collections::HashMap<Vec<usize>, String, S>,
    path_prefix: &[usize],
) -> crate::Result<Vec<SavedNodeState>> {
    let mut saved_state = Vec::new();

    for (i, item) in items.iter_mut().enumerate() {
        let mut path = path_prefix.to_vec();
        path.push(i);

        // Recurse into children first
        let child_saved = lock_for_save(&mut item.items, passwords, &path)?;
        saved_state.extend(child_saved);

        // If this node is unlocked, prepare it for save
        if item.unlocked
            && let Some(pw) = passwords.get(&path)
        {
            let note_backup = item.note.clone();
            let items_backup = item.items.clone();

            // Only re-encrypt if cipher is stale (None = invalidated by edits)
            if item.cipher.is_none() {
                encrypt_item(item, pw)?;
            } else {
                // Cipher is still valid — just clear plaintext for save
                item.items.clear();
                item.note.clear();
                item.unlocked = false;
                item.folded = true;
            }

            saved_state.push((path, note_backup, items_backup));
        }
    }

    Ok(saved_state)
}

/// Restore decrypted state after save.
/// The cipher is kept valid since it was just written to disk.
pub fn restore_after_save(items: &mut [TodoItem], saved: &[SavedNodeState]) {
    for (path, note, children) in saved {
        if let Some(item) = navigate_mut(items, path) {
            item.note.clone_from(note);
            item.items.clone_from(children);
            item.unlocked = true;
            item.folded = false;
            // cipher stays valid — it matches the content we just saved
        }
    }
}

/// Invalidate the cipher cache for an encrypted ancestor.
/// Call this when any modification is made inside an unlocked encrypted subtree.
pub fn invalidate_cipher(item: &mut TodoItem) {
    if item.unlocked && item.cipher.is_some() {
        item.cipher = None;
    }
}

fn navigate_mut<'a>(items: &'a mut [TodoItem], path: &[usize]) -> Option<&'a mut TodoItem> {
    let (&first, rest) = path.split_first()?;
    let mut current = items.get_mut(first)?;
    for &idx in rest {
        current = current.items.get_mut(idx)?;
    }
    Some(current)
}
