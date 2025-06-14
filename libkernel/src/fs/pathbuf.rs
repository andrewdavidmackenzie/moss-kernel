//! A module for owned, mutable paths.
//!
//! This module provides a `PathBuf` struct that is an owned, mutable counterpart
//! to the `Path` slice. It provides methods for manipulating the path in place,
//! such as `push` and `pop`.

use super::path::Path;
use alloc::string::String;
use core::ops::Deref;

/// An owned, mutable path, akin to `String`.
///
/// This struct provides methods like `push` and `pop` that mutate the path
/// in place. It also implements `Deref` to `Path`, meaning that all methods
/// on `Path` slices are available on `PathBuf` values as well.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone, Default)]
pub struct PathBuf {
    inner: String,
}

impl PathBuf {
    /// Creates a new, empty `PathBuf`.
    ///
    /// # Examples
    ///
    /// ```
    /// use libkernel::fs::pathbuf::PathBuf;
    ///
    /// let path = PathBuf::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: String::new(),
        }
    }

    /// Creates a new `PathBuf` with a given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: String::with_capacity(capacity),
        }
    }

    /// Coerces to a `Path` slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use libkernel::fs::path::Path;
    /// use libkernel::fs::pathbuf::PathBuf;
    ///
    /// let p = PathBuf::from("/test");
    /// assert_eq!(Path::new("/test"), p.as_path());
    /// ```
    pub fn as_path(&self) -> &Path {
        self
    }

    /// Extends `self` with `path`.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// # Examples
    ///
    /// ```
    /// use libkernel::fs::pathbuf::PathBuf;
    ///
    /// let mut path = PathBuf::from("/usr");
    /// path.push("bin");
    /// assert_eq!(path.as_str(), "/usr/bin");
    ///
    /// let mut path2 = PathBuf::from("/tmp");
    /// path2.push("/etc/passwd");
    /// assert_eq!(path2.as_str(), "/etc/passwd");
    /// ```
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();

        if path.is_absolute() {
            self.inner = path.as_str().into();
            return;
        }

        if !self.inner.is_empty() && !self.inner.ends_with('/') {
            self.inner.push('/');
        }

        self.inner.push_str(path.as_str());
    }

    /// Truncates `self` to its parent.
    ///
    /// Returns `true` if the path was truncated, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use libkernel::fs::pathbuf::PathBuf;
    ///
    /// let mut path = PathBuf::from("/a/b/c");
    /// assert!(path.pop());
    /// assert_eq!(path.as_str(), "/a/b");
    /// assert!(path.pop());
    /// assert_eq!(path.as_str(), "/a");
    /// assert!(!path.pop());
    /// assert_eq!(path.as_str(), "/a");
    /// ```
    pub fn pop(&mut self) -> bool {
        match self.as_path().parent() {
            Some(parent) => {
                self.inner.truncate(parent.as_str().len());
                true
            }
            None => false,
        }
    }

    /// Updates the file name of the path.
    ///
    /// If there is no file name, it is appended. Otherwise, the existing
    /// file name is replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use libkernel::fs::pathbuf::PathBuf;
    ///
    /// let mut path = PathBuf::from("/tmp/foo");
    /// path.set_file_name("bar.txt");
    /// assert_eq!(path.as_str(), "/tmp/bar.txt");
    ///
    /// let mut path2 = PathBuf::from("/tmp");
    /// path2.set_file_name("foo");
    /// assert_eq!(path2.as_str(), "/tmp/foo");
    /// ```
    pub fn set_file_name<S: AsRef<str>>(&mut self, file_name: S) {
        if self.as_path().file_name().is_some() {
            self.pop();
        }

        self.push(Path::new(file_name.as_ref()));
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl<T: AsRef<str>> From<T> for PathBuf {
    fn from(s: T) -> Self {
        Self {
            inner: s.as_ref().into(),
        }
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        Path::new(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::PathBuf;

    #[test]
    fn test_push() {
        let mut p = PathBuf::from("/a/b");
        p.push("c");
        assert_eq!(p.as_str(), "/a/b/c");

        let mut p2 = PathBuf::from("/a/b/");
        p2.push("c");
        assert_eq!(p2.as_str(), "/a/b/c");

        let mut p3 = PathBuf::from("a");
        p3.push("b");
        assert_eq!(p3.as_str(), "a/b");

        let mut p4 = PathBuf::new();
        p4.push("a");
        assert_eq!(p4.as_str(), "a");

        let mut p5 = PathBuf::from("/a/b/");
        p5.push("c/d");
        assert_eq!(p5.as_str(), "/a/b/c/d");
    }

    #[test]
    fn test_pop() {
        let mut p = PathBuf::from("/a/b/c");
        assert!(p.pop());
        assert_eq!(p.as_str(), "/a/b");
        assert!(p.pop());
        assert_eq!(p.as_str(), "/a");
        assert!(!p.pop());
        assert_eq!(p.as_str(), "/a");

        let mut p2 = PathBuf::from("a/b");
        assert!(p2.pop());
        assert_eq!(p2.as_str(), "a");
        assert!(!p2.pop());
    }

    #[test]
    fn test_set_file_name() {
        let mut p = PathBuf::from("/a/b");
        p.set_file_name("c");
        assert_eq!(p.as_str(), "/a/c");

        let mut p2 = PathBuf::from("/a/");
        p2.set_file_name("b");
        assert_eq!(p2.as_str(), "/a/b");
    }

    #[test]
    fn test_deref() {
        let p = PathBuf::from("/a/b/c");
        assert!(p.is_absolute());
    }
}
