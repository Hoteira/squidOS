pub mod leb128;
pub mod parser;
pub mod interpreter;
pub mod wasi;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

pub struct WasmModule {
    pub version: u32,
    pub types: crate::rust_alloc::vec::Vec<FunctionType>,
    pub imports: crate::rust_alloc::vec::Vec<Import>,
    pub functions: crate::rust_alloc::vec::Vec<u32>, // Indices into type section
    pub tables: crate::rust_alloc::vec::Vec<Table>,
    pub memories: crate::rust_alloc::vec::Vec<Memory>,
    pub exports: crate::rust_alloc::vec::Vec<Export>,
    pub globals: crate::rust_alloc::vec::Vec<Global>,
    pub codes: crate::rust_alloc::vec::Vec<CodeSection>,
    pub datas: crate::rust_alloc::vec::Vec<DataSegment>,
    pub elements: crate::rust_alloc::vec::Vec<ElementSegment>,
    pub start_func: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub element_type: u8, // 0x70 = funcref
    pub min_size: u32,
    pub max_size: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ElementSegment {
    pub table_index: u32,
    pub offset_bytecode: crate::rust_alloc::vec::Vec<u8>,
    pub function_indices: crate::rust_alloc::vec::Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module: crate::rust_alloc::string::String,
    pub name: crate::rust_alloc::string::String,
    pub kind: u8, // 0 = Func, 1 = Table, 2 = Mem, 3 = Global
    pub index: u32, // Type index if Func
}

#[derive(Debug, Clone)]
pub struct Memory {
    pub min_pages: u32,
    pub max_pages: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct DataSegment {
    pub offset_bytecode: crate::rust_alloc::vec::Vec<u8>,
    pub data: crate::rust_alloc::vec::Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub val_type: ValueType,
    pub mutable: bool,
    pub init_bytecode: crate::rust_alloc::vec::Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: crate::rust_alloc::string::String,
    pub kind: u8, // 0 = Func, 1 = Table, 2 = Mem, 3 = Global
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: crate::rust_alloc::vec::Vec<ValueType>,
    pub results: crate::rust_alloc::vec::Vec<ValueType>,
}

#[derive(Debug, Clone)]
pub struct CodeSection {
    pub locals: crate::rust_alloc::vec::Vec<LocalEntry>,
    pub bytecode: crate::rust_alloc::vec::Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LocalEntry {
    pub count: u32,
    pub val_type: ValueType,
}

impl WasmModule {
    pub fn empty() -> Self {
        Self {
            version: 0,
            types: crate::rust_alloc::vec::Vec::new(),
            imports: crate::rust_alloc::vec::Vec::new(),
            functions: crate::rust_alloc::vec::Vec::new(),
            tables: crate::rust_alloc::vec::Vec::new(),
            memories: crate::rust_alloc::vec::Vec::new(),
            exports: crate::rust_alloc::vec::Vec::new(),
            globals: crate::rust_alloc::vec::Vec::new(),
            codes: crate::rust_alloc::vec::Vec::new(),
            datas: crate::rust_alloc::vec::Vec::new(),
            elements: crate::rust_alloc::vec::Vec::new(),
            start_func: None,
        }
    }

    pub fn find_export(&self, name: &str) -> Option<u32> {
        for export in &self.exports {
            if export.name == name && export.kind == 0 {
                return Some(export.index);
            }
        }
        None
    }
}