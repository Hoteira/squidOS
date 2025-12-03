use crate::fs::ext2::fs::FileSystem;
use crate::fs::ext2::structs::{Inode, DirectoryEntryHeader, EXT2_S_IFDIR, EXT2_S_IFREG, EXT2_FT_DIR, EXT2_FT_REG_FILE}; // Wait, EXT2_S_IFReg typo? No, use existing consts.
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::{format, vec};

pub struct Ops<'a> {
    pub fs: &'a mut FileSystem,
}

impl<'a> Ops<'a> {
    pub fn new(fs: &'a mut FileSystem) -> Self {
        Ops { fs }
    }

    // Helper to strip "@0xXX" prefix
    fn strip_disk_prefix<'b>(&self, path: &'b str) -> &'b str {
        if path.starts_with("@0x") {
            if let Some(slash_idx) = path.find('/') {
                return &path[slash_idx..];
            } else {
                return "/";
            }
        }
        // Fallback for legacy "@disk" or no prefix
        let clean = path.trim_start_matches("@disk");
        clean
    }

    fn parse_path<'b>(&self, path: &'b str) -> Vec<&'b str> {
        let clean = self.strip_disk_prefix(path);
        let clean = clean.trim_start_matches('/');
        if clean.is_empty() { return Vec::new(); }
        clean.split('/').collect()
    }

    pub fn resolve(&mut self, path: &str) -> Result<(u32, Inode), String> {
        let parts = self.parse_path(path);
        let mut curr_num = 2; 
        let mut curr_inode = self.fs.read_inode(2);

        for part in parts {
            if part == "." { continue; }
            if part == ".." { panic!("Parent traversal not impl"); }
            
            let data = self.fs.read_inode_data(&curr_inode);
            let mut found = None;
            let mut off = 0;
            while off < data.len() {
                if off + 8 > data.len() { break; }
                let h = unsafe { *(data[off..].as_ptr() as *const DirectoryEntryHeader) };
                if h.inode != 0 {
                    let name = &data[off+8..off+8+h.name_len as usize];
                    if name == part.as_bytes() {
                        found = Some(h.inode);
                        break;
                    }
                }
                off += h.rec_len as usize;
            }

            if let Some(next) = found {
                curr_num = next;
                curr_inode = self.fs.read_inode(next);
            } else {
                return Err(format!("Path not found: {}", part));
            }
        }
        Ok((curr_num, curr_inode))
    }

    pub fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let (_, inode) = self.resolve(path)?;
        if (inode.i_mode & EXT2_S_IFDIR) == 0 { return Err("Not a dir".into()); }

        let data = self.fs.read_inode_data(&inode);
        let mut res = Vec::new();
        let mut off = 0;
        while off < data.len() {
            let h = unsafe { *(data[off..].as_ptr() as *const DirectoryEntryHeader) };
            if h.inode != 0 {
                let name = String::from_utf8_lossy(&data[off+8..off+8+h.name_len as usize]).to_string();
                res.push(name);
            }
            off += h.rec_len as usize;
            if h.rec_len == 0 { break; }
        }
        Ok(res)
    }

    pub fn create_file(&mut self, path: &str, perms: u16) -> Result<(), String> {
        let (dir, name) = self.split_parent(path)?;
        let (p_num, _) = self.resolve(dir)?;
        
        let inode_num = self.fs.alloc_inode().ok_or("No inodes")?;
        let time = 0;
        
        let inode = Inode {
            i_mode: EXT2_S_IFREG | perms,
            i_uid: 0, i_gid: 0, i_links_count: 1,
            i_size: 0, i_atime: time, i_ctime: time, i_mtime: time, i_dtime: 0,
            i_blocks: 0, i_flags: 0, i_osd1: 0, i_block: [0;15], i_generation: 0,
            i_file_acl: 0, i_dir_acl: 0, i_faddr: 0, i_osd2: [0;12]
        };
        self.fs.write_inode(inode_num, &inode);
        self.fs.add_directory_entry(p_num, inode_num, name, EXT2_FT_REG_FILE)?;
        Ok(())
    }

    pub fn create_dir(&mut self, path: &str, perms: u16) -> Result<(), String> {
        let (dir, name) = self.split_parent(path)?;
        let (p_num, _) = self.resolve(dir)?;
        
        let inode_num = self.fs.alloc_inode().ok_or("No inodes")?;
        let block_id = self.fs.alloc_block().ok_or("No blocks")?;
        let bsize = self.fs.block_size();
        let time = 0;

        // Init . and ..
        let mut buf = vec![0u8; bsize as usize];
        let dot = DirectoryEntryHeader { inode: inode_num, rec_len: 12, name_len: 1, file_type: EXT2_FT_DIR };
        unsafe { *(buf.as_mut_ptr() as *mut DirectoryEntryHeader) = dot; }
        buf[8] = b'.';
        
        let dotdot = DirectoryEntryHeader { inode: p_num, rec_len: (bsize - 12) as u16, name_len: 2, file_type: EXT2_FT_DIR };
        unsafe { *(buf[12..].as_mut_ptr() as *mut DirectoryEntryHeader) = dotdot; }
        buf[12+8] = b'.'; buf[12+9] = b'.';
        
        self.fs.write_block(block_id, &buf);

        let inode = Inode {
            i_mode: EXT2_S_IFDIR | perms,
            i_uid: 0, i_gid: 0, i_links_count: 2, // . and parent link
            i_size: bsize as u32, i_atime: time, i_ctime: time, i_mtime: time, i_dtime: 0,
            i_blocks: (bsize as u32)/512, i_flags: 0, i_osd1: 0, i_block: [0;15], i_generation: 0,
            i_file_acl: 0, i_dir_acl: 0, i_faddr: 0, i_osd2: [0;12]
        };
        let mut fin = inode; fin.i_block[0] = block_id;
        self.fs.write_inode(inode_num, &fin);
        
        self.fs.add_directory_entry(p_num, inode_num, name, EXT2_FT_DIR)?;
        Ok(())
    }

    pub fn write_data(&mut self, path: &str, data: &[u8]) -> Result<(), String> {
        let (num, mut inode) = self.resolve(path)?;
        if (inode.i_mode & EXT2_S_IFDIR) != 0 { return Err("Is a directory".into()); }
        
        let bsize = self.fs.block_size() as usize;
        let blocks = (data.len() + bsize - 1) / bsize;
        
        for i in 0..blocks {
            let pid = self.fs.get_or_alloc_block(&mut inode, i as u32);
            let start = i * bsize;
            let end = core::cmp::min(start + bsize, data.len());
            let chunk = &data[start..end];
            self.fs.write_block(pid, chunk);
        }
        inode.i_size = data.len() as u32;
        inode.i_mtime = 0; // Update modification time
        self.fs.write_inode(num, &inode);
        Ok(())
    }

    pub fn remove_file(&mut self, path: &str) -> Result<(), String> {
        let (dir, name) = self.split_parent(path)?;
        let (p_num, _) = self.resolve(dir)?;
        
        let (target_num, target_inode) = self.resolve(path)?;
        if (target_inode.i_mode & EXT2_S_IFDIR) != 0 { return Err("Is directory".into()); }

        self.fs.remove_directory_entry(p_num, name)?;
        self.fs.free_inode(target_num);
        Ok(())
    }

    fn split_parent<'b>(&self, path: &'b str) -> Result<(&'b str, &'b str), String> {
        let clean = self.strip_disk_prefix(path);
        match clean.rfind('/') {
            Some(i) => Ok((if i==0 { "/" } else { &clean[..i] }, &clean[i+1..])),
            None => Ok(("/", clean)), 
        }
    }
}