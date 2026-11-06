//! Path helpers shared by the walker and discovery.

use std::cmp::Ordering;
use std::path::Path;

/// `path` relative to `root`, joined with `/` so results match across operating systems.
/// Returns an empty string for `root` itself.
pub fn relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Orders `/`-separated paths component by component, so a folder's children follow the
/// folder itself. The root (`""`) sorts first because its only component is empty.
pub fn compare(a: &str, b: &str) -> Ordering {
    a.split('/').cmp(b.split('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_sorts_before_everything() {
        assert_eq!(compare("", "apps/web"), Ordering::Less);
        assert_eq!(compare("apps/web", ""), Ordering::Greater);
        assert_eq!(compare("", ""), Ordering::Equal);
    }

    #[test]
    fn children_follow_their_parent() {
        let mut dirs = vec!["apps/web-admin", "apps/web/sub", "apps/web", "apps"];
        dirs.sort_by(|a, b| compare(a, b));
        assert_eq!(dirs, ["apps", "apps/web", "apps/web/sub", "apps/web-admin"]);
    }

    #[test]
    fn relative_uses_forward_slashes() {
        let root = Path::new("/project");
        let path = root.join("apps").join("web");
        assert_eq!(relative(root, &path), "apps/web");
        assert_eq!(relative(root, root), "");
    }
}
