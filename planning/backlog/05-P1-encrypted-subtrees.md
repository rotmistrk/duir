# Epic: Encrypted Subtrees

**ID**: 05
**Priority**: P1
**Status**: backlog

## Goal

Support per-subtree encryption with hierarchical password protection.
Encrypted nodes store ciphertext in the JSON file. Decryption happens
in memory on demand, and keys are forgotten when the subtree is collapsed.

## Design

### Data Model

An encrypted item stores its children and note as an opaque encrypted blob:

```json
{
  "title": "Secret Project",
  "encrypted": true,
  "cipher": "base64-encoded-encrypted-payload",
  "items": []
}
```

When decrypted in memory, `cipher` is decoded and `items`/`note` are populated.
On collapse or save, the plaintext is re-encrypted and `items` cleared from JSON.

### Hierarchical Encryption

- A branch can have password P1 protecting its children
- A sub-branch within can have password P2 protecting deeper children
- Opening the branch requires P1; opening the sub-branch requires P1 then P2
- Each level is independently encrypted — P2 content is encrypted inside P1's payload

### Key Management

- Password prompted via a modal dialog when expanding an encrypted node
- Password stored in memory only, associated with the node's path
- Password forgotten when:
  - Node is collapsed
  - File is closed
  - App exits
- No password storage on disk, ever

### Operations

| Command | Action |
|---------|--------|
| `:encrypt` | Encrypt current subtree (prompts for password) |
| `:encrypt` on already-encrypted | Change password (warns first) |
| `:decrypt` | Permanently decrypt subtree (remove encryption) |
| Expand encrypted node | Prompts for password |
| Collapse encrypted node | Forgets password, re-encrypts |

### Move Safety

- Moving an item OUT of an encrypted subtree to an unencrypted location:
  **WARN** "This item will no longer be encrypted. Continue?"
- Moving an item INTO an encrypted subtree:
  Item becomes part of the encrypted payload (no warning needed)
- Moving between differently-encrypted subtrees:
  **WARN** "Item will be re-encrypted with the target's password. Continue?"

### Crypto

- Use `age` crate (modern, audited, passphrase-based encryption)
- Or `aes-gcm` with Argon2 key derivation
- Prefer `age` for simplicity and security

## Acceptance Criteria

- [ ] `:encrypt` encrypts current subtree with prompted password
- [ ] Expanding encrypted node prompts for password
- [ ] Collapsing encrypted node forgets password and re-encrypts
- [ ] Encrypted data stored as base64 blob in JSON
- [ ] Hierarchical encryption: nested encrypted subtrees work independently
- [ ] `:encrypt` on already-encrypted subtree changes password (with warning)
- [ ] `:decrypt` permanently removes encryption (with warning)
- [ ] Moving items out of encrypted subtree warns user
- [ ] Password never written to disk
- [ ] Wrong password shows error, does not corrupt data

## Stories

- [ ] 05.001 — Encrypted item model and serialization
- [ ] 05.002 — Password prompt modal widget
- [ ] 05.003 — Encrypt/decrypt commands
- [ ] 05.004 — Expand/collapse with auto-encrypt/decrypt
- [ ] 05.005 — Move safety warnings
- [ ] 05.006 — Hierarchical (nested) encryption
- [ ] 05.007 — Crypto implementation (age or aes-gcm+argon2)

## Notes

- This is security-sensitive — needs careful review
- Consider: what happens if user forgets password? Data is lost. Document this clearly.
- The encrypted blob should include a version tag for future algorithm changes
- Test with large subtrees to ensure performance is acceptable

### Markdown Conversion Safety

Collapse/expand and export MUST be blocked when encryption is involved:

| Operation | On encrypted node | On descendant of encrypted | On ancestor of encrypted |
|-----------|-------------------|---------------------------|--------------------------|
| `:collapse` | REFUSE | REFUSE | REFUSE |
| `:expand` | REFUSE | REFUSE | REFUSE |
| `:export md` | REFUSE | REFUSE | REFUSE |
| `:import md` | ALLOW (content joins encrypted payload) | ALLOW | ALLOW |

Rationale: Converting to markdown would expose plaintext that encryption
is meant to protect. Even if the node itself isn't encrypted, its ancestor's
encryption implies the content should stay within the encrypted boundary.

Implementation: Before any collapse/expand/export, walk up the tree to check
for encrypted ancestors, and walk down to check for encrypted descendants.
If any are found, refuse with a clear error message.
