use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use super::structs::{Superblock, BlockGroupDescriptor, Inode, DirectoryEntryHeader, EXT2_SUPER_MAGIC, EXT2_FT_DIR, EXT2_FT_REG_FILE, EXT2_S_IFDIR};
use crate::fs::vfs::FileSystemDriver;
use super::ops::Ops;

pub const BASE_LBA: u64 = 16384;

pub struct FileSystem {
    pub disk_id: u8,
    pub sb: Superblock,
    pub bgdt: Vec<BlockGroupDescriptor>,
}

impl FileSystemDriver for FileSystem {
    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let mut ops = Ops::new(self);
        let (_, inode) = ops.resolve(path)?;
        if (inode.i_mode & EXT2_S_IFDIR) != 0 {
             return Err("Is a directory".into());
        }
        Ok(ops.fs.read_inode_data(&inode))
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), String> {
        let mut ops = Ops::new(self);
        ops.write_data(path, data)
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let mut ops = Ops::new(self);
        ops.list_dir(path)
    }

    fn create_file(&mut self, path: &str) -> Result<(), String> {
        // Default permissions 0x1A4 (644) and time 0
        let mut ops = Ops::new(self);
        ops.create_file(path, 0x1A4)
    }

    fn create_dir(&mut self, path: &str) -> Result<(), String> {
        let mut ops = Ops::new(self);
        ops.create_dir(path, 0x1FF) // 777
    }

    fn remove_file(&mut self, path: &str) -> Result<(), String> {
        let mut ops = Ops::new(self);
        ops.remove_file(path)
    }
}

impl FileSystem {
    pub fn mount(disk_id: u8) -> Self {
        std::println!("EXT2: Mounting disk {}...", disk_id);
        let mut sb_buf = [0u8; 1024];
        
        std::println!("EXT2: Reading Superblock at LBA {}", BASE_LBA + 2);
        crate::fs::disk::read(BASE_LBA + 2, disk_id, &mut sb_buf);
        
        let sb: Superblock = unsafe { core::ptr::read(sb_buf.as_ptr() as *const _) };
        std::println!("EXT2: Superblock read. Magic: {:#x}", sb.s_magic + 0);

        if sb.s_magic != EXT2_SUPER_MAGIC {
            panic!("Invalid Magic: {:#x}", sb.s_magic + 0);
        }

        let block_size = 1024 << sb.s_log_block_size;
        let groups = (sb.s_blocks_count + sb.s_blocks_per_group - 1) / sb.s_blocks_per_group;
        
        std::println!("EXT2: BlockSize: {}, Groups: {}", block_size, groups);

        let bgdt_start = if block_size == 1024 { 2 } else { 1 };
        let bgdt_lba = bgdt_start as u64 * (block_size as u64 / 512);
        let bgdt_bytes = groups as usize * 32;
        let bgdt_sectors = ((bgdt_bytes + 511) / 512) as u8;
        
        std::println!("EXT2: Reading BGDT at LBA {}, sectors: {}", BASE_LBA + bgdt_lba, bgdt_sectors);
        let mut bgdt_buf = vec![0u8; bgdt_sectors as usize * 512];
        crate::fs::disk::read(BASE_LBA + bgdt_lba, disk_id, &mut bgdt_buf);

        let mut bgdt = Vec::new();
        for i in 0..groups {
            let offset = i as usize * 32;
            let bg: BlockGroupDescriptor = unsafe { core::ptr::read(bgdt_buf[offset..].as_ptr() as *const _) };
            bgdt.push(bg);
        }
        
        std::println!("EXT2: Mount successful.");
        FileSystem { disk_id, sb, bgdt }
    }

    pub fn block_size(&self) -> u64 {
        1024 << self.sb.s_log_block_size
    }

    pub fn sectors_per_block(&self) -> u8 {
        (self.block_size() / 512) as u8
    }

    pub fn read_block(&mut self, block_id: u32) -> Vec<u8> {
        let bsize = self.block_size() as usize;
        let spb = self.sectors_per_block();
        let lba = BASE_LBA + (block_id as u64 * spb as u64);
        let mut buf = vec![0u8; bsize];
        crate::fs::disk::read(lba, self.disk_id, &mut buf);
        buf
    }

    pub fn write_block(&mut self, block_id: u32, data: &[u8]) {
        let bsize = self.block_size() as usize;
        let spb = self.sectors_per_block();
        let lba = BASE_LBA + (block_id as u64 * spb as u64);
        
        if data.len() != bsize {
            if data.len() > bsize { panic!("Data too large for block"); }
            let mut padded = vec![0u8; bsize];
            padded[..data.len()].copy_from_slice(data);
            crate::fs::disk::write(lba, self.disk_id, &padded);
        } else {
            crate::fs::disk::write(lba, self.disk_id, data);
        }
    }

    pub fn read_inode(&mut self, inode_num: u32) -> Inode {
        let idx = inode_num - 1;
        let group = (idx / self.sb.s_inodes_per_group) as usize;
        let local = (idx % self.sb.s_inodes_per_group) as usize;
        let bg = &self.bgdt[group];
        
        let table_lba = BASE_LBA + (bg.bg_inode_table as u64 * self.sectors_per_block() as u64);
        let byte_offset = local as u64 * 128;
        let sector_offset = byte_offset / 512;
        let offset_in_sector = (byte_offset % 512) as usize;

        let mut sector_buf = [0u8; 512];
        crate::fs::disk::read(table_lba + sector_offset, self.disk_id, &mut sector_buf);
        unsafe { core::ptr::read(sector_buf[offset_in_sector..].as_ptr() as *const Inode) }
    }

    pub fn write_inode(&mut self, inode_num: u32, inode: &Inode) {
        let idx = inode_num - 1;
        let group = (idx / self.sb.s_inodes_per_group) as usize;
        let local = (idx % self.sb.s_inodes_per_group) as usize;
        let bg = &self.bgdt[group];
        
        let table_lba = BASE_LBA + (bg.bg_inode_table as u64 * self.sectors_per_block() as u64);
        let byte_offset = local as u64 * 128;
        let sector_offset = byte_offset / 512;
        let offset_in_sector = (byte_offset % 512) as usize;

        let mut sector_buf = [0u8; 512];
        crate::fs::disk::read(table_lba + sector_offset, self.disk_id, &mut sector_buf);
        
        let bytes = unsafe { core::slice::from_raw_parts((inode as *const Inode) as *const u8, 128) };
        sector_buf[offset_in_sector..offset_in_sector+128].copy_from_slice(bytes);
        
        crate::fs::disk::write(table_lba + sector_offset, self.disk_id, &sector_buf);
    }

    pub fn read_inode_data(&mut self, inode: &Inode) -> Vec<u8> {
        let size = inode.i_size as usize;
        // std::println!("ReadInodeData: Size={}", size);
        let mut content = Vec::with_capacity(size);
        let mut remaining = size;
        
        // Iterate 0..12 direct
        for i in 0..12 {
            if remaining == 0 { break; }
            self.process_block_read(inode.i_block[i], 0, &mut content, &mut remaining);
        }
        // Indirects
        if remaining > 0 { self.process_block_read(inode.i_block[12], 1, &mut content, &mut remaining); }
        if remaining > 0 { self.process_block_read(inode.i_block[13], 2, &mut content, &mut remaining); }
        if remaining > 0 { self.process_block_read(inode.i_block[14], 3, &mut content, &mut remaining); }
        
        // Ensure content is exactly `size` bytes (truncate if necessary)
        if content.len() > size {
            content.truncate(size);
        }
        
        content
    }

    fn process_block_read(&mut self, bid: u32, lvl: u8, content: &mut Vec<u8>, rem: &mut usize) {
        if *rem == 0 { return; }
        let bsize = self.block_size() as usize;
        
        if bid == 0 {
            // Sparse
            let ptrs = bsize / 4;
            let mut cov = bsize;
            for _ in 0..lvl { cov *= ptrs; }
            let fill = core::cmp::min(cov, *rem);
            content.extend(core::iter::repeat(0).take(fill));
            *rem -= fill;
            return;
        }

        if lvl == 0 {
            // Data Block
            let read_len = core::cmp::min(bsize, *rem);
            let buf = self.read_block(bid);
            content.extend_from_slice(&buf[..read_len]);
            *rem -= read_len;
        } else {
            // Pointer Block
            let buf = self.read_block(bid);
            let ptrs = bsize / 4;
            for i in 0..ptrs {
                if *rem == 0 { break; }
                let next = u32::from_le_bytes(buf[i*4..i*4+4].try_into().unwrap());
                self.process_block_read(next, lvl - 1, content, rem);
            }
        }
    }

    // --- ALLOCATION ---

    fn find_free(&self, buf: &[u8]) -> Option<usize> {
        for (i, &b) in buf.iter().enumerate() {
            if b != 0xFF {
                for bit in 0..8 {
                    if (b & (1 << bit)) == 0 { return Some(i * 8 + bit); }
                }
            }
        }
        None
    }

    pub fn alloc_block(&mut self) -> Option<u32> {
        let bsize = self.block_size() as usize;
        for g in 0..self.bgdt.len() {
            if self.bgdt[g].bg_free_blocks_count > 0 {
                let bmp_block = self.bgdt[g].bg_block_bitmap;
                let mut buf = self.read_block(bmp_block);
                
                // DEBUG
                // std::println!("AllocBlock: Group {} BitmapBlock={}", g, bmp_block);

                if let Some(local) = self.find_free(&buf) {
                    buf[local/8] |= 1 << (local%8);
                    self.write_block(bmp_block, &buf);
                    
                    self.sb.s_free_blocks_count -= 1;
                    self.bgdt[g].bg_free_blocks_count -= 1;
                    self.sync();
                    
                    let block_id = (g as u32 * self.sb.s_blocks_per_group) + local as u32 + self.sb.s_first_data_block;
                    std::println!("Allocated Block: {}", block_id);
                    return Some(block_id);
                }
            }
        }
        None
    }

    pub fn alloc_inode(&mut self) -> Option<u32> {
        let bsize = self.block_size() as usize;
        for g in 0..self.bgdt.len() {
            if self.bgdt[g].bg_free_inodes_count > 0 {
                let bmp_block = self.bgdt[g].bg_inode_bitmap;
                let mut buf = self.read_block(bmp_block);
                
                // DEBUG: Print bitmap state
                // std::println!("AllocInode: Group {} Bitmap[0]={:#04x} Free={}", g, buf[0], self.bgdt[g].bg_free_inodes_count);

                if let Some(local) = self.find_free(&buf) {
                    buf[local/8] |= 1 << (local%8);
                    self.write_block(bmp_block, &buf);
                    
                    self.sb.s_free_inodes_count -= 1;
                    self.bgdt[g].bg_free_inodes_count -= 1;
                    self.sync();
                    
                    let inode_num = (g as u32 * self.sb.s_inodes_per_group) + local as u32 + 1;
                    std::println!("Allocated Inode: {}", inode_num);
                    return Some(inode_num);
                }
            }
        }
        None
    }

    pub fn free_block(&mut self, block_id: u32) {
        let group = ((block_id - self.sb.s_first_data_block) / self.sb.s_blocks_per_group) as usize;
        let local = ((block_id - self.sb.s_first_data_block) % self.sb.s_blocks_per_group) as usize;
        let bmp_block = self.bgdt[group].bg_block_bitmap;
        
        let mut buf = self.read_block(bmp_block);
        if (buf[local/8] & (1 << (local%8))) != 0 {
            buf[local/8] &= !(1 << (local%8));
            self.write_block(bmp_block, &buf);
            self.sb.s_free_blocks_count += 1;
            self.bgdt[group].bg_free_blocks_count += 1;
            self.sync();
        }
    }

    pub fn free_inode(&mut self, inode_num: u32) {
        let idx = inode_num - 1;
        let group = (idx / self.sb.s_inodes_per_group) as usize;
        let local = (idx % self.sb.s_inodes_per_group) as usize;
        let bmp_block = self.bgdt[group].bg_inode_bitmap;
        
        let mut buf = self.read_block(bmp_block);
        if (buf[local/8] & (1 << (local%8))) != 0 {
            buf[local/8] &= !(1 << (local%8));
            self.write_block(bmp_block, &buf);
            self.sb.s_free_inodes_count += 1;
            self.bgdt[group].bg_free_inodes_count += 1;
            self.sync();
        }
    }

    pub fn sync(&mut self) {
        let sb_bytes = unsafe { core::slice::from_raw_parts((&self.sb as *const _) as *const u8, 1024) };
        let mut buf = [0u8; 1024];
        buf[..core::mem::size_of::<Superblock>()].copy_from_slice(&sb_bytes[..core::mem::size_of::<Superblock>()]);
        crate::fs::disk::write(BASE_LBA + 2, self.disk_id, &buf);

        let bsize = self.block_size();
        let bgdt_start = if bsize == 1024 { 2 } else { 1 };
        let bgdt_lba = bgdt_start as u64 * (bsize as u64 / 512);
        let total_bytes = self.bgdt.len() * 32;
        let total_sectors = ((total_bytes + 511) / 512) as u8;
        let mut bgdt_buf = vec![0u8; total_sectors as usize * 512];
        
        for (i, bg) in self.bgdt.iter().enumerate() {
            let raw = unsafe { core::mem::transmute::<BlockGroupDescriptor, [u8;32]>(*bg) };
            bgdt_buf[i*32..(i+1)*32].copy_from_slice(&raw);
        }
        crate::fs::disk::write(BASE_LBA + bgdt_lba, self.disk_id, &bgdt_buf);
    }

    pub fn add_directory_entry(&mut self, parent_inode_num: u32, inode_num: u32, name: &str, ftype: u8) -> Result<(), String> {
        let mut parent = self.read_inode(parent_inode_num);
        let bsize = self.block_size() as usize;

        for i in 0..12 {
            let mut bid = parent.i_block[i];
            if bid == 0 {
                // New block needed
                bid = self.alloc_block().ok_or("No blocks")?;
                parent.i_block[i] = bid;
                parent.i_blocks += (bsize as u32) / 512;
                parent.i_size += bsize as u32; // Directory size grows
                self.write_inode(parent_inode_num, &parent);
                
                // Init new block with one big entry
                let mut buf = vec![0u8; bsize];
                let h = DirectoryEntryHeader { inode: inode_num, rec_len: bsize as u16, name_len: name.len() as u8, file_type: ftype };
                unsafe { *(buf.as_mut_ptr() as *mut DirectoryEntryHeader) = h; }
                buf[8..8+name.len()].copy_from_slice(name.as_bytes());
                self.write_block(bid, &buf);
                return Ok(());
            }

            let mut buf = self.read_block(bid);
            if self.try_insert_entry(&mut buf, inode_num, name, ftype) {
                self.write_block(bid, &buf);
                return Ok(());
            }
        }
        Err("Directory full".into())
    }

    fn try_insert_entry(&self, buf: &mut [u8], inode: u32, name: &str, ftype: u8) -> bool {
        let mut off = 0;
        let bsize = buf.len();
        while off < bsize {
            if off + 8 > bsize { return false; }
            let h_ptr = buf[off..].as_ptr() as *mut DirectoryEntryHeader;
            let h = unsafe { &mut *h_ptr };
            
            let real = 8 + h.name_len as usize;
            let align = (real + 3) & !3;
            let rec = h.rec_len as usize;

            // Use deleted entry?
            if h.inode == 0 && rec >= (8 + name.len()) {
                h.inode = inode; h.name_len = name.len() as u8; h.file_type = ftype;
                buf[off+8..off+8+name.len()].copy_from_slice(name.as_bytes());
                return true;
            }

            // Shrink existing?
            let need = 8 + name.len();
            let align_need = (need + 3) & !3;
            let free = rec.saturating_sub(align);
            
            if free >= align_need {
                h.rec_len = align as u16;
                let new_off = off + align;
                let new_h_ptr = buf[new_off..].as_ptr() as *mut DirectoryEntryHeader;
                let new_h = unsafe { &mut *new_h_ptr };
                new_h.inode = inode; new_h.rec_len = free as u16; new_h.name_len = name.len() as u8; new_h.file_type = ftype;
                buf[new_off+8..new_off+8+name.len()].copy_from_slice(name.as_bytes());
                return true;
            }
            off += rec;
            if rec == 0 { return false; }
        }
        false
    }

    pub fn remove_directory_entry(&mut self, parent_id: u32, name: &str) -> Result<(), String> {
        let parent = self.read_inode(parent_id);
        let bsize = self.block_size() as usize;
        
        for i in 0..12 {
            let bid = parent.i_block[i];
            if bid == 0 { continue; }
            let mut buf = self.read_block(bid);
            
            let mut off = 0;
            let mut prev_off: Option<usize> = None;
            let mut found = false;
            
            while off < bsize {
                if off + 8 > bsize { break; }
                let h_ptr = buf[off..].as_ptr() as *mut DirectoryEntryHeader;
                let h = unsafe { &mut *h_ptr };
                let rec = h.rec_len as usize;
                
                if h.inode != 0 {
                    let n = &buf[off+8..off+8+h.name_len as usize];
                    if n == name.as_bytes() {
                        found = true;
                        if let Some(p) = prev_off {
                            let p_ptr = buf[p..].as_ptr() as *mut DirectoryEntryHeader;
                            unsafe { (&mut *p_ptr).rec_len += h.rec_len; }
                        } else {
                            h.inode = 0; // Just mark unused if first
                        }
                        break;
                    }
                }
                prev_off = Some(off);
                off += rec;
                if rec == 0 { break; }
            }
            
            if found {
                self.write_block(bid, &buf);
                return Ok(());
            }
        }
        Err("Not found".into())
    }
    
    pub fn get_or_alloc_block(&mut self, inode: &mut Inode, logical: u32) -> u32 {
        let bsize = self.block_size() as u32;
        let ptrs = bsize / 4;
        
        // 1. Direct Blocks (0-11)
        if logical < 12 {
            let idx = logical as usize;
            let mut bid = inode.i_block[idx];
            if bid == 0 {
                bid = self.alloc_block().expect("Full");
                inode.i_block[idx] = bid;
                inode.i_blocks += bsize / 512;
            }
            return bid;
        }
        
        let mut rel = logical - 12;

        // 2. Single Indirect (12)
        if rel < ptrs {
            let mut ptr = inode.i_block[12];
            let (res, added) = self.recurse_alloc(&mut ptr, rel, 1);
            if inode.i_block[12] != ptr { inode.i_block[12] = ptr; }
            inode.i_blocks += added;
            return res;
        }
        rel -= ptrs;

        // 3. Double Indirect (13)
        let ptrs_sq = ptrs * ptrs;
        if rel < ptrs_sq {
            let mut ptr = inode.i_block[13];
            let (res, added) = self.recurse_alloc(&mut ptr, rel, 2);
            if inode.i_block[13] != ptr { inode.i_block[13] = ptr; }
            inode.i_blocks += added;
            return res;
        }
        rel -= ptrs_sq;

        // 4. Triple Indirect (14)
        // Capacity: ptrs^3. For 1KB blocks (256 ptrs), this is ~16GB.
        let ptrs_cu = ptrs_sq * ptrs;
        if rel < ptrs_cu {
            let mut ptr = inode.i_block[14];
            let (res, added) = self.recurse_alloc(&mut ptr, rel, 3);
            if inode.i_block[14] != ptr { inode.i_block[14] = ptr; }
            inode.i_blocks += added;
            return res;
        }
        
        panic!("File too big");
    }

    fn recurse_alloc(&mut self, ptr: &mut u32, off: u32, level: u8) -> (u32, u32) {
        let bsize = self.block_size() as usize;
        let mut added = 0;
        
        if *ptr == 0 {
            *ptr = self.alloc_block().expect("Full");
            added += (bsize as u32) / 512;
            // Zero out index blocks to prevent garbage pointers
            if level > 0 { self.write_block(*ptr, &vec![0u8; bsize]); }
        }
        
        if level == 0 { return (*ptr, added); }

        let mut buf = self.read_block(*ptr);
        let ptrs = (bsize / 4) as u32;
        
        // Calculate index and sub-offset for this level
        // Level 1 (Indirect): Divisor = 1
        // Level 2 (Dbl): Divisor = ptrs
        // Level 3 (Tpl): Divisor = ptrs^2
        let divisor = ptrs.pow((level - 1) as u32);
        
        let idx = (off / divisor) as usize;
        let sub_off = off % divisor;
        
        let entry_off = idx * 4; 
        let mut child = u32::from_le_bytes(buf[entry_off..entry_off+4].try_into().unwrap());
        
        let (res, child_added) = self.recurse_alloc(&mut child, sub_off, level - 1);
        added += child_added;

        let current_val = u32::from_le_bytes(buf[entry_off..entry_off+4].try_into().unwrap());
        if current_val != child {
            buf[entry_off..entry_off+4].copy_from_slice(&child.to_le_bytes());
            self.write_block(*ptr, &buf);
        }
        (res, added)
    }
}
