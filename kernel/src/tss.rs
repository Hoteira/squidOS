use crate::boot::TaskStateSegment;

#[repr(C, packed)]
struct Descriptor {
    size: u16,
    offset: u64,
}

pub fn set_tss(kernel_stack: u64) {
    unsafe {
        // 1. Get the current Task Register (TR) selector
        let tr: u16;
        core::arch::asm!("str {:x}", out(reg) tr);
        
        // 2. Get the GDT base address
        let mut gdt_ptr = Descriptor { size: 0, offset: 0 };
        core::arch::asm!("sgdt [{}]", in(reg) &mut gdt_ptr, options(nostack, preserves_flags));
        
        let gdt_base = gdt_ptr.offset;
        let tr_index = tr >> 3; // Selector index (TR / 8)
        
        // 3. Calculate the address of the TSS Descriptor in the GDT
        // Each descriptor is 8 bytes.
        // Note: In 64-bit mode, a TSS descriptor is 16 bytes (occupies 2 slots).
        // We need the first 8 bytes (Low part) and next 8 bytes (High part) to reconstruct the base.
        
        let tss_desc_low_ptr = (gdt_base + (tr_index as u64 * 8)) as *mut u64;
        let tss_desc_high_ptr = (gdt_base + (tr_index as u64 * 8) + 8) as *mut u64;
        
        let low = *tss_desc_low_ptr;
        let high = *tss_desc_high_ptr;
        
        // 4. Decode the TSS Base Address from the Descriptor
        // Low:  Base[23:16] (bits 39:32) | Base[15:0] (bits 31:16)
        // High: Base[63:32] (bits 31:0) | Base[31:24] (bits 63:56 of Low part, wait... typical layout)
        
        // Standard 64-bit TSS Descriptor Layout:
        // Low (64-bit): 
        //   Bit 63-56: Base 31:24
        //   Bit 55-52: Flags
        //   Bit 51-48: Limit 19:16
        //   Bit 47-40: Access Byte
        //   Bit 39-32: Base 23:16
        //   Bit 31-16: Base 15:0
        //   Bit 15-00: Limit 15:0
        // High (64-bit):
        //   Bit 31-00: Base 63:32
        //   Bit 63-32: Reserved (Zero)

        let mut base = 0u64;
        base |= (low >> 16) & 0xFFFF;          // Base 0-15
        base |= ((low >> 32) & 0xFF) << 16;    // Base 16-23
        base |= ((low >> 56) & 0xFF) << 24;    // Base 24-31
        base |= (high & 0xFFFFFFFF) << 32;     // Base 32-63
        
        // 5. Cast to TSS struct and update RSP0
        let tss_struct = base as *mut TaskStateSegment;
        (*tss_struct).rsp0 = kernel_stack;
    }
}
