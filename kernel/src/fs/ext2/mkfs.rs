use alloc::vec;
use alloc::string::String;
use super::structs::{Superblock, BlockGroupDescriptor, Inode, DirectoryEntryHeader, EXT2_SUPER_MAGIC, EXT2_S_IFDIR, EXT2_S_IRUSR, EXT2_S_IWUSR, EXT2_S_IXUSR, EXT2_FT_DIR};
use crate::fs::ext2::fs::BASE_LBA;
use crate::fs::disk;

pub fn format_disk(disk_id: u8, size: u64) -> Result<(), String> {
    // We assume the disk driver is already initialized.
    // 'size' is the size of the partition we are formatting.

    // Parameters
    let block_size: u32 = 1024;
    let blocks_count = (size / block_size as u64) as u32;
    let inodes_count = blocks_count / 4;
    let blocks_per_group = 8192;
    let inodes_per_group = inodes_count; // Simplified: 1 group
    
    // 1. Init Superblock
    let sb = Superblock {
        s_inodes_count: inodes_count,
        s_blocks_count: blocks_count,
        s_r_blocks_count: 0,
        s_free_blocks_count: 0,
        s_free_inodes_count: 0,
        s_first_data_block: if block_size == 1024 { 1 } else { 0 },
        s_log_block_size: 0,
        s_log_frag_size: 0,
        s_blocks_per_group: blocks_per_group,
        s_frags_per_group: blocks_per_group,
        s_inodes_per_group: inodes_per_group,
        s_mtime: 0, s_wtime: 0,
        s_mnt_count: 0, s_max_mnt_count: 0xFFFF,
        s_magic: EXT2_SUPER_MAGIC,
        s_state: 1, s_errors: 1, s_minor_rev_level: 0,
        s_lastcheck: 0, s_checkinterval: 0, s_creator_os: 0,
        s_rev_level: 0, s_def_resuid: 0, s_def_resgid: 0,
    };

    // Layout Group 0
    let sb_block = sb.s_first_data_block;
    let bgdt_block = sb_block + 1;
    let b_bitmap_block = bgdt_block + 1;
    let i_bitmap_block = b_bitmap_block + 1;
    let i_table_block = i_bitmap_block + 1;
    
    let inode_size = 128;
    let i_table_blocks = (inodes_per_group * inode_size + block_size - 1) / block_size;
    let data_start_block = i_table_block + i_table_blocks;

    // 2. Init Group Descriptor
    let mut bg = BlockGroupDescriptor {
        bg_block_bitmap: b_bitmap_block,
        bg_inode_bitmap: i_bitmap_block,
        bg_inode_table: i_table_block,
        bg_free_blocks_count: 0,
        bg_free_inodes_count: 0,
        bg_used_dirs_count: 0,
        bg_pad: 0, bg_reserved: [0; 3],
    };

    // 3. Prepare Bitmaps
    let mut b_bitmap = vec![0u8; block_size as usize];
    let mut i_bitmap = vec![0u8; block_size as usize];

    // Reserve Metadata Blocks
    let reserved_blocks = data_start_block - sb.s_first_data_block;
    for i in 0..reserved_blocks {
        let byte = (i / 8) as usize;
        let bit = (i % 8) as usize;
        b_bitmap[byte] |= 1 << bit;
    }
    
    // Reserve Inodes 1..10 (Inodes are 1-based, bitmap 0-based)
    for i in 0..10 { 
        let byte = (i / 8) as usize;
        let bit = (i % 8) as usize;
        i_bitmap[byte] |= 1 << bit;
    }

    // Root Dir Data Block
    let root_data_block = data_start_block;
    let root_block_rel = root_data_block - sb.s_first_data_block;
    b_bitmap[(root_block_rel/8) as usize] |= 1 << (root_block_rel%8);

    // 4. Calc Free Counts
    let total_used_blocks = reserved_blocks + 1; 
    let free_blocks = blocks_count - total_used_blocks - sb.s_first_data_block; 
    let free_inodes = inodes_count - 10;

    let mut final_sb = sb;
    final_sb.s_free_blocks_count = free_blocks;
    final_sb.s_free_inodes_count = free_inodes;
    
    bg.bg_free_blocks_count = free_blocks as u16;
    bg.bg_free_inodes_count = free_inodes as u16;
    bg.bg_used_dirs_count = 1;

    // 5. Write Structures to Disk
    // Helper to write a block
    let write_block = |block: u32, data: &[u8]| {
        let lba = BASE_LBA + (block as u64 * (block_size as u64 / 512));
        let mut padded = vec![0u8; block_size as usize];
        padded[..data.len()].copy_from_slice(data);
        disk::write(lba, disk_id, &padded);
    };
    
    // Superblock (always at 1024 bytes offset, which is block 1 if block_size=1024, or start of block 0 + 1024)
    // Since s_first_data_block=1 for 1k blocks, sb is at block 1 * 1024 = 1024?
    // Wait, standard: SB is at byte 1024.
    // If block size is 1024, block 0 is [0, 1023], block 1 is [1024, 2047].
    // So SB is in block 1.
    // However, my code earlier did `disk.read_pio(2, ...)` which is 1024 bytes (LBA 2 = 512*2).
    // `write_block` maps block X to LBA X * (1024/512) = 2X.
    // So block 1 -> LBA 2. Correct.
    
    // We can't use write_block simply for SB because SB is 1024 bytes but might not take up whole block if block>1024.
    // But here block_size=1024. So SB takes exactly block 1.
    
    // Write Superblock
    let mut sb_buf = vec![0u8; block_size as usize];
    let sb_bytes = any_as_bytes(&final_sb);
    sb_buf[..sb_bytes.len()].copy_from_slice(sb_bytes);
    write_block(1, &sb_buf);

    // Write BGDT
    let mut bgdt_buf = vec![0u8; block_size as usize];
    let bg_bytes = any_as_bytes(&bg);
    bgdt_buf[..bg_bytes.len()].copy_from_slice(bg_bytes);
    write_block(bgdt_block, &bgdt_buf);

    // Write Bitmaps
    write_block(b_bitmap_block, &b_bitmap);
    write_block(i_bitmap_block, &i_bitmap);

    // 6. Root Inode (Inode 2 -> Index 1)
    // Inode table is at `i_table_block`. Inode size 128.
    // Index 1 is at offset 128 bytes into the table.
    // We read the block, modify, write back. But we are formatting, so we can zero + write.
    let mut itable_buf = vec![0u8; block_size as usize];
    let mut root = Inode {
        i_mode: EXT2_S_IFDIR | EXT2_S_IRUSR | EXT2_S_IWUSR | EXT2_S_IXUSR,
        i_uid: 0, i_size: block_size, i_atime: 0, i_ctime: 0, i_mtime: 0, i_dtime: 0, i_gid: 0,
        i_links_count: 2, i_blocks: (block_size / 512), i_flags: 0, i_osd1: 0,
        i_block: [0; 15], i_generation: 0, i_file_acl: 0, i_dir_acl: 0, i_faddr: 0, i_osd2: [0; 12]
    };
    root.i_block[0] = root_data_block;
    
    let root_inode_bytes = any_as_bytes(&root);
    // Offset 128
    itable_buf[128..128+128].copy_from_slice(root_inode_bytes);
    write_block(i_table_block, &itable_buf);

    // 7. Root Dir Entries
    let mut root_data = vec![0u8; block_size as usize];
    let dot = DirectoryEntryHeader { inode: 2, rec_len: 12, name_len: 1, file_type: EXT2_FT_DIR };
    unsafe { *(root_data.as_mut_ptr() as *mut DirectoryEntryHeader) = dot; }
    root_data[8] = b'.';
    
    let dotdot = DirectoryEntryHeader { inode: 2, rec_len: (block_size - 12) as u16, name_len: 2, file_type: EXT2_FT_DIR };
    unsafe { *(root_data[12..].as_mut_ptr() as *mut DirectoryEntryHeader) = dotdot; }
    root_data[12+8] = b'.'; root_data[12+9] = b'.';

    write_block(root_data_block, &root_data);

    Ok(())
}

fn any_as_bytes<T: Sized>(p: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>()) }
}