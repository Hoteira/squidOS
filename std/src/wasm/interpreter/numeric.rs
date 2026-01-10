use crate::wasm::{Value, interpreter::Interpreter};
use crate::wasm::leb128::Leb128;
use crate::math::FloatMath;

impl Interpreter {
    pub(crate) fn execute_numeric(&mut self, opcode: u8, pc: &mut usize, bytecode: &[u8]) -> Result<(), &'static str> {
        match opcode {
            0x41 => { self.stack.push(Value::I32(Leb128::decode_i32(bytecode, pc))); }
            0x42 => { self.stack.push(Value::I64(Leb128::decode_i64(bytecode, pc))); }
            0x43 => { let mut b = [0u8; 4]; b.copy_from_slice(&bytecode[*pc..*pc+4]); *pc += 4; self.stack.push(Value::F32(f32::from_le_bytes(b))); }
            0x44 => { let mut b = [0u8; 8]; b.copy_from_slice(&bytecode[*pc..*pc+8]); *pc += 8; self.stack.push(Value::F64(f64::from_le_bytes(b))); }
            
            0x45..=0x4F => {
                let b = self.pop_i32()?;
                let a = if opcode == 0x45 { 0 } else { self.pop_i32()? };
                let r = match opcode { 0x45 => b == 0, 0x46 => a == b, 0x47 => a != b, 0x48 => a < b, 0x49 => (a as u32) < (b as u32), 0x4A => a > b, 0x4B => (a as u32) > (b as u32), 0x4C => a <= b, 0x4D => (a as u32) <= (b as u32), 0x4E => a >= b, 0x4F => (a as u32) >= (b as u32), _ => false };
                self.stack.push(Value::I32(r as i32));
            }
            0x50..=0x5A => {
                let b = self.pop_i64()?;
                let a = if opcode == 0x50 { 0 } else { self.pop_i64()? };
                let r = match opcode { 0x50 => b == 0, 0x51 => a == b, 0x52 => a != b, 0x53 => a < b, 0x54 => (a as u64) < (b as u64), 0x55 => a > b, 0x56 => (a as u64) > (b as u64), 0x57 => a <= b, 0x58 => (a as u64) <= (b as u64), 0x59 => a >= b, 0x5A => (a as u64) >= (b as u64), _ => false };
                self.stack.push(Value::I32(r as i32));
            }
            0x5B..=0x60 => {
                let b = self.pop_f32()?; let a = self.pop_f32()?;
                let r = match opcode { 0x5B => a == b, 0x5C => a != b, 0x5D => a < b, 0x5E => a > b, 0x5F => a <= b, 0x60 => a >= b, _ => false };
                self.stack.push(Value::I32(r as i32));
            }
            0x61..=0x66 => {
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                let r = match opcode { 0x61 => a == b, 0x62 => a != b, 0x63 => a < b, 0x64 => a > b, 0x65 => a <= b, 0x66 => a >= b, _ => false };
                self.stack.push(Value::I32(r as i32));
            }
            0x67..=0x69 => {
                let a = self.pop_i32()?;
                let r = match opcode { 0x67 => a.leading_zeros() as i32, 0x68 => a.trailing_zeros() as i32, 0x69 => a.count_ones() as i32, _ => 0 };
                self.stack.push(Value::I32(r));
            }
            0x6A..=0x78 => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                let r = match opcode { 0x6A => a.wrapping_add(b), 0x6B => a.wrapping_sub(b), 0x6C => a.wrapping_mul(b), 0x6D => if b == 0 { return Err("div0") } else { a / b }, 0x6E => if b == 0 { return Err("div0") } else { (a as u32 / b as u32) as i32 }, 0x6F => if b == 0 { return Err("div0") } else { a % b }, 0x70 => if b == 0 { return Err("div0") } else { (a as u32 % b as u32) as i32 }, 0x71 => a & b, 0x72 => a | b, 0x73 => a ^ b, 0x74 => a.wrapping_shl(b as u32), 0x75 => a.wrapping_shr(b as u32), 0x76 => (a as u32).wrapping_shr(b as u32) as i32, 0x77 => a.rotate_left(b as u32), 0x78 => a.rotate_right(b as u32), _ => 0 };
                self.stack.push(Value::I32(r));
            }
            0x79..=0x7B => {
                let a = self.pop_i64()?;
                let r = match opcode { 0x79 => a.leading_zeros() as i64, 0x7A => a.trailing_zeros() as i64, 0x7B => a.count_ones() as i64, _ => 0 };
                self.stack.push(Value::I64(r));
            }
            0x7C..=0x8A => {
                let b = self.pop_i64()?;
                let a = self.pop_i64()?;
                let r = match opcode { 0x7C => a.wrapping_add(b), 0x7D => a.wrapping_sub(b), 0x7E => a.wrapping_mul(b), 0x7F => if b == 0 { return Err("div0") } else { a / b }, 0x80 => if b == 0 { return Err("div0") } else { (a as u64 / b as u64) as i64 }, 0x81 => if b == 0 { return Err("div0") } else { a % b }, 0x82 => if b == 0 { return Err("div0") } else { (a as u64 % b as u64) as i64 }, 0x83 => a & b, 0x84 => a | b, 0x85 => a ^ b, 0x86 => a.wrapping_shl(b as u32), 0x87 => a.wrapping_shr(b as u32), 0x88 => (a as u64).wrapping_shr(b as u32) as i64, 0x89 => a.rotate_left(b as u32), 0x8A => a.rotate_right(b as u32), _ => 0 };
                self.stack.push(Value::I64(r));
            }
            0x8B..=0x91 => {
                let a = self.pop_f32()?;
                let r = match opcode { 0x8B => FloatMath::abs(a), 0x8C => -a, 0x8D => FloatMath::ceil(a), 0x8E => FloatMath::floor(a), 0x8F => FloatMath::trunc(a), 0x90 => self.nearest_f32(a), 0x91 => FloatMath::sqrt(a), _ => 0.0 };
                self.stack.push(Value::F32(r));
            }
            0x92..=0x98 => {
                let b = self.pop_f32()?; let a = self.pop_f32()?;
                self.stack.push(Value::F32(match opcode { 
                    0x92 => a + b, 
                    0x93 => a - b, 
                    0x94 => a * b, 
                    0x95 => a / b, 
                    0x96 => self.min_f32(a, b), 
                    0x97 => self.max_f32(a, b), 
                    0x98 => self.copysign_f32(a, b), 
                    _ => 0.0 
                }));
            }
            0x99..=0x9F => {
                let a = self.pop_f64()?;
                let r = match opcode { 0x99 => FloatMath::abs(a), 0x9A => -a, 0x9B => FloatMath::ceil(a), 0x9C => FloatMath::floor(a), 0x9D => FloatMath::trunc(a), 0x9E => self.nearest_f64(a), 0x9F => FloatMath::sqrt(a), _ => 0.0 };
                self.stack.push(Value::F64(r));
            }
            0xA0..=0xA6 => {
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                self.stack.push(Value::F64(match opcode { 
                    0xA0 => a + b, 
                    0xA1 => a - b, 
                    0xA2 => a * b, 
                    0xA3 => a / b, 
                    0xA4 => self.min_f64(a, b), 
                    0xA5 => self.max_f64(a, b), 
                    0xA6 => self.copysign_f64(a, b), 
                    _ => 0.0 
                }));
            }
            _ => return Err("Invalid numeric opcode"),
        }
        Ok(())
    }
}
