use std::collections::BTreeMap;

#[derive(Clone)]
pub struct FileEntry {
    pub mode: &'static str,
    pub owner: &'static str,
    pub group: &'static str,
    pub size: u64,
    pub modified: &'static str,
}

impl FileEntry {
    pub fn new(
        mode: &'static str,
        owner: &'static str,
        group: &'static str,
        size: u64,
        modified: &'static str,
    ) -> Self {
        Self {
            mode,
            owner,
            group,
            size,
            modified,
        }
    }
}

#[derive(Clone)]
pub struct DirEntry {
    pub mode: &'static str,
    pub owner: &'static str,
    pub group: &'static str,
    pub size: u64,
    pub modified: &'static str,
    pub children: BTreeMap<String, DirItem>,
}

impl DirEntry {
    pub fn new(
        mode: &'static str,
        owner: &'static str,
        group: &'static str,
        size: u64,
        modified: &'static str,
    ) -> Self {
        Self {
            mode,
            owner,
            group,
            size,
            modified,
            children: BTreeMap::new(),
        }
    }

    pub fn with_children(mut self, children: BTreeMap<String, DirItem>) -> Self {
        self.children = children;
        self
    }
}

#[derive(Clone)]
pub enum DirItem {
    Dir(DirEntry),
    File(FileEntry),
}

#[derive(Clone)]
pub struct FakeFileSystem {
    root: DirEntry,
}

impl FakeFileSystem {
    pub fn new(root: DirEntry) -> Self {
        Self { root }
    }

    pub fn display_path(&self, path: &[String]) -> String {
        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path.join("/"))
        }
    }

    pub fn prompt_path(&self, path: &[String]) -> String {
        match path {
            [] => "/".to_string(),
            [home, user] if home == "home" && user == "ubuntu" => "~".to_string(),
            [home, user, rest @ ..] if home == "home" && user == "ubuntu" => {
                format!("~/{}", rest.join("/"))
            }
            _ => self.display_path(path),
        }
    }

    pub fn resolve_path(&self, cwd: &[String], target: &str) -> Result<Vec<String>, String> {
        let mut path = self.base_path(cwd, target);
        self.push_segments(&mut path, target);

        match self.get_item(&path) {
            _ if path.is_empty() => Ok(path),
            Some(DirItem::Dir(_)) => Ok(path),
            Some(DirItem::File(_)) => Err(format!("cd: {}: Not a directory", target)),
            None => Err(format!("cd: {}: No such file or directory", target)),
        }
    }

    pub fn list_dir<'a>(
        &'a self,
        cwd: &[String],
        target: Option<&str>,
        show_all: bool,
    ) -> Result<Vec<ListEntry<'a>>, String> {
        if let Some(path) = target {
            let resolved = self.resolve_listing_path(cwd, path)?;
            if let Some(item) = self.get_item(&resolved) {
                if !matches!(item, DirItem::Dir(_)) {
                    let name = resolved
                        .last()
                        .expect("resolved file path has a final segment");
                    return Ok(vec![ListEntry::from_item(name, item)]);
                }
            }
        }

        let resolved = match target {
            Some(path) => self.resolve_listing_path(cwd, path)?,
            None => cwd.to_vec(),
        };

        let dir = self
            .get_dir(&resolved)
            .ok_or_else(|| match self.get_item(&resolved) {
                None => format!(
                    "ls: cannot access '{}': No such file or directory",
                    target.unwrap_or_default()
                ),
                Some(DirItem::Dir(_)) | Some(DirItem::File(_)) => unreachable!(),
            })?;

        let mut entries = Vec::new();
        if show_all {
            entries.push(ListEntry::virtual_dir(".", dir));
            entries.push(ListEntry::virtual_parent("..", self.parent_dir(&resolved)));
        }

        for (name, item) in &dir.children {
            if !show_all && name.starts_with('.') {
                continue;
            }
            entries.push(ListEntry::from_item(name, item));
        }

        Ok(entries)
    }

    pub fn get_dir(&self, path: &[String]) -> Option<&DirEntry> {
        if path.is_empty() {
            return Some(&self.root);
        }

        match self.get_item(path) {
            Some(DirItem::Dir(dir)) => Some(dir),
            _ => None,
        }
    }

    fn resolve_listing_path(&self, cwd: &[String], target: &str) -> Result<Vec<String>, String> {
        let mut path = self.base_path(cwd, target);
        self.push_segments(&mut path, target);

        if self.get_dir(&path).is_some() || self.get_item(&path).is_some() {
            Ok(path)
        } else {
            Err(format!(
                "ls: cannot access '{}': No such file or directory",
                target
            ))
        }
    }

    fn parent_dir(&self, path: &[String]) -> &DirEntry {
        if path.is_empty() {
            &self.root
        } else {
            self.get_dir(&path[..path.len() - 1]).unwrap_or(&self.root)
        }
    }

    fn base_path(&self, cwd: &[String], target: &str) -> Vec<String> {
        if target.starts_with('/') {
            Vec::new()
        } else if target == "~" || target.starts_with("~/") {
            self.home_path()
        } else {
            cwd.to_vec()
        }
    }

    fn push_segments(&self, path: &mut Vec<String>, target: &str) {
        let trimmed = target
            .strip_prefix("~/")
            .or_else(|| target.strip_prefix('~'))
            .unwrap_or(target);

        for part in trimmed.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    path.pop();
                }
                segment => path.push(segment.to_string()),
            }
        }
    }

    fn home_path(&self) -> Vec<String> {
        vec!["home".to_string(), "ubuntu".to_string()]
    }

    fn get_item(&self, path: &[String]) -> Option<&DirItem> {
        let mut current = &self.root;
        for (idx, segment) in path.iter().enumerate() {
            let item = current.children.get(segment)?;
            if idx == path.len() - 1 {
                return Some(item);
            }
            match item {
                DirItem::Dir(dir) => current = dir,
                DirItem::File(_) => return None,
            }
        }

        None
    }
}

pub struct ListEntry<'a> {
    pub name: String,
    pub mode: &'a str,
    pub owner: &'a str,
    pub group: &'a str,
    pub size: u64,
    pub modified: &'a str,
    pub is_dir: bool,
}

impl<'a> ListEntry<'a> {
    fn from_item(name: impl Into<String>, item: &'a DirItem) -> Self {
        match item {
            DirItem::Dir(dir) => Self {
                name: name.into(),
                mode: dir.mode,
                owner: dir.owner,
                group: dir.group,
                size: dir.size,
                modified: dir.modified,
                is_dir: true,
            },
            DirItem::File(file) => Self {
                name: name.into(),
                mode: file.mode,
                owner: file.owner,
                group: file.group,
                size: file.size,
                modified: file.modified,
                is_dir: false,
            },
        }
    }

    fn virtual_dir(name: impl Into<String>, dir: &'a DirEntry) -> Self {
        Self {
            name: name.into(),
            mode: dir.mode,
            owner: dir.owner,
            group: dir.group,
            size: dir.size,
            modified: dir.modified,
            is_dir: true,
        }
    }

    fn virtual_parent(name: impl Into<String>, dir: &'a DirEntry) -> Self {
        Self {
            name: name.into(),
            mode: dir.mode,
            owner: dir.owner,
            group: dir.group,
            size: dir.size,
            modified: dir.modified,
            is_dir: true,
        }
    }
}
