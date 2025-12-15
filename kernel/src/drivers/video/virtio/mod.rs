pub mod consts;
pub mod structs;
pub mod queue;
pub mod util;

use alloc::vec::Vec;
use core::ptr::write_volatile;
use crate::drivers::pci::{PciCapability, PciDevice};
use crate::{debugln, println};
use self::consts::*;
use self::structs::*;
use self::queue::*;
use self::util::*;

pub static mut COMMON_CFG_ADDR: u64 = 0;

pub fn init() {
    let virtio_opt = crate::drivers::pci::find_device(0x1AF4, 0x1050);

    if virtio_opt.is_none() {
        debugln!("VirtIO GPU: Device not found.");
        return;
    }

    let virtio = virtio_opt.unwrap();
    debugln!("VirtIO GPU: Found device at Bus {}, Device {}, Func {}", virtio.bus, virtio.device, virtio.function);

    if virtio.enable_bus_mastering() {
        debugln!("VirtIO GPU: Bus mastering enabled.");
    } else {
        debugln!("VirtIO GPU: Failed to enable bus mastering.");
    }

    let caps = virtio.list_capabilities();
    let virtio_caps = parse_virtio_caps(&virtio, &caps);

    let mut common_cfg_ptr: *mut u8 = core::ptr::null_mut();
    let mut notify_base: u64 = 0;
    let mut notify_multiplier: u32 = 0;

    for cap in virtio_caps {
        if cap.cfg_type == VIRTIO_CAP_COMMON {
            if let Some(bar_base) = virtio.get_bar(cap.bar) {
                let addr = (bar_base as u64) + (cap.offset as u64);
                common_cfg_ptr = addr as *mut u8;
                unsafe { COMMON_CFG_ADDR = addr; }
                debugln!("VirtIO GPU: Common Config found at BAR {} offset {:#x} -> Phys {:#x}", cap.bar, cap.offset, addr);
            }
        } else if cap.cfg_type == VIRTIO_CAP_NOTIFY {
             if let Some(bar_base) = virtio.get_bar(cap.bar) {
                 notify_base = (bar_base as u64) + (cap.offset as u64);
                 notify_multiplier = virtio.read_capability_data(cap.offset as u8, 16);
             }
        }
    }

    if common_cfg_ptr.is_null() {
        debugln!("VirtIO GPU: Could not find Common Config capability.");
        return;
    }

    unsafe {
        write_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS, 0);

        let mut status = read_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS);
        status |= STATUS_ACKNOWLEDGE;
        write_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS, status);

        status |= STATUS_DRIVER;
        write_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS, status);

        // Feature Negotiation
        // Read Device Features (Select 1 for bits 32-63)
        write_common_u32(common_cfg_ptr, OFF_DEVICE_FEATURE_SELECT, 1);
        let device_features_high = read_common_u32(common_cfg_ptr, OFF_DEVICE_FEATURE);
        
        let mut driver_features_high = 0;
        if (device_features_high & (1 << 0)) != 0 { // VIRTIO_F_VERSION_1 is bit 32 (bit 0 of high dword)
             driver_features_high |= 1 << 0;
             debugln!("VirtIO GPU: Negotiated VIRTIO_F_VERSION_1");
        }

        // Write Driver Features
        write_common_u32(common_cfg_ptr, OFF_DRIVER_FEATURE_SELECT, 1);
        write_common_u32(common_cfg_ptr, OFF_DRIVER_FEATURE, driver_features_high);
        
        // Select 0 for bits 0-31 (if we wanted to negotiate low bits, but we don't need any yet)
        write_common_u32(common_cfg_ptr, OFF_DRIVER_FEATURE_SELECT, 0);
        write_common_u32(common_cfg_ptr, OFF_DRIVER_FEATURE, 0);

        status |= STATUS_FEATURES_OK;
        write_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS, status);

        let final_status = read_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS);
        if (final_status & STATUS_FEATURES_OK) == 0 {
            debugln!("VirtIO GPU: Features negotiation failed.");
            return;
        }

        let num_queues = read_common_u16(common_cfg_ptr, OFF_NUM_QUEUES);
        debugln!("VirtIO GPU: Num Queues: {}", num_queues);
        debugln!("VirtIO GPU: Notify Base: {:#x}, Multiplier: {}", notify_base, notify_multiplier);

        setup_queue(common_cfg_ptr, 0, notify_base, notify_multiplier);
        
        if num_queues > 1 {
            setup_queue(common_cfg_ptr, 1, notify_base, notify_multiplier);
        }

        status |= STATUS_DRIVER_OK;
        write_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS, status);

        debugln!("VirtIO GPU: Initialization complete (Driver OK). Status: {:#x}", read_common_u8(common_cfg_ptr, OFF_DEVICE_STATUS));
    }
}

pub fn parse_virtio_caps(pci_device: &PciDevice, caps: &[PciCapability]) -> Vec<VirtioPciCap> {
    let mut virtio_caps = Vec::new();

    for cap in caps.iter() {
        if cap.id != 0x09 {
            continue;
        }

        let cfg_type = pci_device.read_u8(cap.offset as u32 + 3);
        let bar      = pci_device.read_u8(cap.offset as u32 + 4);
        let offset   = pci_device.read_u32(cap.offset as u32 + 8);
        let length   = pci_device.read_u32(cap.offset as u32 + 12);

        virtio_caps.push(VirtioPciCap { cfg_type, bar, offset, length });
    }

    virtio_caps
}

pub unsafe fn start_gpu(width: u32, height: u32, phys_buffer: u64) {
    let req_info = VirtioGpuCtrlHeader {
        type_: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
        flags: 0,
        fence_id: 0,
        ctx_id: 0,
        ring_idx: 0,
        padding: [0; 3],
    };
    let mut resp_info: VirtioGpuRespDisplayInfo = core::mem::zeroed();

    send_command_simple(
        &req_info as *const _ as u64,
        core::mem::size_of_val(&req_info) as u32,
        &resp_info as *const _ as u64,
        core::mem::size_of_val(&resp_info) as u32,
    );

    debugln!("VirtIO GPU: Display Info Type: {:#x}", resp_info.hdr.type_);

    let req_create = VirtioGpuResourceCreate2d {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        resource_id: 1,
        format: 1,
        width,
        height,
    };
    let mut resp_create: VirtioGpuCtrlHeader = core::mem::zeroed();

    send_command_simple(
        &req_create as *const _ as u64,
        core::mem::size_of_val(&req_create) as u32,
        &resp_create as *const _ as u64,
        core::mem::size_of_val(&resp_create) as u32,
    );
    debugln!("VirtIO GPU: Create 2D Resp: {:#x}", resp_create.type_);

    // 3. Attach Backing
    // We need a contiguous struct of (AttachBacking + MemEntry)
    #[repr(C)]
    struct AttachRequest {
        hdr: VirtioGpuResourceAttachBacking,
        entry: VirtioGpuMemEntry,
    }

    let req_attach = AttachRequest {
        hdr: VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHeader {
                type_: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                ring_idx: 0,
                padding: [0; 3],
            },
            resource_id: 1,
            nr_entries: 1,
        },
        entry: VirtioGpuMemEntry {
            addr: phys_buffer,
            length: width * height * 4,
            padding: 0,
        },
    };
    let mut resp_attach: VirtioGpuCtrlHeader = core::mem::zeroed();

    send_command_simple(
        &req_attach as *const _ as u64,
        core::mem::size_of_val(&req_attach) as u32,
        &resp_attach as *const _ as u64,
        core::mem::size_of_val(&resp_attach) as u32,
    );
    debugln!("VirtIO GPU: Attach Backing Resp: {:#x}", resp_attach.type_);

    // 4. Set Scanout
    let req_scanout = VirtioGpuSetScanout {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_SET_SCANOUT,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        r: VirtioGpuRect { x: 0, y: 0, width, height },
        scanout_id: 0,
        resource_id: 1,
    };
    let mut resp_scanout: VirtioGpuCtrlHeader = core::mem::zeroed();

    send_command_simple(
        &req_scanout as *const _ as u64,
        core::mem::size_of_val(&req_scanout) as u32,
        &resp_scanout as *const _ as u64,
        core::mem::size_of_val(&resp_scanout) as u32,
    );
    debugln!("VirtIO GPU: Set Scanout Resp: {:#x}", resp_scanout.type_);

    debugln!("VirtIO GPU: Started. Scanout set to Resource 1.");
}

pub unsafe fn flush(x: u32, y: u32, width: u32, height: u32, screen_width: u32) {
    let offset = (y as u64 * screen_width as u64 + x as u64) * 4;
    
    let req_transfer = VirtioGpuTransferToHost2d {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        r: VirtioGpuRect { x, y, width, height },
        offset,
        resource_id: 1,
        padding: 0,
    };
    let mut resp_transfer: VirtioGpuCtrlHeader = core::mem::zeroed();

    let _ = send_command_simple(
        &req_transfer as *const _ as u64,
        core::mem::size_of_val(&req_transfer) as u32,
        &resp_transfer as *const _ as u64,
        core::mem::size_of_val(&resp_transfer) as u32,
    );

    let req_flush = VirtioGpuResourceFlush {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        r: VirtioGpuRect { x, y, width, height },
        resource_id: 1,
        padding: 0,
    };
    let mut resp_flush: VirtioGpuCtrlHeader = core::mem::zeroed();

    let _ = send_command_simple(
        &req_flush as *const _ as u64,
        core::mem::size_of_val(&req_flush) as u32,
        &resp_flush as *const _ as u64,
        core::mem::size_of_val(&resp_flush) as u32,
    );
}

unsafe fn dump_debug_regs() {
    let common = COMMON_CFG_ADDR as *mut u8;
    if common.is_null() { return; }

    debugln!("--- VirtIO Debug Dump ---");
    let device_status = read_common_u8(common, OFF_DEVICE_STATUS);
    debugln!("Device Status: {:#x}", device_status);

    // Read Negotiated Features (Selector 0 and 1)
    write_common_u32(common, OFF_DRIVER_FEATURE_SELECT, 0);
    let features_lo = read_common_u32(common, OFF_DRIVER_FEATURE);
    write_common_u32(common, OFF_DRIVER_FEATURE_SELECT, 1);
    let features_hi = read_common_u32(common, OFF_DRIVER_FEATURE);
    debugln!("Driver Features: Lo={:#x}, Hi={:#x}", features_lo, features_hi);

    write_common_u32(common, OFF_DEVICE_FEATURE_SELECT, 0);
    let dev_features_lo = read_common_u32(common, OFF_DEVICE_FEATURE);
    write_common_u32(common, OFF_DEVICE_FEATURE_SELECT, 1);
    let dev_features_hi = read_common_u32(common, OFF_DEVICE_FEATURE);
    debugln!("Device Features: Lo={:#x}, Hi={:#x}", dev_features_lo, dev_features_hi);

    // Check Queue 0
    write_common_u16(common, OFF_QUEUE_SELECT, 0);
    let q0_ready = read_common_u16(common, OFF_QUEUE_ENABLE);
    let q0_size = read_common_u16(common, OFF_QUEUE_SIZE);
    let q0_desc = read_common_u64(common, OFF_QUEUE_DESC);
    debugln!("Queue 0: Ready={}, Size={}, Desc={:#x}", q0_ready, q0_size, q0_desc);

    // Check Queue 1
    write_common_u16(common, OFF_QUEUE_SELECT, 1);
    let q1_ready = read_common_u16(common, OFF_QUEUE_ENABLE);
    let q1_size = read_common_u16(common, OFF_QUEUE_SIZE);
    let q1_desc = read_common_u64(common, OFF_QUEUE_DESC);
    debugln!("Queue 1: Ready={}, Size={}, Desc={:#x}", q1_ready, q1_size, q1_desc);
    debugln!("-------------------------");
}

pub unsafe fn setup_cursor(width: u32, height: u32, phys_buffer: u64, hot_x: u32, hot_y: u32) -> bool {
    dump_debug_regs();
    debugln!("VirtIO Debug: setup_cursor called with w={} h={} phys={:#x} hot={},{}", width, height, phys_buffer, hot_x, hot_y);
    
    let cursor_w = 64;
    let cursor_h = 64;
    debugln!("VirtIO Debug: Forcing cursor size to {}x{}", cursor_w, cursor_h);

    // 1. Create Cursor Resource (ID 2)
    debugln!("VirtIO Debug: Sending RESOURCE_CREATE_2D (ID 2)...");
    let req_create = VirtioGpuResourceCreate2d {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        resource_id: 2, // ID 2 for Cursor
        format: 1,      // B8G8R8A8
        width: cursor_w,
        height: cursor_h,
    };
    let mut resp_create: VirtioGpuCtrlHeader = core::mem::zeroed();
    
    // Create is a control command (Queue 0)
    let _ = send_command_simple(
        &req_create as *const _ as u64,
        core::mem::size_of_val(&req_create) as u32,
        &resp_create as *const _ as u64,
        core::mem::size_of_val(&resp_create) as u32,
    );
    debugln!("VirtIO Debug: Create 2D Resp Type: {:#x}", resp_create.type_);

    if resp_create.type_ != VIRTIO_GPU_RESP_OK_NODATA {
        debugln!("VirtIO Debug: Create 2D Failed!");
        return false;
    }

    // 2. Attach Backing
    debugln!("VirtIO Debug: Sending RESOURCE_ATTACH_BACKING (ID 2, Addr {:#x})...", phys_buffer);
    #[repr(C)]
    struct AttachRequest {
        hdr: VirtioGpuResourceAttachBacking,
        entry: VirtioGpuMemEntry,
    }

    let req_attach = AttachRequest {
        hdr: VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHeader {
                type_: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                ring_idx: 0,
                padding: [0; 3],
            },
            resource_id: 2,
            nr_entries: 1,
        },
        entry: VirtioGpuMemEntry {
            addr: phys_buffer,
            length: cursor_w * cursor_h * 4, // 16KB
            padding: 0,
        },
    };
    let mut resp_attach: VirtioGpuCtrlHeader = core::mem::zeroed();

    // Attach is a control command (Queue 0)
    let _ = send_command_simple(
        &req_attach as *const _ as u64,
        core::mem::size_of_val(&req_attach) as u32,
        &resp_attach as *const _ as u64,
        core::mem::size_of_val(&resp_attach) as u32,
    );
    debugln!("VirtIO Debug: Attach Backing Resp Type: {:#x}", resp_attach.type_);

    // 3. Transfer Data (Upload)
    debugln!("VirtIO Debug: Sending TRANSFER_TO_HOST_2D (ID 2)...");
    let req_transfer = VirtioGpuTransferToHost2d {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        r: VirtioGpuRect { x: 0, y: 0, width: cursor_w, height: cursor_h },
        offset: 0,
        resource_id: 2,
        padding: 0,
    };
    let mut resp_transfer: VirtioGpuCtrlHeader = core::mem::zeroed();

    // Transfer is a control command (Queue 0)
    let _ = send_command_simple(
        &req_transfer as *const _ as u64,
        core::mem::size_of_val(&req_transfer) as u32,
        &resp_transfer as *const _ as u64,
        core::mem::size_of_val(&resp_transfer) as u32,
    );
    debugln!("VirtIO Debug: Transfer Resp Type: {:#x}", resp_transfer.type_);

    // 4. Update Cursor (Enable it)
    debugln!("VirtIO Debug: Sending UPDATE_CURSOR (ID 2, Pos 500,500)...");
    let req_update = VirtioGpuUpdateCursor {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_UPDATE_CURSOR,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        pos: VirtioGpuCursorPos {
            scanout_id: 0,
            x: 500,
            y: 500,
            padding: 0,
        },
        resource_id: 2,
        hot_x,
        hot_y,
        padding: 0,
    };
    let mut resp_update: VirtioGpuCtrlHeader = core::mem::zeroed();

    // Update cursor can go on Cursor Queue (Queue 1)
    if !send_cursor_command(
        &req_update as *const _ as u64,
        core::mem::size_of_val(&req_update) as u32,
        &resp_update as *const _ as u64,
        core::mem::size_of_val(&resp_update) as u32,
    ) { return false; }
    
    debugln!("VirtIO Debug: Update Resp Type: {:#x}", resp_update.type_);
    match resp_update.type_ {
        VIRTIO_GPU_RESP_OK_NODATA => debugln!("VirtIO Cursor Update: OK"),
        _ => {
            debugln!("VirtIO Cursor Update: Failed with response {:#x}", resp_update.type_);
            return false;
        }
    }
    
    debugln!("VirtIO GPU: Hardware Cursor setup complete.");
    true
}

pub unsafe fn move_cursor(x: u32, y: u32) {
    let req_move = VirtioGpuUpdateCursor {
        hdr: VirtioGpuCtrlHeader {
            type_: VIRTIO_GPU_CMD_MOVE_CURSOR,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        },
        pos: VirtioGpuCursorPos {
            scanout_id: 0,
            x,
            y,
            padding: 0,
        },
        resource_id: 0, // Not used for move
        hot_x: 0,
        hot_y: 0,
        padding: 0,
    };
    let mut resp_move: VirtioGpuCtrlHeader = core::mem::zeroed();

    let _ = send_cursor_command(
        &req_move as *const _ as u64,
        core::mem::size_of_val(&req_move) as u32,
        &resp_move as *const _ as u64,
        core::mem::size_of_val(&resp_move) as u32,
    );
    
    if resp_move.type_ != VIRTIO_GPU_RESP_OK_NODATA {
        // Only log once to avoid spam, or check if it's a new error
        // debugln!("VirtIO Debug: Move Cursor Failed! Type: {:#x}", resp_move.type_);
    }
}