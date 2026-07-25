use std::fmt;

use serde::{Deserialize, Serialize};

/// All data types supported by the neural engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DType {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float16,
    BFloat16,
    Float32,
    Float64,
    Complex64,
    Complex128,
}

impl DType {
    /// Size in bytes of a single element of this type.
    #[must_use]
    #[inline]
    pub const fn byte_size(self) -> usize {
        match self {
            Self::Bool | Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 | Self::Float16 | Self::BFloat16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float32 | Self::Complex64 => 4,
            Self::Int64 | Self::UInt64 | Self::Float64 | Self::Complex128 => 8,
        }
    }

    /// Returns true if this is a floating-point type.
    #[must_use]
    #[inline]
    pub const fn is_float(self) -> bool {
        matches!(
            self,
            Self::Float16 | Self::BFloat16 | Self::Float32 | Self::Float64
        )
    }

    /// Returns true if this is an integer type.
    #[must_use]
    #[inline]
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::UInt8
                | Self::UInt16
                | Self::UInt32
                | Self::UInt64
        )
    }

    /// Returns true if this is a complex type.
    #[must_use]
    #[inline]
    pub const fn is_complex(self) -> bool {
        matches!(self, Self::Complex64 | Self::Complex128)
    }

    /// Returns true if this is a signed type.
    #[must_use]
    #[inline]
    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Float16
                | Self::BFloat16
                | Self::Float32
                | Self::Float64
                | Self::Complex64
                | Self::Complex128
        )
    }

    /// Returns true if this is a numeric type (integer, float, or complex).
    #[must_use]
    #[inline]
    pub const fn is_numeric(self) -> bool {
        self.is_integer() || self.is_float() || self.is_complex()
    }

    /// Returns the human-readable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int8 => "int8",
            Self::Int16 => "int16",
            Self::Int32 => "int32",
            Self::Int64 => "int64",
            Self::UInt8 => "uint8",
            Self::UInt16 => "uint16",
            Self::UInt32 => "uint32",
            Self::UInt64 => "uint64",
            Self::Float16 => "float16",
            Self::BFloat16 => "bfloat16",
            Self::Float32 => "float32",
            Self::Float64 => "float64",
            Self::Complex64 => "complex64",
            Self::Complex128 => "complex128",
        }
    }

    /// Returns the highest-precision floating point type among a pair.
    #[must_use]
    pub fn promote_float(self, other: DType) -> DType {
        if self == other {
            return self;
        }
        match (self, other) {
            (DType::Float64, _) | (_, DType::Float64) => DType::Float64,
            (DType::Float32, _) | (_, DType::Float32) => DType::Float32,
            (DType::BFloat16, _) | (_, DType::BFloat16) => DType::BFloat16,
            (DType::Float16, _) | (_, DType::Float16) => DType::Float16,
            _ => DType::Float32,
        }
    }

    /// Returns the promoted type for two numeric dtypes.
    #[must_use]
    pub fn promote(self, other: DType) -> DType {
        if self == other {
            return self;
        }
        if self.is_complex() || other.is_complex() {
            return match (self, other) {
                (DType::Complex128, _) | (_, DType::Complex128) => DType::Complex128,
                (DType::Complex64, _) | (_, DType::Complex64) => DType::Complex64,
                (DType::Float64, _) | (_, DType::Float64) => DType::Complex128,
                (DType::Float32, _) | (_, DType::Float32) => DType::Complex64,
                _ => DType::Complex64,
            };
        }
        if self.is_float() || other.is_float() {
            return self.promote_float(other);
        }
        // Both are integers: promote to the larger type
        match (self.byte_size(), other.byte_size()) {
            (8, _) | (_, 8) => DType::Int64,
            (4, _) | (_, 4) => DType::Int32,
            (2, _) | (_, 2) => DType::Int16,
            _ => DType::Int8,
        }
    }

    /// Parses a dtype from its string name.
    #[must_use]
    pub fn from_name(s: &str) -> Option<DType> {
        match s {
            "bool" | "Bool" => Some(DType::Bool),
            "int8" | "Int8" | "i8" => Some(DType::Int8),
            "int16" | "Int16" | "i16" => Some(DType::Int16),
            "int32" | "Int32" | "i32" => Some(DType::Int32),
            "int64" | "Int64" | "i64" => Some(DType::Int64),
            "uint8" | "UInt8" | "u8" => Some(DType::UInt8),
            "uint16" | "UInt16" | "u16" => Some(DType::UInt16),
            "uint32" | "UInt32" | "u32" => Some(DType::UInt32),
            "uint64" | "UInt64" | "u64" => Some(DType::UInt64),
            "float16" | "Float16" | "f16" => Some(DType::Float16),
            "bfloat16" | "BFloat16" | "bf16" => Some(DType::BFloat16),
            "float32" | "Float32" | "f32" => Some(DType::Float32),
            "float64" | "Float64" | "f64" => Some(DType::Float64),
            "complex64" | "Complex64" | "c64" => Some(DType::Complex64),
            "complex128" | "Complex128" | "c128" => Some(DType::Complex128),
            _ => None,
        }
    }

    /// Returns zero bytes for this dtype.
    #[must_use]
    pub fn zero_bytes(self) -> Vec<u8> {
        vec![0u8; self.byte_size()]
    }

    /// Returns one bytes for this dtype.
    #[must_use]
    pub fn one_bytes(self) -> Vec<u8> {
        let mut buf = vec![0u8; self.byte_size()];
        match self {
            Self::Bool | Self::Int8 | Self::UInt8 => buf[0] = 1,
            Self::Int16 | Self::UInt16 => {
                let bytes = 1u16.to_le_bytes();
                buf[0] = bytes[0];
                buf[1] = bytes[1];
            }
            Self::BFloat16 => {
                let bytes = 0x3C00u16.to_le_bytes();
                buf[0] = bytes[0];
                buf[1] = bytes[1];
            }
            Self::Float16 => {
                let bytes = 0x3C00u16.to_le_bytes();
                buf[0] = bytes[0];
                buf[1] = bytes[1];
            }
            Self::Int32 | Self::UInt32 => {
                let bytes = 1u32.to_le_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    buf[i] = b;
                }
            }
            Self::Float32 | Self::Complex64 => {
                let bytes = 1f32.to_le_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    buf[i] = b;
                }
            }
            Self::Int64 | Self::UInt64 => {
                let bytes = 1u64.to_le_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    buf[i] = b;
                }
            }
            Self::Float64 | Self::Complex128 => {
                let bytes = 1f64.to_le_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    buf[i] = b;
                }
            }
        }
        buf
    }

    /// Returns a default fill value (zero) as f64.
    #[must_use]
    pub fn default_fill_f64(self) -> f64 {
        match self {
            Self::Bool => 0.0,
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => 0.0,
            Self::UInt8 | Self::UInt16 | Self::UInt32 | Self::UInt64 => 0.0,
            Self::Float16 | Self::BFloat16 | Self::Float32 | Self::Float64 => 0.0,
            Self::Complex64 | Self::Complex128 => 0.0,
        }
    }
}

impl fmt::Display for DType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Default for DType {
    fn default() -> Self {
        Self::Float32
    }
}

/// Safe element access functions for DType using byte-level operations.
pub mod access {
    use super::DType;

    /// Reads an f64 value from raw bytes.
    #[must_use]
    pub fn read_f64(data: &[u8], offset: usize) -> f64 {
        let mut bytes = [0u8; 8];
        let end = (offset + 8).min(data.len());
        let len = end - offset;
        bytes[..len].copy_from_slice(&data[offset..end]);
        f64::from_le_bytes(bytes)
    }

    /// Writes an f64 value to raw bytes.
    pub fn write_f64(data: &mut [u8], offset: usize, value: f64) {
        let bytes = value.to_le_bytes();
        let end = (offset + 8).min(data.len());
        let len = end - offset;
        data[offset..end].copy_from_slice(&bytes[..len]);
    }

    /// Reads an f32 value from raw bytes.
    #[must_use]
    pub fn read_f32(data: &[u8], offset: usize) -> f32 {
        let mut bytes = [0u8; 4];
        let end = (offset + 4).min(data.len());
        let len = end - offset;
        bytes[..len].copy_from_slice(&data[offset..end]);
        f32::from_le_bytes(bytes)
    }

    /// Writes an f32 value to raw bytes.
    pub fn write_f32(data: &mut [u8], offset: usize, value: f32) {
        let bytes = value.to_le_bytes();
        let end = (offset + 4).min(data.len());
        let len = end - offset;
        data[offset..end].copy_from_slice(&bytes[..len]);
    }

    /// Reads a value as f64 from raw bytes according to the given dtype.
    #[must_use]
    pub fn read_as_f64(data: &[u8], offset: usize, dtype: DType) -> f64 {
        match dtype {
            DType::Bool => u8::from_le_bytes([data[offset]]) as f64,
            DType::Int8 => i8::from_le_bytes([data[offset]]) as f64,
            DType::Int16 => {
                let mut b = [0u8; 2];
                b.copy_from_slice(&data[offset..offset + 2]);
                i16::from_le_bytes(b) as f64
            }
            DType::Int32 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(&data[offset..offset + 4]);
                i32::from_le_bytes(b) as f64
            }
            DType::Int64 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[offset..offset + 8]);
                i64::from_le_bytes(b) as f64
            }
            DType::UInt8 => u8::from_le_bytes([data[offset]]) as f64,
            DType::UInt16 => {
                let mut b = [0u8; 2];
                b.copy_from_slice(&data[offset..offset + 2]);
                u16::from_le_bytes(b) as f64
            }
            DType::UInt32 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(&data[offset..offset + 4]);
                u32::from_le_bytes(b) as f64
            }
            DType::UInt64 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[offset..offset + 8]);
                u64::from_le_bytes(b) as f64
            }
            DType::Float16 | DType::BFloat16 => {
                let mut b = [0u8; 2];
                b.copy_from_slice(&data[offset..offset + 2]);
                let bits = u16::from_le_bytes(b);
                f16_bits_to_f32(bits) as f64
            }
            DType::Float32 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(&data[offset..offset + 4]);
                f32::from_le_bytes(b) as f64
            }
            DType::Float64 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[offset..offset + 8]);
                f64::from_le_bytes(b)
            }
            DType::Complex64 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(&data[offset..offset + 4]);
                f32::from_le_bytes(b) as f64
            }
            DType::Complex128 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[offset..offset + 8]);
                f64::from_le_bytes(b)
            }
        }
    }

    /// Writes an f64 value to raw bytes according to the given dtype.
    pub fn write_f64_as(data: &mut [u8], offset: usize, dtype: DType, value: f64) {
        match dtype {
            DType::Bool => data[offset] = if value != 0.0 { 1 } else { 0 },
            DType::Int8 => data[offset] = (value as i8) as u8,
            DType::Int16 => {
                let bytes = (value as i16).to_le_bytes();
                data[offset..offset + 2].copy_from_slice(&bytes);
            }
            DType::Int32 => {
                let bytes = (value as i32).to_le_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
            }
            DType::Int64 => {
                let bytes = (value as i64).to_le_bytes();
                data[offset..offset + 8].copy_from_slice(&bytes);
            }
            DType::UInt8 => data[offset] = value as u8,
            DType::UInt16 => {
                let bytes = (value as u16).to_le_bytes();
                data[offset..offset + 2].copy_from_slice(&bytes);
            }
            DType::UInt32 => {
                let bytes = (value as u32).to_le_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
            }
            DType::UInt64 => {
                let bytes = (value as u64).to_le_bytes();
                data[offset..offset + 8].copy_from_slice(&bytes);
            }
            DType::Float16 | DType::BFloat16 => {
                let bits = f32_to_f16_bits(value as f32);
                let bytes = bits.to_le_bytes();
                data[offset..offset + 2].copy_from_slice(&bytes);
            }
            DType::Float32 => {
                let bytes = (value as f32).to_le_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
            }
            DType::Float64 => {
                let bytes = value.to_le_bytes();
                data[offset..offset + 8].copy_from_slice(&bytes);
            }
            DType::Complex64 => {
                let bytes = (value as f32).to_le_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
            }
            DType::Complex128 => {
                let bytes = value.to_le_bytes();
                data[offset..offset + 8].copy_from_slice(&bytes);
            }
        }
    }

    /// Converts f16 bits to f32 using software emulation.
    #[must_use]
    pub fn f16_bits_to_f32(bits: u16) -> f32 {
        let sign = (bits >> 15) & 1;
        let exponent = (bits >> 10) & 0x1F;
        let mantissa = bits & 0x3FF;

        if exponent == 0 {
            if mantissa == 0 {
                return if sign == 1 { -0.0 } else { 0.0 };
            }
            let mut value = mantissa as f32;
            let mut shift = 10;
            while (value as u32 & 0x400) == 0 {
                value *= 2.0;
                shift += 1;
            }
            value = (value - 1024.0) * 2f32.powi(-(shift as i32));
            return if sign == 1 { -value } else { value };
        }
        if exponent == 31 {
            return if mantissa == 0 {
                if sign == 1 {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                }
            } else {
                f32::NAN
            };
        }
        let value = f32::from_bits(((sign as u32) << 31) | (((exponent as u32) + 127 - 15) << 23) | ((mantissa as u32) << 13));
        value
    }

    /// Converts f32 to f16 bits using software emulation.
    #[must_use]
    pub fn f32_to_f16_bits(value: f32) -> u16 {
        let bits = value.to_bits();
        let sign = (bits >> 16) & 0x8000;
        let exponent = ((bits >> 23) & 0xFF) as i32;
        let mantissa = bits & 0x7F_FFFF;

        if exponent == 255 {
            return (sign | 0x7C00 | if mantissa != 0 { 0x200 } else { 0 }) as u16;
        }
        if exponent == 0 {
            return (sign | (mantissa >> 13)) as u16;
        }
        let new_exp = exponent - 127 + 15;
        if new_exp >= 31 {
            return (sign | 0x7C00) as u16;
        }
        if new_exp <= 0 {
            return (sign | ((mantissa | 0x80_0000) >> (1 - new_exp + 13))) as u16;
        }
        (sign | ((new_exp as u32) << 10) | (mantissa >> 13)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtype_sizes() {
        assert_eq!(DType::Bool.byte_size(), 1);
        assert_eq!(DType::Int8.byte_size(), 1);
        assert_eq!(DType::Int16.byte_size(), 2);
        assert_eq!(DType::Int32.byte_size(), 4);
        assert_eq!(DType::Int64.byte_size(), 8);
        assert_eq!(DType::UInt8.byte_size(), 1);
        assert_eq!(DType::UInt16.byte_size(), 2);
        assert_eq!(DType::UInt32.byte_size(), 4);
        assert_eq!(DType::UInt64.byte_size(), 8);
        assert_eq!(DType::Float16.byte_size(), 2);
        assert_eq!(DType::BFloat16.byte_size(), 2);
        assert_eq!(DType::Float32.byte_size(), 4);
        assert_eq!(DType::Float64.byte_size(), 8);
        assert_eq!(DType::Complex64.byte_size(), 4);
        assert_eq!(DType::Complex128.byte_size(), 8);
    }

    #[test]
    fn dtype_classification() {
        assert!(DType::Float32.is_float());
        assert!(!DType::Int32.is_float());
        assert!(DType::Int32.is_integer());
        assert!(!DType::Float32.is_integer());
        assert!(DType::Complex64.is_complex());
        assert!(!DType::Float32.is_complex());
        assert!(DType::Int32.is_signed());
        assert!(!DType::UInt32.is_signed());
        assert!(DType::Float32.is_numeric());
        assert!(!DType::Bool.is_numeric());
    }

    #[test]
    fn dtype_promotion() {
        assert_eq!(DType::Int8.promote(DType::Int32), DType::Int32);
        assert_eq!(DType::Float32.promote(DType::Float64), DType::Float64);
        assert_eq!(DType::Int32.promote(DType::Float32), DType::Float32);
        assert_eq!(DType::Complex64.promote(DType::Float64), DType::Complex128);
    }

    #[test]
    fn dtype_from_name() {
        assert_eq!(DType::from_name("float32"), Some(DType::Float32));
        assert_eq!(DType::from_name("f32"), Some(DType::Float32));
        assert_eq!(DType::from_name("int64"), Some(DType::Int64));
        assert_eq!(DType::from_name("bfloat16"), Some(DType::BFloat16));
        assert_eq!(DType::from_name("complex128"), Some(DType::Complex128));
        assert_eq!(DType::from_name("unknown"), None);
    }

    #[test]
    fn zero_and_one_bytes() {
        let z = DType::Float32.zero_bytes();
        assert_eq!(z.len(), 4);
        let o = DType::Float32.one_bytes();
        assert_eq!(o.len(), 4);
        let val = access::read_f32(&o, 0);
        assert_eq!(val, 1.0);
    }

    #[test]
    fn f16_roundtrip() {
        let values = [0.0f32, 1.0, -1.0, 0.5, 65504.0];
        for v in &values {
            let bits = access::f32_to_f16_bits(*v);
            let back = access::f16_bits_to_f32(bits);
            assert!(
                (back - v).abs() < 0.001 || (v.is_nan() && back.is_nan()),
                "f16 roundtrip failed for {}: got {}",
                v,
                back
            );
        }
    }

    #[test]
    fn read_write_as_f64() {
        let mut buf = vec![0u8; 8];
        access::write_f64_as(&mut buf, 0, DType::Float32, 42.0);
        let val = access::read_as_f64(&buf, 0, DType::Float32);
        assert_eq!(val, 42.0);

        access::write_f64_as(&mut buf, 0, DType::Int64, -100.0);
        let val = access::read_as_f64(&buf, 0, DType::Int64);
        assert_eq!(val, -100.0);

        access::write_f64_as(&mut buf, 0, DType::Bool, 1.0);
        let val = access::read_as_f64(&buf, 0, DType::Bool);
        assert_eq!(val, 1.0);
    }
}
