use crate::wasm::{Value, interpreter::Interpreter};

impl Interpreter {
    pub(crate) fn execute_conversion(&mut self, opcode: u8, _pc: &mut usize, _bytecode: &[u8]) -> Result<(), &'static str> {
        match opcode {
            0xA7 => { let v = self.pop_i64()?; self.stack.push(Value::I32(v as i32)); }
            0xA8..=0xAB => {
                let v = if opcode <= 0xA9 { Value::F32(self.pop_f32()?) } else { Value::F64(self.pop_f64()?) };
                match v {
                    Value::F32(f) => self.stack.push(Value::I32(if opcode == 0xA8 { f as i32 } else { f as u32 as i32 })),
                    Value::F64(f) => self.stack.push(Value::I32(if opcode == 0xAA { f as i32 } else { f as u32 as i32 })),
                    _ => unreachable!(),
                }
            }
            0xAC => { let v = self.pop_i32()?; self.stack.push(Value::I64(v as i64)); }
            0xAD => { let v = self.pop_i32()?; self.stack.push(Value::I64(v as u32 as i64)); }
            0xB2 => { let v = self.pop_i32()?; self.stack.push(Value::F32(v as f32)); }
            0xB3 => { let v = self.pop_i32()?; self.stack.push(Value::F32(v as u32 as f32)); }
            0xC0 => { let v = self.pop_i32()?; self.stack.push(Value::I32(v as i8 as i32)); }
            0xC1 => { let v = self.pop_i32()?; self.stack.push(Value::I32(v as i16 as i32)); }
            0xC2 => { let v = self.pop_i64()?; self.stack.push(Value::I64(v as i8 as i64)); }
            0xC3 => { let v = self.pop_i64()?; self.stack.push(Value::I64(v as i16 as i64)); }
            0xC4 => { let v = self.pop_i64()?; self.stack.push(Value::I64(v as i32 as i64)); }
            _ => return Err("Invalid conversion opcode"),
        }
        Ok(())
    }
}
