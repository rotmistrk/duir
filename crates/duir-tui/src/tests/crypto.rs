use super::*;

#[test]
fn encrypt_sets_prompt() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    assert!(app.password_prompt.is_some());
}

#[test]
fn encrypt_then_decrypt_roundtrip() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    assert!(app.files[0].data.items[0].cipher.is_some());
    assert!(app.files[0].data.items[0].items.is_empty());

    app.cursor = 1;
    app.expand_current();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    assert!(app.files[0].data.items[0].unlocked);
    assert_eq!(app.files[0].data.items[0].items.len(), 2);
    assert_eq!(app.files[0].data.items[0].note, "branch1 note");
}

/// Tests the EXACT code path used in the real app:
/// prompt → stash (password, callback) → process on next iteration.
/// This is the path that broke TWICE due to callback being lost.
#[test]
fn encrypt_decrypt_via_pending_crypto_path() {
    let mut app = make_app_with_tree();
    app.cursor = 1;

    // Encrypt via pending_crypto (real app path)
    app.cmd_encrypt();
    assert!(app.password_prompt.is_some());
    let prompt = app.password_prompt.take().unwrap();
    app.pending_crypto = Some(("pass".to_owned(), prompt.callback));
    // Simulate: next iteration processes pending_crypto
    let (pw, action) = app.pending_crypto.take().unwrap();
    app.handle_password_result(&pw, action);

    assert!(app.files[0].data.items[0].cipher.is_some());
    assert!(app.files[0].data.items[0].items.is_empty());

    // Decrypt via pending_crypto (real app path)
    app.cursor = 1;
    app.expand_current();
    assert!(app.password_prompt.is_some());
    let prompt = app.password_prompt.take().unwrap();
    app.pending_crypto = Some(("pass".to_owned(), prompt.callback));
    let (pw, action) = app.pending_crypto.take().unwrap();
    app.handle_password_result(&pw, action);

    assert!(app.files[0].data.items[0].unlocked);
    assert_eq!(app.files[0].data.items[0].items.len(), 2);
    assert_eq!(app.files[0].data.items[0].note, "branch1 note");
}

#[test]
fn decrypt_wrong_password_no_corruption() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("correct", cb);
    }
    let cipher = app.files[0].data.items[0].cipher.clone();

    app.cursor = 1;
    app.expand_current();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("wrong", cb);
    }
    assert_eq!(app.files[0].data.items[0].cipher, cipher);
    assert!(app.files[0].data.items[0].items.is_empty());
    assert_eq!(app.status_level, StatusLevel::Error);
}

#[test]
fn collapse_encrypted_relocks() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    app.cursor = 1;
    app.expand_current();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    assert!(app.files[0].data.items[0].unlocked);

    app.cursor = 1;
    app.collapse_current();
    assert!(!app.files[0].data.items[0].unlocked);
    assert!(app.files[0].data.items[0].items.is_empty());
}

#[test]
fn decrypt_requires_unlock() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    app.cmd_decrypt();
    assert_eq!(app.status_level, StatusLevel::Warning);
    assert!(app.files[0].data.items[0].cipher.is_some());
}

#[test]
fn save_reencrypts_unlocked() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.cmd_encrypt();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }
    app.cursor = 1;
    app.expand_current();
    {
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
    }

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::FileStorage::new(dir.path()).unwrap();
    app.save_all(&storage);

    let loaded = storage.load("test").unwrap();
    assert!(loaded.items[0].cipher.is_some());
    assert!(loaded.items[0].items.is_empty());
    // In memory still unlocked
    assert!(app.files[0].data.items[0].unlocked);
}
