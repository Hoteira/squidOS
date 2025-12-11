use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

// Using const { None } for array initialization of non-Copy types (Option<Box<...>>)
// This assumes a recent Rust compiler supporting inline const expressions or relaxed array initialization.
pub static mut FILESYSTEMS: [Option<Box<dyn FileSystem>>; 256] = [const { None }; 256];
pub static mut OPEN_FILES: [Option<FileHandle>; 256] = [const { None }; 256];

pub struct FileHandle {
    pub node: Box<dyn VfsNode>,
    pub offset: u64,
}

pub fn init() {
    // No explicit initialization needed for static arrays initialized with None
}

pub fn mount(disk_id: u8, fs: Box<dyn FileSystem>) {
    unsafe {
        FILESYSTEMS[disk_id as usize] = Some(fs);
    }
}

pub fn open_file(path_obj: &Path) -> Result<usize, String> {
    let node = open(path_obj)?;
    unsafe {
        for i in 3..256 {
            if OPEN_FILES[i].is_none() {
                OPEN_FILES[i] = Some(FileHandle { node, offset: 0 });
                return Ok(i);
            }
        }
        Err(String::from("No free file descriptors"))
    }
}

pub fn get_file(fd: usize) -> Option<&'static mut FileHandle> {
    unsafe {
        if fd < 256 {
            OPEN_FILES[fd].as_mut()
        } else {
            None
        }
    }
}

pub fn close_file(fd: usize) {
    unsafe {
         if fd > 2 && fd < 256 {
             OPEN_FILES[fd] = None;
         }
    }
}

pub fn open(path_obj: &Path) -> Result<Box<dyn VfsNode>, String> {
    unsafe {
        if let Some(fs) = &mut FILESYSTEMS[path_obj.disk_id as usize] {
             let mut node = fs.root()?;
             for component in &path_obj.components {
                 node = node.find(&component)?;
             }
             Ok(node)
        } else {
            Err(String::from("Disk ID not mounted"))
        }
    }
}

pub fn read(path_obj: Path, offset: u64, size: u64, buffer: *mut u8) -> Result<usize, String> {
    unsafe {
        if let Some(fs) = &mut FILESYSTEMS[path_obj.disk_id as usize] {
             let mut node = fs.root()?;
             for component in path_obj.components {
                 node = node.find(&component)?;
             }
             let slice = core::slice::from_raw_parts_mut(buffer, size as usize);
             node.read(offset, slice)
        } else {
            Err(String::from("Disk ID not mounted"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub disk_id: u8,
    pub components: Vec<String>,
}

impl Path {
    pub fn new(path_str: &str) -> Result<Self, String> {
        if !path_str.starts_with('@') {
            return Err(String::from("Path must start with '@' (e.g., @0/path/to/file)"));
        }

        let disk_end = path_str.find('/').ok_or(String::from("Invalid path format: missing '/' after disk ID"))?;
        
        let disk_part = &path_str[1..disk_end];
        let path_part = &path_str[disk_end+1..];

        let disk_id = if disk_part.starts_with("0x") || disk_part.starts_with("0X") {
            u8::from_str_radix(&disk_part[2..], 16).map_err(|_| String::from("Invalid hex disk ID"))?
        } else {
            disk_part.parse::<u8>().map_err(|_| String::from("Invalid decimal disk ID"))?
        };

        let components: Vec<String> = path_part
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(Path {
            disk_id,
            components,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Device,
    Unknown,
}

pub trait FileSystem: Send + Sync {
    fn root(&mut self) -> Result<Box<dyn VfsNode>, String>;
}

pub trait VfsNode {
    fn name(&self) -> String;
    fn size(&self) -> u64;
    fn kind(&self) -> FileType;
    
    fn read(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, String>;
    fn write(&mut self, offset: u64, buffer: &[u8]) -> Result<usize, String>;
    
    fn children(&mut self) -> Result<Vec<Box<dyn VfsNode>>, String>;
    fn find(&mut self, name: &str) -> Result<Box<dyn VfsNode>, String>;
}