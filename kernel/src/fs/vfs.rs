use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

pub trait FileSystemDriver {
    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, String>;
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), String>;
    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String>;
    fn create_file(&mut self, path: &str) -> Result<(), String>;
    fn create_dir(&mut self, path: &str) -> Result<(), String>;
    fn remove_file(&mut self, path: &str) -> Result<(), String>;
}

pub struct Vfs {
    mounts: Vec<(String, Box<dyn FileSystemDriver>)>,
}

impl Vfs {
    pub fn new() -> Self {
        Vfs { mounts: Vec::new() }
    }

    pub fn mount(&mut self, mount_point: &str, fs: Box<dyn FileSystemDriver>) {
        let mut mp = String::from(mount_point);
        if !mp.starts_with('/') {
            mp = String::from("/") + &mp;
        }
        if mp.len() > 1 && mp.ends_with('/') {
            mp.pop();
        }
        
        self.mounts.push((mp, fs));
        self.mounts.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    }

    fn resolve_mount<'a>(&'a mut self, path: &'a str) -> Result<(&'a mut Box<dyn FileSystemDriver>, &'a str), String> {
        for (mp, fs) in &mut self.mounts {
            if path.starts_with(mp.as_str()) {
                let rest = &path[mp.len()..];

                if rest.is_empty() || mp.ends_with('/') || rest.starts_with('/') {
                    let rel_path = if rest.is_empty() { "/" } else { rest };
                    return Ok((fs, rel_path));
                }
            }
        }
        Err(String::from("No mount point found"))
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.read_file(rel_path)
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.write_file(rel_path, data)
    }

    pub fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.list_dir(rel_path)
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.create_file(rel_path)
    }

    pub fn create_dir(&mut self, path: &str) -> Result<(), String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.create_dir(rel_path)
    }

    pub fn remove_file(&mut self, path: &str) -> Result<(), String> {
        let (fs, rel_path) = self.resolve_mount(path)?;
        fs.remove_file(rel_path)
    }
}
