pub const EXT2_SUPER_MAGIC: u16 = 0xEF53;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub s_inodes_count: u32,      // Total number of inodes in the filesystem
    pub s_blocks_count: u32,      // Total number of blocks in the filesystem
    pub s_r_blocks_count: u32,    // Number of reserved blocks (for root user)
    pub s_free_blocks_count: u32, // Number of free blocks
    pub s_free_inodes_count: u32, // Number of free inodes
    pub s_first_data_block: u32,  // First data block (usually 0 for 1KB block size, or 1 otherwise)
    pub s_log_block_size: u32,    // Logarithmic block size (block_size = 1024 << s_log_block_size)
    pub s_log_frag_size: u32,     // Logarithmic fragment size
    pub s_blocks_per_group: u32,  // Number of blocks per block group
    pub s_frags_per_group: u32,   // Number of fragments per block group
    pub s_inodes_per_group: u32,  // Number of inodes per block group
    pub s_mtime: u32,             // Last mount time
    pub s_wtime: u32,             // Last write time
    pub s_mnt_count: u16,         // Mount count since last fsck
    pub s_max_mnt_count: u16,     // Max mount count before fsck
    pub s_magic: u16,             // Magic signature (0xEF53)
    pub s_state: u16,             // Filesystem state (e.g., clean, errors)
    pub s_errors: u16,            // What to do when errors are detected
    pub s_minor_rev_level: u16,   // Minor revision level
    pub s_lastcheck: u32,         // Last check time
    pub s_checkinterval: u32,     // Max time between checks
    pub s_creator_os: u32,        // Creator OS
    pub s_rev_level: u32,         // Revision level
    pub s_def_resuid: u16,        // Default UID for reserved blocks
    pub s_def_resgid: u16,        // Default GID for reserved blocks
}

impl Superblock {
    pub fn block_size(&self) -> u64 {
        1024 << self.s_log_block_size
    }

}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BlockGroupDescriptor {
    pub bg_block_bitmap: u32,      // Blocks bitmap block
    pub bg_inode_bitmap: u32,      // Inodes bitmap block
    pub bg_inode_table: u32,       // Inodes table block
    pub bg_free_blocks_count: u16, // Free blocks count
    pub bg_free_inodes_count: u16, // Free inodes count
    pub bg_used_dirs_count: u16,   // Directories count
    pub bg_pad: u16,               // Padding
    pub bg_reserved: [u32; 3],     // Reserved
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Inode {
    pub i_mode: u16,        // File mode (permissions and file type)
    pub i_uid: u16,         // User ID
    pub i_size: u32,        // Size in bytes
    pub i_atime: u32,       // Access time
    pub i_ctime: u32,       // Creation time
    pub i_mtime: u32,       // Modification time
    pub i_dtime: u32,       // Deletion time
    pub i_gid: u16,         // Group ID
    pub i_links_count: u16, // Hard links count
    pub i_blocks: u32,      // Count of 512-byte blocks allocated
    pub i_flags: u32,       // File flags
    pub i_osd1: u32,        // OS dependent value (Linux: i_version for 2.4+ kernel)
    pub i_block: [u32; 15], // Pointers to data blocks
    pub i_generation: u32,  // File version (for NFS)
    pub i_file_acl: u32,    // File ACL (Advanced user access control list)
    pub i_dir_acl: u32,     // Directory ACL
    pub i_faddr: u32,       // Fragment address
    pub i_osd2: [u8; 12],   // OS dependent values
}

// Inode mode flags (common ones)
pub const EXT2_S_IFSOCK: u16 = 0xC000; // socket
pub const EXT2_S_IFLNK: u16 = 0xA000;  // symbolic link
pub const EXT2_S_IFREG: u16 = 0x8000;  // regular file
pub const EXT2_S_IFBLK: u16 = 0x6000;  // block device
pub const EXT2_S_IFDIR: u16 = 0x4000;  // directoryz
pub const EXT2_S_IFCHR: u16 = 0x2000;  // character device
pub const EXT2_S_IFIFO: u16 = 0x1000;  // fifo

// Permission bits (example)
pub const EXT2_S_IRUSR: u16 = 0x0100;  // user read
pub const EXT2_S_IWUSR: u16 = 0x0080;  // user write
pub const EXT2_S_IXUSR: u16 = 0x0040;  // user execute
pub const EXT2_S_IRGRP: u16 = 0x0020;  // group read
pub const EXT2_S_IWGRP: u16 = 0x0010;  // group write
pub const EXT2_S_IXGRP: u16 = 0x0008;  // group execute
pub const EXT2_S_IROTH: u16 = 0x0004;  // others read
pub const EXT2_S_IWOTH: u16 = 0x0002;  // others write
pub const EXT2_S_IXOTH: u16 = 0x0001;  // others execute

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DirectoryEntryHeader {
    pub inode: u32,      // Inode number
    pub rec_len: u16,    // Record length
    pub name_len: u8,    // Name length
    pub file_type: u8,   // File type
}

// File types for Directory Entries
pub const EXT2_FT_UNKNOWN: u8 = 0;
pub const EXT2_FT_REG_FILE: u8 = 1;
pub const EXT2_FT_DIR: u8 = 2;
pub const EXT2_FT_CHRDEV: u8 = 3;
pub const EXT2_FT_BLKDEV: u8 = 4;
pub const EXT2_FT_FIFO: u8 = 5;
pub const EXT2_FT_SOCK: u8 = 6;
pub const EXT2_FT_SYMLINK: u8 = 7;