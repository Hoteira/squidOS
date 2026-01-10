use crate::wasm::{WasmModule, FunctionType, CodeSection, LocalEntry, ValueType};
use crate::wasm::leb128::Leb128;
use crate::rust_alloc::vec::Vec;

pub struct Parser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn parse(&mut self) -> Result<WasmModule, &'static str> {
        let mut module = WasmModule::empty();

        // 1. Verify Magic Number (\0asm)
        if &self.data[0..4] != b"\0asm" {
            return Err("Invalid WASM magic number");
        }
        self.offset += 4;

        // 2. Verify Version (1)
        module.version = u32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;

        if module.version != 1 {
            return Err("Unsupported WASM version");
        }

        // 3. Parse Sections
        while self.offset < self.data.len() {
            let section_id = self.data[self.offset];
            self.offset += 1;
            let section_size = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let section_end = self.offset + section_size;

            match section_id {
                1 => self.parse_type_section(&mut module)?,
                2 => self.parse_import_section(&mut module)?,
                3 => self.parse_function_section(&mut module)?,
                4 => self.parse_table_section(&mut module)?,
                5 => self.parse_memory_section(&mut module)?,
                6 => self.parse_global_section(&mut module)?,
                7 => self.parse_export_section(&mut module)?,
                8 => module.start_func = Some(Leb128::decode_u32(self.data, &mut self.offset)),
                9 => self.parse_element_section(&mut module)?,
                10 => self.parse_code_section(&mut module)?,
                11 => self.parse_data_section(&mut module)?,
                _ => {
                    // Skip unknown/unimplemented sections for now
                    self.offset = section_end;
                }
            }

            // Ensure we are exactly at the end of the section
            self.offset = section_end;
        }

        Ok(module)
    }

    fn parse_table_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let element_type = self.data[self.offset];
            self.offset += 1;
            let flags = self.data[self.offset];
            self.offset += 1;
            let min = Leb128::decode_u32(self.data, &mut self.offset);
            let max = if flags & 1 != 0 {
                Some(Leb128::decode_u32(self.data, &mut self.offset))
            } else {
                None
            };
            module.tables.push(crate::wasm::Table { element_type, min_size: min, max_size: max });
        }
        Ok(())
    }

    fn parse_element_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let table_index = Leb128::decode_u32(self.data, &mut self.offset);
            
            // Offset expression
            let mut offset_bytecode = Vec::new();
            while self.data[self.offset] != 0x0B {
                offset_bytecode.push(self.data[self.offset]);
                self.offset += 1;
            }
            self.offset += 1;

            let func_count = Leb128::decode_u32(self.data, &mut self.offset);
            let mut function_indices = Vec::new();
            for _ in 0..func_count {
                function_indices.push(Leb128::decode_u32(self.data, &mut self.offset));
            }

            module.elements.push(crate::wasm::ElementSegment {
                table_index,
                offset_bytecode,
                function_indices,
            });
        }
        Ok(())
    }

    fn parse_import_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let mod_len = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let module_name = crate::rust_alloc::string::String::from_utf8_lossy(&self.data[self.offset..self.offset + mod_len]).into_owned();
            self.offset += mod_len;

            let name_len = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let field_name = crate::rust_alloc::string::String::from_utf8_lossy(&self.data[self.offset..self.offset + name_len]).into_owned();
            self.offset += name_len;

            let kind = self.data[self.offset];
            self.offset += 1;

            let index = Leb128::decode_u32(self.data, &mut self.offset);

            module.imports.push(crate::wasm::Import {
                module: module_name,
                name: field_name,
                kind,
                index,
            });
        }
        Ok(())
    }

    fn parse_memory_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let flags = self.data[self.offset];
            self.offset += 1;
            let min = Leb128::decode_u32(self.data, &mut self.offset);
            let max = if flags & 1 != 0 {
                Some(Leb128::decode_u32(self.data, &mut self.offset))
            } else {
                None
            };
            module.memories.push(crate::wasm::Memory { min_pages: min, max_pages: max });
        }
        Ok(())
    }

    fn parse_data_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let _index = Leb128::decode_u32(self.data, &mut self.offset); // Usually 0
            
            // Offset expression
            let mut offset_bytecode = Vec::new();
            while self.data[self.offset] != 0x0B {
                offset_bytecode.push(self.data[self.offset]);
                self.offset += 1;
            }
            self.offset += 1; // skip 0x0B

            let data_len = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let mut data = Vec::new();
            data.extend_from_slice(&self.data[self.offset..self.offset + data_len]);
            self.offset += data_len;

            module.datas.push(crate::wasm::DataSegment { offset_bytecode, data });
        }
        Ok(())
    }

    fn parse_global_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let val_type = self.parse_value_type()?;
            let mutable = self.data[self.offset] == 1;
            self.offset += 1;

            // Simple expression parser (usually just i32.const value end)
            let mut init_bytecode = Vec::new();
            while self.data[self.offset] != 0x0B {
                init_bytecode.push(self.data[self.offset]);
                self.offset += 1;
            }
            self.offset += 1; // skip 0x0B

            module.globals.push(crate::wasm::Global { val_type, mutable, init_bytecode });
        }
        Ok(())
    }

    fn parse_export_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let name_len = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let name_bytes = &self.data[self.offset..self.offset + name_len];
            self.offset += name_len;
            let name = crate::rust_alloc::string::String::from_utf8_lossy(name_bytes).into_owned();

            let kind = self.data[self.offset];
            self.offset += 1;

            let index = Leb128::decode_u32(self.data, &mut self.offset);

            module.exports.push(crate::wasm::Export { name, kind, index });
        }
        Ok(())
    }

    fn parse_type_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            if self.data[self.offset] != 0x60 {
                return Err("Invalid function type prefix");
            }
            self.offset += 1;

            // Params
            let param_count = Leb128::decode_u32(self.data, &mut self.offset);
            let mut params = Vec::new();
            for _ in 0..param_count {
                params.push(self.parse_value_type()?);
            }

            // Results
            let result_count = Leb128::decode_u32(self.data, &mut self.offset);
            let mut results = Vec::new();
            for _ in 0..result_count {
                results.push(self.parse_value_type()?);
            }

            module.types.push(FunctionType { params, results });
        }
        Ok(())
    }

    fn parse_function_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let type_idx = Leb128::decode_u32(self.data, &mut self.offset);
            module.functions.push(type_idx);
        }
        Ok(())
    }

    fn parse_code_section(&mut self, module: &mut WasmModule) -> Result<(), &'static str> {
        let count = Leb128::decode_u32(self.data, &mut self.offset);
        for _ in 0..count {
            let body_size = Leb128::decode_u32(self.data, &mut self.offset) as usize;
            let body_end = self.offset + body_size;

            // Locals
            let local_count = Leb128::decode_u32(self.data, &mut self.offset);
            let mut locals = Vec::new();
            for _ in 0..local_count {
                let n = Leb128::decode_u32(self.data, &mut self.offset);
                let val_type = self.parse_value_type()?;
                locals.push(LocalEntry { count: n, val_type });
            }

            // Bytecode (the rest of the body until body_end - 1, usually ends with 0x0B)
            let bytecode_len = body_end - self.offset;
            let mut bytecode = Vec::new();
            bytecode.extend_from_slice(&self.data[self.offset..body_end]);
            
            module.codes.push(CodeSection { locals, bytecode });
            self.offset = body_end;
        }
        Ok(())
    }

    fn parse_value_type(&mut self) -> Result<ValueType, &'static str> {
        let byte = self.data[self.offset];
        self.offset += 1;
        match byte {
            0x7F => Ok(ValueType::I32),
            0x7E => Ok(ValueType::I64),
            0x7D => Ok(ValueType::F32),
            0x7C => Ok(ValueType::F64),
            _ => Err("Invalid value type"),
        }
    }
}
