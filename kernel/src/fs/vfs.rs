use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Device,
    Unknown,
}

pub trait FileSystem {
    fn root(&mut self) -> Result<Box<dyn VfsNode>, String>;
}

pub trait VfsNode {
    fn name(&self) -> String;
    fn size(&self) -> u64;
    fn kind(&self) -> FileType;
    
    // File operations
    fn read(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, String>;
    fn write(&mut self, offset: u64, buffer: &[u8]) -> Result<usize, String>;
    
    // Directory operations
    fn children(&mut self) -> Result<Vec<Box<dyn VfsNode>>, String>;
    fn find(&mut self, name: &str) -> Result<Box<dyn VfsNode>, String>;
}
