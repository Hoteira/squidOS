use core::arch::asm;

pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_WRITABLE: u64 = 1 << 1;
pub const PAGE_USER: u64 = 1 << 2;
pub const PAGE_WRITE_THROUGH: u64 = 1 << 3;
pub const PAGE_NO_CACHE: u64 = 1 << 4;
pub const PAGE_ACCESSED: u64 = 1 << 5;
pub const PAGE_DIRTY: u64 = 1 << 6;
pub const PAGE_HUGE: u64 = 1 << 7;
pub const PAGE_GLOBAL: u64 = 1 << 8;
pub const PAGE_NO_EXECUTE: u64 = 1 << 63;

pub const PAGE_SIZE: u64 = 4096;

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub fn zero(&mut self) {
        for i in 0..512 {
            self.entries[i] = 0;
        }
    }
}

// Helper to access physical memory directly (assuming identity map or similar)
pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    let cr3: u64;
    asm!("mov {}, cr3", out(reg) cr3);
    let phys = cr3 & 0x000FFFFFFFFFF000;
    &mut *(phys as *mut PageTable)
}

pub unsafe fn get_table<'a>(entry: u64) -> Option<&'a mut PageTable> {
    if entry & PAGE_PRESENT == 0 {
        return None;
    }
    // If huge page bit is set at this level (not expected for L4/L3 usually, but L2 yes), return None 
    // because it points to memory, not a table.
    if entry & PAGE_HUGE != 0 {
        return None;
    }
    
    let phys = entry & 0x000FFFFFFFFFF000;
    Some(&mut *(phys as *mut PageTable))
}

pub unsafe fn get_entry_ptr(virt: u64) -> Option<*mut u64> {
    let p4 = active_level_4_table();
    let p4_idx = (virt >> 39) & 0x1FF;
    
    let p3_entry = p4.entries[p4_idx as usize];
    let p3 = get_table(p3_entry)?;
    let p3_idx = (virt >> 30) & 0x1FF;

    let p2_entry = p3.entries[p3_idx as usize];
    let p2 = get_table(p2_entry)?;
    let p2_idx = (virt >> 21) & 0x1FF;

    let p1_entry = p2.entries[p2_idx as usize];
    let p1 = get_table(p1_entry)?; // This fails if P2 is a huge page
    let p1_idx = (virt >> 12) & 0x1FF;

    Some(&mut p1.entries[p1_idx as usize] as *mut u64)
}

pub unsafe fn translate_addr(virt: u64) -> Option<u64> {
    let p4 = active_level_4_table();
    let p4_idx = (virt >> 39) & 0x1FF;
    
    let p3_entry = p4.entries[p4_idx as usize];
    if p3_entry & PAGE_PRESENT == 0 { return None; }
    if p3_entry & PAGE_HUGE != 0 {
        // 1GB Page
        let offset = virt & 0x3FFFFFFF;
        return Some((p3_entry & 0x000FFFFFC0000000) + offset);
    }

    let p3 = &mut *((p3_entry & 0x000FFFFFFFFFF000) as *mut PageTable);
    let p3_idx = (virt >> 30) & 0x1FF;
    
    let p2_entry = p3.entries[p3_idx as usize];
    if p2_entry & PAGE_PRESENT == 0 { return None; }
    if p2_entry & PAGE_HUGE != 0 {
        // 2MB Page
        let offset = virt & 0x1FFFFF;
        return Some((p2_entry & 0x000FFFFFFFE00000) + offset);
    }

    let p2 = &mut *((p2_entry & 0x000FFFFFFFFFF000) as *mut PageTable);
    let p2_idx = (virt >> 21) & 0x1FF;

    let p1_entry = p2.entries[p2_idx as usize];
    if p1_entry & PAGE_PRESENT == 0 { return None; }
    
    let p1 = &mut *((p1_entry & 0x000FFFFFFFFFF000) as *mut PageTable);
    let p1_idx = (virt >> 12) & 0x1FF;
    
    let entry = p1.entries[p1_idx as usize];
    if entry & PAGE_PRESENT == 0 { return None; }

    let offset = virt & 0xFFF;
    Some((entry & 0x000FFFFFFFFFF000) + offset)
}
