ENTRY(_start)

SECTIONS {
    . = 0x00100000;

    KERNEL_START = .;

    .start : {
        *(.start)
    }
    
    . = ALIGN(4096);
    .text : {
        *(.text .text.*)
    }

    . = ALIGN(4096);
    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4096);
    .data : {
        *(.data .data.*)
    }

    . = ALIGN(4096);
    .bss : {
        *(.bss .bss.*)
    }

    . = ALIGN(4096);
    .eh_frame : {
        *(.eh_frame .eh_frame.*)
    }
    .eh_frame_hdr : {
        *(.eh_frame_hdr .eh_frame_hdr.*)
    }

    . = KERNEL_START + 0x00100000 - 2;

    .phony : {
        SHORT(0xFFFF);
    }
}