
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    mmap: MemoryMap,
    rsdp: Rsdp,
    pub tss: u16,
    vbe: VbeInfoBlock,
    mode: VbeModeInfoBlock,
    pub pml4: u64,
    pub kernel_stack: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub memory_type: u32,
    pub reserved_acpi: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    pub entries: [MemoryMapEntry; 32],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TaskStateSegment {
    pub link: u16,
    pub padding_0: u16,
    pub esp0: u32,
    pub ss0: u16,
    pub padding_1: u16,
    pub esp1: u32,
    pub ss1: u16,
    pub padding_2: u16,
    pub esp2: u32,
    pub ss2: u16,
    pub padding_3: u16,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u16,
    pub padding_4: u16,
    pub cs: u16,
    pub padding_5: u16,
    pub ss: u16,
    pub padding_6: u16,
    pub ds: u16,
    pub padding_7: u16,
    pub fs: u16,
    pub padding_8: u16,
    pub gs: u16,
    pub padding_9: u16,
    pub ldtr: u16,
    pub padding_10: u16,
    pub padding_11: u16,
    pub iopb: u16,
    pub ssp: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeInfoBlock {
    pub signature: [u8; 4],
    pub version: u16,
    pub oem: [u16; 2],
    pub dunno: [u8; 4],
    pub video_ptr: u32,
    pub memory_size: u16,
    pub reserved: [u8; 492],
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VbeModeInfoBlock {
    attributes: u16,
    window_a: u8,
    window_b: u8,
    granularity: u16,
    window_size: u16,
    segment_a: u16,
    segment_b: u16,
    win_func_ptr: u32,
    pitch: u16,
    width: u16,
    height: u16,
    w_char: u8,
    y_char: u8,
    planes: u8,
    bpp: u8,
    banks: u8,
    memory_model: u8,
    bank_size: u8,
    image_pages: u8,
    reserved0: u8,
    red_mask_size: u8,
    red_field_position: u8,
    green_mask_size: u8,
    green_field_position: u8,
    blue_mask_size: u8,
    blue_field_position: u8,
    reserved_mask_size: u8,
    reserved_field_position: u8,
    direct_color_mode_info: u8,
    framebuffer: u32,
    reserved1: u32,
    reserved2: u16,
    lin_bytes_per_scan_line: u16,
    bnk_image_pages: u8,
    lin_image_pages: u8,
    lin_red_mask_size: u8,
    lin_red_field_position: u8,
    lin_green_mask_size: u8,
    lin_green_field_position: u8,
    lin_blue_mask_size: u8,
    lin_blue_field_position: u8,
    lin_reserved_mask_size: u8,
    lin_reserved_field_position: u8,
    max_pixel_clock: u32,
    reserved3: [u8; 189],
}