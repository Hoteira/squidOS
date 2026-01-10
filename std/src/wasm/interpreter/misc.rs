use crate::wasm::{Value, interpreter::Interpreter, WasmModule};
use crate::wasm::leb128::Leb128;

impl Interpreter {
    pub(crate) fn execute_misc(&mut self, _opcode: u8, pc: &mut usize, bytecode: &[u8], module: &WasmModule) -> Result<(), &'static str> {
        let sub_opcode = Leb128::decode_u32(bytecode, pc);
        
        match sub_opcode {
            // i32.trunc_sat_f32_s
            0 => {
                let f = self.pop_f32()?;
                self.stack.push(Value::I32(f as i32));
            }
            // i32.trunc_sat_f32_u
            1 => {
                let f = self.pop_f32()?;
                self.stack.push(Value::I32(f as u32 as i32));
            }
            // i32.trunc_sat_f64_s
            2 => {
                let f = self.pop_f64()?;
                self.stack.push(Value::I32(f as i32));
            }
            // i32.trunc_sat_f64_u
            3 => {
                let f = self.pop_f64()?;
                self.stack.push(Value::I32(f as u32 as i32));
            }
            // i64.trunc_sat_f32_s
            4 => {
                let f = self.pop_f32()?;
                self.stack.push(Value::I64(f as i64));
            }
            // i64.trunc_sat_f32_u
            5 => {
                let f = self.pop_f32()?;
                self.stack.push(Value::I64(f as u64 as i64));
            }
            // i64.trunc_sat_f64_s
            6 => {
                let f = self.pop_f64()?;
                self.stack.push(Value::I64(f as i64));
            }
            // i64.trunc_sat_f64_u
            7 => {
                let f = self.pop_f64()?;
                self.stack.push(Value::I64(f as u64 as i64));
            }
            // memory.init
            8 => {
                let seg_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let _mem_idx = Leb128::decode_u32(bytecode, pc);
                let n = self.pop_i32()? as usize;
                let src = self.pop_i32()? as usize;
                let dst = self.pop_i32()? as usize;
                
                if seg_idx >= module.datas.len() {
                    return Err("memory.init: segment index out of bounds");
                }
                
                if self.dropped_data.get(seg_idx).copied().unwrap_or(false) {
                    return Err("memory.init: segment dropped");
                }

                let segment = &module.datas[seg_idx];
                if src + n > segment.data.len() {
                    return Err("memory.init: source out of bounds");
                }

                if dst + n > self.memory.len() {
                    return Err("memory.init: destination out of bounds");
                }

                self.memory[dst..dst+n].copy_from_slice(&segment.data[src..src+n]);
            }
            // data.drop
            9 => {
                let seg_idx = Leb128::decode_u32(bytecode, pc) as usize;
                if seg_idx < self.dropped_data.len() {
                    self.dropped_data[seg_idx] = true;
                }
            }
            // memory.copy
            10 => {
                let _dst_mem = Leb128::decode_u32(bytecode, pc);
                let _src_mem = Leb128::decode_u32(bytecode, pc);
                let n = self.pop_i32()? as usize;
                let src = self.pop_i32()? as usize;
                let dst = self.pop_i32()? as usize;
                
                if src + n > self.memory.len() || dst + n > self.memory.len() {
                    return Err("memory.copy OOB");
                }
                
                self.memory.copy_within(src..src + n, dst);
            }
            // memory.fill
            11 => {
                let _mem_idx = Leb128::decode_u32(bytecode, pc);
                let n = self.pop_i32()? as usize;
                let val = self.pop_i32()? as u8;
                let dst = self.pop_i32()? as usize;
                
                if dst + n > self.memory.len() {
                    return Err("memory.fill OOB");
                }
                
                for i in 0..n {
                    self.memory[dst + i] = val;
                }
            }
            // table.init
            12 => {
                let elem_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let table_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let n = self.pop_i32()? as usize;
                let src = self.pop_i32()? as usize;
                let dst = self.pop_i32()? as usize;

                if elem_idx >= module.elements.len() || table_idx >= self.tables.len() {
                    return Err("table.init OOB index");
                }
                if self.dropped_elem.get(elem_idx).copied().unwrap_or(false) {
                    return Err("table.init: elem dropped");
                }

                let elem = &module.elements[elem_idx];
                if src + n > elem.function_indices.len() { return Err("table.init: src OOB"); }
                
                let table = &mut self.tables[table_idx];
                if dst + n > table.len() { return Err("table.init: dst OOB"); }

                for i in 0..n {
                    table[dst + i] = elem.function_indices[src + i];
                }
            }
            // elem.drop
            13 => {
                let elem_idx = Leb128::decode_u32(bytecode, pc) as usize;
                if elem_idx < self.dropped_elem.len() {
                    self.dropped_elem[elem_idx] = true;
                }
            }
            // table.copy
            14 => {
                let dst_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let src_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let n = self.pop_i32()? as usize;
                let src = self.pop_i32()? as usize;
                let dst = self.pop_i32()? as usize;

                if src_idx >= self.tables.len() || dst_idx >= self.tables.len() {
                    return Err("table.copy OOB index");
                }

                if src + n > self.tables[src_idx].len() || dst + n > self.tables[dst_idx].len() {
                    return Err("table.copy OOB offset");
                }

                if src_idx == dst_idx {
                    for i in 0..n {
                        let val = self.tables[src_idx][src + i];
                        self.tables[dst_idx][dst + i] = val;
                    }
                } else {
                    for i in 0..n {
                        self.tables[dst_idx][dst + i] = self.tables[src_idx][src + i];
                    }
                }
            }
            // table.grow
            15 => {
                let table_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let n = self.pop_i32()? as usize;
                let val = self.pop_i32()? as u32; // init value

                if table_idx >= self.tables.len() { return Err("table.grow OOB index"); }
                let table = &mut self.tables[table_idx];
                let old_size = table.len();
                table.resize(old_size + n, val);
                self.stack.push(Value::I32(old_size as i32));
            }
            // table.size
            16 => {
                let table_idx = Leb128::decode_u32(bytecode, pc) as usize;
                if table_idx >= self.tables.len() { return Err("table.size OOB index"); }
                self.stack.push(Value::I32(self.tables[table_idx].len() as i32));
            }
            // table.fill
            17 => {
                let table_idx = Leb128::decode_u32(bytecode, pc) as usize;
                let n = self.pop_i32()? as usize;
                let val = self.pop_i32()? as u32;
                let dst = self.pop_i32()? as usize;

                if table_idx >= self.tables.len() { return Err("table.fill OOB index"); }
                let table = &mut self.tables[table_idx];
                if dst + n > table.len() { return Err("table.fill OOB offset"); }
                for i in 0..n { table[dst + i] = val; }
            }
            _ => {
                crate::debugln!("Misc Op {:#x} unimplemented", sub_opcode);
                return Err("unimplemented misc op");
            }
        }
        Ok(())
    }
}
