use std::collections::HashMap;
use crate::error::{InferenceError, InferenceResult};
use crate::quantization::{QuantizationConfig, QuantizationType, QuantizedWeight, QuantizationResult, QuantizationMetrics};
use crate::model::ModelMetadata;

pub struct QuantizationEngine {
    config: QuantizationConfig,
}

impl QuantizationEngine {
    pub fn new(config: QuantizationConfig) -> Self {
        Self { config }
    }

    pub fn quantize_weights(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        match self.config.target_type {
            QuantizationType::Fp16 => self.quantize_fp16(weights, shapes),
            QuantizationType::Bf16 => self.quantize_bf16(weights, shapes),
            QuantizationType::Int8 => self.quantize_int8(weights, shapes),
            QuantizationType::Int4 => self.quantize_int4(weights, shapes),
            QuantizationType::Gptq4Bit => self.quantize_gptq(weights, shapes, 4),
            QuantizationType::Awq4Bit => self.quantize_awq(weights, shapes),
            QuantizationType::GgufQ4_0 => self.quantize_gguf_q4_0(weights, shapes),
            QuantizationType::GgufQ4_1 => self.quantize_gguf_q4_1(weights, shapes),
            QuantizationType::GgufQ8_0 => self.quantize_gguf_q8_0(weights, shapes),
            _ => self.quantize_int8(weights, shapes),
        }
    }

    fn quantize_fp16(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        for (name, weight) in weights {
            let fp16_data: Vec<u16> = weight.iter().map(|&v| {
                let bits = v.to_bits();
                let sign = ((bits >> 16) & 0x8000) as u16;
                let exponent = ((bits >> 23) & 0xFF) as i32 - 127 + 15;
                let mantissa = ((bits >> 13) & 0x3FF) as u16;
                if exponent <= 0 { sign } else if exponent >= 31 { sign | 0x7C00 | mantissa } else { sign | ((exponent as u16) << 10) | mantissa }
            }).collect();
            let mut data = Vec::with_capacity(fp16_data.len() * 2);
            for &v in &fp16_data {
                data.extend_from_slice(&v.to_le_bytes());
            }
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type: QuantizationType::Fp16,
                data,
                scale: vec![1.0],
                zero_point: None,
                group_size: 0,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Fp16)
    }

    fn quantize_bf16(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        for (name, weight) in weights {
            let bf16_data: Vec<u16> = weight.iter().map(|&v| {
                let bits = v.to_bits();
                ((bits >> 16) & 0xFFFF) as u16
            }).collect();
            let mut data = Vec::with_capacity(bf16_data.len() * 2);
            for &v in &bf16_data {
                data.extend_from_slice(&v.to_le_bytes());
            }
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type: QuantizationType::Bf16,
                data,
                scale: vec![1.0],
                zero_point: None,
                group_size: 0,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Bf16)
    }

    fn quantize_int8(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        let group_size = self.config.group_size;
        for (name, weight) in weights {
            let abs_max = weight.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
            let scale = abs_max / 127.0;
            let int8_data: Vec<i8> = weight.iter().map(|&v| {
                (v / scale).round().max(-128.0).min(127.0) as i8
            }).collect();
            let data: Vec<u8> = int8_data.iter().map(|&v| v as u8).collect();
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type: QuantizationType::Int8,
                data,
                scale: vec![scale],
                zero_point: None,
                group_size,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Int8)
    }

    fn quantize_int4(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        for (name, weight) in weights {
            let abs_max = weight.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
            let scale = abs_max / 7.0;
            let int4_data: Vec<u8> = weight.chunks(2).map(|pair| {
                let v0 = (pair[0] / scale).round().max(-8.0).min(7.0) as i8;
                let v1 = pair.get(1).map(|&v| (v / scale).round().max(-8.0).min(7.0) as i8).unwrap_or(0);
                ((v0 as u8) & 0x0F) | (((v1 as u8) & 0x0F) << 4)
            }).collect();
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type: QuantizationType::Int4,
                data: int4_data,
                scale: vec![scale],
                zero_point: None,
                group_size: self.config.group_size,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Int4)
    }

    fn quantize_gptq(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
        bits: u32,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        for (name, weight) in weights {
            let abs_max = weight.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
            let q_max = (1u32 << (bits - 1)) as f32 - 1.0;
            let scale = abs_max / q_max;
            let q_data: Vec<u8> = weight.iter().map(|&v| {
                ((v / scale).round().max(-q_max).min(q_max) as i32 + q_max as i32) as u8
            }).collect();
            let quant_type = match bits {
                3 => QuantizationType::Gptq3Bit,
                _ => QuantizationType::Gptq4Bit,
            };
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type,
                data: q_data,
                scale: vec![scale],
                zero_point: Some(vec![q_max as i32]),
                group_size: self.config.group_size,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Gptq4Bit)
    }

    fn quantize_awq(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        for (name, weight) in weights {
            let mean = weight.iter().sum::<f32>() / weight.len() as f32;
            let abs_mean: f32 = weight.iter().map(|v| (v - mean).abs()).sum::<f32>() / weight.len() as f32;
            let scale = abs_mean / 7.0;
            let quantized: Vec<u8> = weight.iter().map(|&v| {
                ((v / scale).round().max(-8.0).min(7.0) as i8 + 8) as u8
            }).collect();
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type: QuantizationType::Awq4Bit,
                data: quantized,
                scale: vec![scale],
                zero_point: Some(vec![8]),
                group_size: self.config.group_size,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, QuantizationType::Awq4Bit)
    }

    fn quantize_gguf_q4_0(&self, weights: &HashMap<String, Vec<f32>>, shapes: &HashMap<String, Vec<usize>>) -> InferenceResult<QuantizationResult> {
        self.quantize_gguf(weights, shapes, QuantizationType::GgufQ4_0, 4, false)
    }

    fn quantize_gguf_q4_1(&self, weights: &HashMap<String, Vec<f32>>, shapes: &HashMap<String, Vec<usize>>) -> InferenceResult<QuantizationResult> {
        self.quantize_gguf(weights, shapes, QuantizationType::GgufQ4_1, 4, true)
    }

    fn quantize_gguf_q8_0(&self, weights: &HashMap<String, Vec<f32>>, shapes: &HashMap<String, Vec<usize>>) -> InferenceResult<QuantizationResult> {
        self.quantize_gguf(weights, shapes, QuantizationType::GgufQ8_0, 8, false)
    }

    fn quantize_gguf(
        &self,
        weights: &HashMap<String, Vec<f32>>,
        shapes: &HashMap<String, Vec<usize>>,
        quant_type: QuantizationType,
        bits: u32,
        has_offset: bool,
    ) -> InferenceResult<QuantizationResult> {
        let mut quantized_weights = HashMap::new();
        let block_size = 32;
        for (name, weight) in weights {
            let mut data = Vec::new();
            let mut scales = Vec::new();
            for block in weight.chunks(block_size) {
                let abs_max = block.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
                let q_max = ((1u32 << (bits - 1)) as f32) - 1.0;
                let scale = abs_max / q_max;
                scales.push(scale);
                for &v in block {
                    let q = (v / scale).round().max(-q_max).min(q_max) as i8;
                    data.push((q as u8) & 0xFF);
                }
                if has_offset {
                    let mean: f32 = block.iter().sum::<f32>() / block.len() as f32;
                    data.extend_from_slice(&mean.to_le_bytes());
                }
            }
            quantized_weights.insert(name.clone(), QuantizedWeight {
                shape: shapes.get(name).cloned().unwrap_or_default(),
                quant_type,
                data,
                scale: scales,
                zero_point: None,
                group_size: block_size,
                original_dtype: "fp32".to_string(),
            });
        }
        self.build_result(quantized_weights, quant_type)
    }

    fn build_result(
        &self,
        weights: HashMap<String, QuantizedWeight>,
        quant_type: QuantizationType,
    ) -> InferenceResult<QuantizationResult> {
        let original_size: u64 = weights.values()
            .map(|w| w.shape.iter().product::<usize>() as u64 * 4)
            .sum();
        let quantized_size: u64 = weights.values().map(|w| w.data.len() as u64).sum();
        let compression_ratio = if quantized_size > 0 { original_size as f64 / quantized_size as f64 } else { 1.0 };
        Ok(QuantizationResult {
            quant_type,
            weights,
            compression_ratio,
            original_size,
            quantized_size,
            calibration_metrics: None,
        })
    }

    pub fn dequantize(&self, weight: &QuantizedWeight) -> InferenceResult<Vec<f32>> {
        match weight.quant_type {
            QuantizationType::Fp16 => {
                let floats: Vec<f32> = weight.data.chunks(2).map(|chunk| {
                    if chunk.len() < 2 { return 0.0; }
                    let h = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let sign = ((h >> 15) & 1) as u32;
                    let exponent = ((h >> 10) & 0x1F) as i32 - 15;
                    let mantissa = (h & 0x3FF) as u32;
                    let val = if exponent == -15 && mantissa == 0 {
                        0.0
                    } else {
                        let f = (1.0 + mantissa as f64 / 1024.0) * 2.0f64.powi(exponent);
                        if sign == 1 { -f } else { f }
                    };
                    val as f32
                }).collect();
                Ok(floats)
            }
            QuantizationType::Bf16 => {
                let floats: Vec<f32> = weight.data.chunks(2).map(|chunk| {
                    if chunk.len() < 2 { return 0.0; }
                    let h = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let bits = (h as u32) << 16;
                    f32::from_bits(bits)
                }).collect();
                Ok(floats)
            }
            QuantizationType::Int8 => {
                let scale = weight.scale.first().copied().unwrap_or(1.0);
                Ok(weight.data.iter().map(|&v| (v as i8 as f32) * scale).collect())
            }
            QuantizationType::Int4 => {
                let scale = weight.scale.first().copied().unwrap_or(1.0);
                let mut floats = Vec::new();
                for &packed in &weight.data {
                    let lo = (packed & 0x0F) as i8;
                    let hi = ((packed >> 4) & 0x0F) as i8;
                    floats.push((lo as f32 - 8.0) * scale);
                    floats.push((hi as f32 - 8.0) * scale);
                }
                Ok(floats)
            }
            _ => {
                let scale = weight.scale.first().copied().unwrap_or(1.0);
                Ok(weight.data.iter().map(|&v| (v as f32) * scale).collect())
            }
        }
    }
}
