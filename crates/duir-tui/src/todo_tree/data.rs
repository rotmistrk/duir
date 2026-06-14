//! `TodoTreeData` — tree data provider backed by duir-core's `TodoFile`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use txv_core::cell::{Attrs, Style};
use txv_widgets::tree_view::TreeData;

use super::flat_node::FlatNode;
use super::model::{self, Completion, TodoFile, TodoItem, TreePath};

/// Data provider for the todo tree.
pub struct TodoTreeData {
    pub(crate) file: TodoFile,
    file_path: PathBuf,
    pub(crate) trash_path: Option<PathBuf>,
    pub(super) nodes: Vec<FlatNode>,
    visible: Vec<usize>,
    pub(crate) filter_text: String,
    pub(super) badges: Vec<String>,
    pub(super) timestamps: Vec<[String; 3]>,
    pub(crate) show_timestamps: bool,
    last_mtime: Option<SystemTime>,
}

impl TodoTreeData {
    #[must_use]
    pub fn new(file_path: &Path) -> Self {
        let file = model::load_todo_file(file_path);
        let mtime = Self::read_mtime(file_path);
        let trash_path = file_path.parent().map(|p| p.join("trash.json"));
        let mut data = Self {
            file,
            file_path: file_path.to_path_buf(),
            trash_path,
            nodes: Vec::new(),
            visible: Vec::new(),
            filter_text: String::new(),
            badges: Vec::new(),
            timestamps: Vec::new(),
            show_timestamps: false,
            last_mtime: mtime,
        };
        data.rebuild_flat();
        data
    }

    pub fn save(&mut self) {
        if !model::save_todo_file(&self.file_path, &self.file) {
            log::error!("Failed to save todo file: {}", self.file_path.display());
        }
        self.last_mtime = Self::read_mtime(&self.file_path);
    }

    pub fn reload_if_changed(&mut self) -> bool {
        let current = Self::read_mtime(&self.file_path);
        if current == self.last_mtime {
            return false;
        }
        self.file = model::load_todo_file(&self.file_path);
        self.last_mtime = current;
        self.rebuild_flat();
        true
    }

    pub fn rebuild_flat(&mut self) {
        self.nodes.clear();
        Self::flatten(&self.file.items.clone(), &[], &mut self.nodes);
        self.rebuild_badges();
        self.rebuild_timestamps();
        self.rebuild_visible();
    }

    fn flatten(items: &[TodoItem], parent_path: &[usize], out: &mut Vec<FlatNode>) {
        let count = items.len();
        for (i, item) in items.iter().enumerate() {
            let mut path = parent_path.to_vec();
            path.push(i);
            let depth = path.len() - 1;
            out.push(FlatNode {
                depth,
                path: path.clone(),
                expandable: !item.items.is_empty(),
                expanded: !item.folded,
                is_last_child: i == count - 1,
            });
            if !item.items.is_empty() && !item.folded {
                Self::flatten(&item.items, &path, out);
            }
        }
    }

    fn rebuild_visible(&mut self) {
        if self.filter_text.is_empty() {
            self.visible = (0..self.nodes.len()).collect();
        } else {
            let filter = self.filter_text.to_lowercase();
            self.visible = (0..self.nodes.len())
                .filter(|&i| {
                    self.item_at(i)
                        .is_some_and(|item| item.title.to_lowercase().contains(&filter))
                })
                .collect();
        }
    }

    #[must_use]
    pub fn path_at(&self, id: usize) -> Option<&TreePath> {
        self.nodes.get(id).map(|n| &n.path)
    }

    #[must_use]
    pub fn item_at(&self, id: usize) -> Option<&TodoItem> {
        let path = self.path_at(id)?;
        model::get_item(&self.file, path)
    }

    #[must_use]
    pub fn row_for_path(&self, path: &TreePath) -> Option<usize> {
        self.visible
            .iter()
            .position(|&i| self.nodes.get(i).is_some_and(|n| n.path == *path))
    }

    pub fn update_title(&mut self, row: usize, title: String) {
        let id = self.visible_id(row);
        if let Some(path) = self.path_at(id).cloned() {
            if let Some(item) = model::get_item_mut(&mut self.file, &path) {
                item.title = title;
            }
            self.save();
            self.rebuild_flat();
        }
    }

    pub fn add_first_item(&mut self) {
        self.file.items.push(TodoItem::new("<new task>"));
        self.save();
        self.rebuild_flat();
    }

    fn read_mtime(path: &Path) -> Option<SystemTime> {
        std::fs::metadata(path).ok()?.modified().ok()
    }
}

impl TreeData for TodoTreeData {
    fn root_count(&self) -> usize {
        self.file.items.len()
    }

    fn child_count(&self, id: usize) -> usize {
        self.item_at(id).map_or(0, |item| item.items.len())
    }

    fn label(&self, id: usize) -> &str {
        self.item_at(id).map_or("", |item| &item.title)
    }

    fn is_expandable(&self, id: usize) -> bool {
        self.nodes.get(id).is_some_and(|n| n.expandable)
    }

    fn is_expanded(&self, id: usize) -> bool {
        self.nodes.get(id).is_some_and(|n| n.expanded)
    }

    fn toggle(&mut self, id: usize) {
        if let Some(path) = self.path_at(id).cloned() {
            if let Some(item) = model::get_item_mut(&mut self.file, &path) {
                item.folded = !item.folded;
            }
            self.save();
            self.rebuild_flat();
        }
    }

    fn depth(&self, id: usize) -> usize {
        self.nodes.get(id).map_or(0, |n| n.depth)
    }

    fn visible_count(&self) -> usize {
        self.visible.len()
    }

    fn visible_id(&self, row: usize) -> usize {
        self.visible.get(row).copied().unwrap_or(0)
    }

    fn style(&self, id: usize) -> Style {
        let Some(item) = self.item_at(id) else {
            return Style::default();
        };
        match item.completed {
            Completion::Done => Style::default().with_attrs(Attrs::default().dim()),
            _ if item.important => Style::default().with_attrs(Attrs::default().bold()),
            _ => Style::default(),
        }
    }
}
