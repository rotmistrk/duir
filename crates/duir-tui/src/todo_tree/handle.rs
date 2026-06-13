//! Key handling — maps keys to tree operations.

use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

use super::data::TodoTreeData;
use super::model::{self, Completion, TodoItem};
use duir_core::model::WorkStatus;

/// Action to take after handling a key.
pub enum HandleAction {
    Stay,
    MoveTo(usize),
    EditNew(usize),
    EnterFilter,
    /// Prompt for passphrase (encrypt or decrypt).
    CryptoPrompt(model::TreePath, CryptoMode),
}

/// Whether to encrypt or decrypt.
pub enum CryptoMode {
    Encrypt,
    Decrypt,
}

/// Process todo-specific keys. Returns `Some(action)` if consumed.
pub fn handle_todo_key(key: &KeyEvent, data: &mut TodoTreeData, cursor: usize) -> Option<HandleAction> {
    let id = data.visible_id(cursor);
    match key.code() {
        KeyCode::Char('K') => shift_move(data, id, model::swap_up),
        KeyCode::Char('J') => shift_move(data, id, model::swap_down),
        KeyCode::Char('H') => shift_move(data, id, model::promote),
        KeyCode::Char('L') => shift_move(data, id, model::demote),
        KeyCode::Up if key.modifiers().shift() => shift_move(data, id, model::swap_up),
        KeyCode::Down if key.modifiers().shift() => shift_move(data, id, model::swap_down),
        KeyCode::Left if key.modifiers().shift() => shift_move(data, id, model::promote),
        KeyCode::Right if key.modifiers().shift() => shift_move(data, id, model::demote),
        KeyCode::Char(' ') => toggle_complete(data, id),
        KeyCode::Char('n') => new_sibling(data, id, cursor),
        KeyCode::Char('b') => new_child(data, id, cursor),
        KeyCode::Char('d') => delete(data, id, cursor),
        KeyCode::Char('S') => sort(data, id),
        KeyCode::Char('/') => Some(HandleAction::EnterFilter),
        KeyCode::Char('!') => toggle_priority_5(data, id),
        KeyCode::Char('+') => priority_up(data, id),
        KeyCode::Char('-') => priority_down(data, id),
        KeyCode::Char('>') => loe_up(data, id),
        KeyCode::Char('<') => loe_down(data, id),
        KeyCode::Char('i' | '=') => toggle_progress(data, id),
        KeyCode::Char('\\') => toggle_pause(data, id),
        KeyCode::Char('l') if key.modifiers().ctrl() => crypto_prompt(data, id),
        _ => None,
    }
}

fn shift_move(
    data: &mut TodoTreeData,
    id: usize,
    op: fn(&mut model::TodoFile, &model::TreePath) -> Option<model::TreePath>,
) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let new_path = op(&mut data.file, &path)?;
    data.save();
    data.rebuild_flat();
    data.row_for_path(&new_path).map(HandleAction::MoveTo)
}

fn toggle_complete(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    item.completed = match item.completed {
        Completion::Done => Completion::Open,
        _ => Completion::Done,
    };
    model::propagate_completion(&mut data.file, &path);
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn toggle_priority_5(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    let current = item.priority.unwrap_or(0);
    item.priority = Some(if current == 5 { 0 } else { 5 });
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn priority_up(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    let current = item.priority.unwrap_or(0);
    let new_val = current.saturating_add(1).min(9);
    item.priority = if new_val == 0 { None } else { Some(new_val) };
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn priority_down(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    let current = item.priority.unwrap_or(0);
    let new_val = current.saturating_sub(1);
    item.priority = if new_val == 0 { None } else { Some(new_val) };
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn loe_up(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    const FIB: &[u8] = &[0, 1, 2, 3, 5, 8, 13, 21];
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    let current = item.effort.unwrap_or(0);
    let idx = FIB.iter().position(|&v| v >= current).unwrap_or(0);
    let new_idx = (idx + 1).min(FIB.len() - 1);
    let new_val = FIB.get(new_idx).copied().unwrap_or(0);
    item.effort = if new_val == 0 { None } else { Some(new_val) };
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn loe_down(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    const FIB: &[u8] = &[0, 1, 2, 3, 5, 8, 13, 21];
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    let current = item.effort.unwrap_or(0);
    let idx = FIB.iter().position(|&v| v >= current).unwrap_or(0);
    let new_idx = idx.saturating_sub(1);
    let new_val = FIB.get(new_idx).copied().unwrap_or(0);
    item.effort = if new_val == 0 { None } else { Some(new_val) };
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn new_sibling(data: &mut TodoTreeData, id: usize, cursor: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let new_item = TodoItem::new("<new task>");
    if !model::add_sibling(&mut data.file, &path, new_item) {
        return Some(HandleAction::Stay);
    }
    let mut new_path = path;
    if let Some(last) = new_path.last_mut() {
        *last += 1;
    }
    model::propagate_completion(&mut data.file, &new_path);
    data.save();
    data.rebuild_flat();
    let row = data.row_for_path(&new_path).unwrap_or(cursor + 1);
    Some(HandleAction::EditNew(row))
}

fn new_child(data: &mut TodoTreeData, id: usize, cursor: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let child_idx = model::get_item(&data.file, &path).map_or(0, |item| item.items.len());
    let new_item = TodoItem::new("<new task>");
    if !model::add_child(&mut data.file, &path, new_item) {
        return Some(HandleAction::Stay);
    }
    if let Some(item) = model::get_item_mut(&mut data.file, &path) {
        item.folded = false;
    }
    let mut new_path = path;
    new_path.push(child_idx);
    model::propagate_completion(&mut data.file, &new_path);
    data.save();
    data.rebuild_flat();
    let row = data.row_for_path(&new_path).unwrap_or(cursor + 1);
    Some(HandleAction::EditNew(row))
}

fn delete(data: &mut TodoTreeData, id: usize, cursor: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    model::remove_item(&mut data.file, &path)?;
    model::propagate_completion(&mut data.file, &path);
    data.save();
    data.rebuild_flat();
    let max = data.visible_count().saturating_sub(1);
    Some(HandleAction::MoveTo(cursor.min(max)))
}

fn sort(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    model::sort_children(&mut data.file, &path);
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn toggle_progress(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    if !item.items.is_empty() {
        return Some(HandleAction::Stay);
    }
    let now = now_epoch();
    if item.work_status == WorkStatus::InProgress {
        if let Some(started) = item.progress_started_at.take() {
            item.time_spent_secs += now.saturating_sub(started);
        }
        item.work_status = WorkStatus::Idle;
    } else {
        if item.started_at.is_none() {
            item.started_at = Some(now);
        }
        item.progress_started_at = Some(now);
        item.work_status = WorkStatus::InProgress;
    }
    item.updated_at = Some(now);
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn toggle_pause(data: &mut TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item_mut(&mut data.file, &path)?;
    if !item.items.is_empty() {
        return Some(HandleAction::Stay); // leaf-only
    }
    let now = now_epoch();
    match item.work_status {
        WorkStatus::Paused => {
            item.work_status = WorkStatus::Idle;
        }
        WorkStatus::InProgress => {
            // Pause: accumulate time first
            if let Some(started) = item.progress_started_at.take() {
                item.time_spent_secs += now.saturating_sub(started);
            }
            item.work_status = WorkStatus::Paused;
        }
        WorkStatus::Idle => {
            item.work_status = WorkStatus::Paused;
        }
    }
    item.updated_at = Some(now);
    data.save();
    data.rebuild_flat();
    Some(HandleAction::Stay)
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn crypto_prompt(data: &TodoTreeData, id: usize) -> Option<HandleAction> {
    let path = data.path_at(id)?.clone();
    let item = model::get_item(&data.file, &path)?;
    let mode = if item.is_locked() {
        CryptoMode::Decrypt
    } else {
        CryptoMode::Encrypt
    };
    Some(HandleAction::CryptoPrompt(path, mode))
}
