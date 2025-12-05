use alloc::vec::Vec;
use elfic::{Elf64, Elf64Phdr, ProgramType, ProgramFlags, Elf64Rela, Elf64Sym};
use crate::memory::{pmm, vmm, paging};
use crate::debugln;

pub fn load_elf(data: &[u8]) -> Result<u64, alloc::string::String> {
    let elf = Elf64::new(data).map_err(|e| alloc::format!("ELF Parse Error: {:?}", e))?;

    // Check for PIE (Position Independent Executable) -> Type 3 (ET_DYN)
    let load_base = if elf.header.e_type == 3 { 0x80_0000_0000 } else { 0 };
    
    if load_base > 0 {
        debugln!("[ELF] Detected PIE Executable. Relocating to base {:#x}", load_base);
    }

    // 1. Load Segments
    for phdr in elf.program_headers() {
        if ProgramType::from(phdr.p_type) == ProgramType::Load {
            if phdr.p_memsz == 0 { continue; }

            let virt_start = phdr.p_vaddr + load_base;
            let virt_end = virt_start + phdr.p_memsz;
            
            // debugln!("[ELF] Loading Segment: virt_start={:#x}, memsz={}", virt_start, phdr.p_memsz);

            let page_start = virt_start & !(paging::PAGE_SIZE - 1);
            let page_end = (virt_end + paging::PAGE_SIZE - 1) & !(paging::PAGE_SIZE - 1);
            
            let pages = (page_end - page_start) / paging::PAGE_SIZE;

            let mut flags = paging::PAGE_PRESENT | paging::PAGE_USER;
            if (phdr.p_flags & ProgramFlags::WRITE) != 0 {
                flags |= paging::PAGE_WRITABLE;
            }

            for i in 0..pages {
                let frame = unsafe { pmm::allocate_frame().expect("OOM loading ELF") };
                let virt = page_start + (i * paging::PAGE_SIZE);
                vmm::map_page(virt, frame, flags);
                
                unsafe {
                    core::ptr::write_bytes(frame as *mut u8, 0, paging::PAGE_SIZE as usize);
                }
            }

            let file_size = phdr.p_filesz as usize;
            if file_size > 0 {
                let segment_data = &data[phdr.p_offset as usize..(phdr.p_offset as usize + file_size)];
                
                let mut remaining = file_size;
                let mut src_offset = 0;
                let mut current_virt = virt_start;
                
                while remaining > 0 {
                    let page_offset = (current_virt % paging::PAGE_SIZE) as usize;
                    let to_copy = core::cmp::min(remaining, (paging::PAGE_SIZE - page_offset as u64) as usize);
                    
                    let phys_addr = unsafe { paging::translate_addr(current_virt).expect("Translation failed during load") };
                    
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            segment_data.as_ptr().add(src_offset),
                            phys_addr as *mut u8,
                            to_copy
                        );
                    }
                    
                    remaining -= to_copy;
                    src_offset += to_copy;
                    current_virt += to_copy as u64;
                }
            }
        }
    }

    // 2. Find Symbol Table (DYNSYM) for relocations
    let mut dynsym_shdr: Option<&elfic::Elf64Shdr> = None;
    for shdr in elf.section_headers() {
        if shdr.sh_type == 11 { // SHT_DYNSYM
            dynsym_shdr = Some(shdr);
            break;
        }
    }

    // 3. Perform Relocations
    if load_base > 0 {
        for shdr in elf.section_headers() {
            if shdr.sh_type == 4 { // SHT_RELA
                 let num_entries = shdr.sh_size / shdr.sh_entsize;
                 let offset = shdr.sh_offset as usize;
                 
                 // debugln!("[ELF] Processing Relocation Section: entries={}", num_entries);

                 let relas = unsafe {
                     core::slice::from_raw_parts(
                         data.as_ptr().add(offset) as *const Elf64Rela,
                         num_entries as usize
                     )
                 };

                 for rela in relas {
                     let r_type = rela.get_type();
                     let r_sym = rela.get_symbol();
                     let target_virt = rela.r_offset + load_base;
                     
                     let mut val: u64 = 0;
                     let mut found_val = false;

                     match r_type {
                         8 => { // R_X86_64_RELATIVE
                             // B + A
                             val = load_base.wrapping_add(rela.r_addend as u64);
                             found_val = true;
                         }
                         1 | 6 | 7 => { // R_X86_64_64 (1), GLOB_DAT (6), JUMP_SLOT (7)
                             // S + A (64) or S (GLOB_DAT/JUMP_SLOT)
                             // We need to lookup the symbol value
                             if let Some(sym_tab) = dynsym_shdr {
                                 let sym_offset = sym_tab.sh_offset as usize + (r_sym as usize * core::mem::size_of::<Elf64Sym>());
                                 if sym_offset < data.len() {
                                     let sym = unsafe { &*(data.as_ptr().add(sym_offset) as *const Elf64Sym) };
                                     
                                     if sym.st_shndx != 0 { // If defined in ELF
                                         val = sym.st_value + load_base;
                                     } else {
                                         // Undefined symbol (Imported).
                                         // In a real OS, we would look this up in Kernel exports or Shared Libs.
                                         // For now, we leave it as 0 (weak) or fail.
                                         // debugln!("[ELF] Warning: Undefined symbol index {}", r_sym);
                                     }
                                     
                                     if r_type == 1 { // R_X86_64_64 adds addend
                                         val = val.wrapping_add(rela.r_addend as u64);
                                     }
                                     found_val = true;
                                 }
                             }
                         }
                         _ => {
                             // Ignore others for now (NONE, COPY, etc)
                         }
                     }

                     if found_val {
                         // Write `val` to `target_virt`
                         if let Some(phys) = unsafe { paging::translate_addr(target_virt) } {
                             unsafe {
                                 *(phys as *mut u64) = val;
                             }
                         }
                     }
                 }
            }
        }
    }

    Ok(elf.header.e_entry + load_base)
}
