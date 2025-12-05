use alloc::string::ToString;
use crate::alloc::string::String;
use crate::syscall::malloc;
use alloc::vec::Vec;
use crate::println;

#[derive(Clone, Debug)]
pub struct File {
    pub fname: [u8; 256],
    pub size: u32,
    pub attributes: u16,
    pub permissions: u16,
    pub ptr: u32,
    pub read: bool,
    pub write: bool,
}

pub fn make_file(fname: &str) {
    let string_ptr = fname.as_ptr();
    let string_len = fname.len();

    crate::syscall::syscall(40, string_ptr as u32, 0, string_len as u32);
}

pub fn make_dir(fname: &str) {
    let string_ptr = fname.as_ptr();
    let string_len = fname.len();

    crate::syscall::syscall(42, string_ptr as u32, 0, string_len as u32);
}
impl File {
    pub fn new(fname: &str) -> File {
        let size = size(fname);
        println!("{} -> {:?}", fname, size);

        if size == 699669 {
            panic!("[x] File not found");
        } else {
            let addr = malloc(size);

            let string_ptr = fname.as_ptr();
            let string_len = fname.len();

            println!("{} -> OK", fname);

            crate::syscall::syscall(2, string_ptr as u32, addr, string_len as u32);

            println!("{} -> FILE READ", fname);

            return File {
                fname: str_to_array(fname),
                size,
                attributes: 0,
                permissions: 0,
                ptr: addr,
                read: false,
                write: false,
            };
        }
    }

    pub fn read_bytes(&mut self) -> &[u8] {
        crate::println!("{:?}", self);

        if self.ptr == 0 {
            self.size = size(&array_to_string(&self.fname));

            if self.size == 699669 {
                return &[69];
            }

            self.ptr = malloc(self.size);

            let string_ptr = self.fname.as_ptr();
            let string_len = self.fname.len();

            crate::syscall::syscall(2, string_ptr as u32, self.ptr, string_len as u32);

            return unsafe {
                core::slice::from_raw_parts(self.ptr as *const u8, self.size as usize)
            };
        } else {
            return unsafe {
                core::slice::from_raw_parts(self.ptr as *const u8, self.size as usize)
            };
        }
    }

    pub fn read_to_buffer(&self, buffer: u32) {
        let string_ptr = self.fname.as_ptr();
        let string_len = self.fname.len();

        crate::syscall::syscall(2, string_ptr as u32, buffer, string_len as u32);
    }

    pub fn write(&self, data: &[u8]) {
        let string_ptr = self.fname.as_ptr();
        let string_len = self.fname.len();

        let data_ref = (data.as_ptr() as u32, data.len() as u32);

        crate::syscall::syscall(
            38,
            string_ptr as u32,
            &data_ref as *const (u32, u32) as u32,
            string_len as u32,
        );
    }

    pub fn append(&self, data: &[u8]) {
        let string_ptr = self.fname.as_ptr();
        let string_len = self.fname.len();

        let data_ref = (data.as_ptr() as u32, data.len() as u32);

        crate::syscall::syscall(
            39,
            string_ptr as u32,
            &data_ref as *const (u32, u32) as u32,
            string_len as u32,
        );
    }

    pub fn get_name(&self) -> String {
        String::from_utf8(self.fname.to_vec()).unwrap_or("".to_string())
            .trim_end_matches(|c: char| c == '\0' || c.is_whitespace())
            .trim_end_matches(|c: char| c == '.')
            .to_string()
    }

    pub fn close(&self) {
        crate::syscall::free(self.ptr);
    }

    pub fn is_dir(&self) -> bool {
        self.attributes & 0x10 != 0
    }

    pub fn get_extension(&self) -> String {
        let s = array_to_string(&self.fname);

        s.rfind('.')
            .map(|i| &s[i..])
            .unwrap_or("")
            .to_string()
    }
}

pub fn size(fname: &str) -> u32 {
    let string_ptr = fname.as_ptr();
    let string_len = fname.len();

    crate::syscall::syscall(4, string_ptr as u32, 0, string_len as u32)
}

pub fn dir_entries(fname: &str) -> u32 {
    let string_ptr = fname.as_ptr();
    let string_len = fname.len();

    crate::syscall::syscall(28, string_ptr as u32, 0, string_len as u32)
}

pub fn get_entry(fname: &str, index: u8) -> Option<File> {
    let string_ptr = fname.as_ptr();
    let string_len = fname.len();

    let mut temp: File = File {
        fname: str_to_array(""),
        size: 0,
        attributes: 0,
        permissions: 0,
        ptr: 0,
        read: false,
        write: false,
    };

    let index = (index, &mut temp as *mut _ as u32);

    crate::syscall::syscall(29, string_ptr as u32, &index as *const _ as u32, string_len as u32);

    if temp.fname[0] == 0 { return None;} else { Some(temp) }
}


pub fn expand_path_8_3(path: &str) -> &[u8] {
    assert!(path.len() == 11, "Path must be exactly 11 characters long");

    if !path.contains(' ') {
        return path.as_bytes();
    }

    static mut BUFFER: [u8; 12] = [0; 12];

    let name_part = &path[..8];
    let ext_part = &path[8..];

    let trimmed_name = name_part.trim_end();
    let trimmed_ext = ext_part.trim_end();

    unsafe {
        let mut idx = 0;

        for byte in trimmed_name.bytes() {
            BUFFER[idx] = byte;
            idx += 1;
        }

        if !trimmed_ext.is_empty() {
            BUFFER[idx] = b'.';
            idx += 1;

            for byte in trimmed_ext.bytes() {
                BUFFER[idx] = byte;
                idx += 1;
            }
        }

        &BUFFER[..idx]
    }
}

fn str_to_array(s: &str) -> [u8; 256] {
    let mut array = [0u8; 256]; // Initialize with zeros
    let bytes = s.as_bytes(); // Get UTF-8 bytes
    let len = bytes.len().min(256); // Limit to 256 bytes
    array[..len].copy_from_slice(&bytes[..len]); // Copy bytes
    array
}

fn array_to_string(array: &[u8]) -> String {
    String::from_utf8_lossy(&array[..])
        .trim_end_matches(|c: char| c == '\0' || c.is_whitespace())
        .to_string()
}

pub fn get_file_extention(fname: &str) -> String {
    fname.rfind('.')
        .map(|i| &fname[i..])
        .unwrap_or("")
        .to_string()
}

pub fn get_file_extention_clean(fname: &str) -> String {
    fname.rfind('.')
        .map(|i| &fname[i+1..])
        .unwrap_or("")
        .to_string()
}