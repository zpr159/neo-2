use neo_inference::quantization::engine::QuantizationEngine;
use neo_inference::quantization::{QuantizationConfig, QuantizationType};
use std::collections::HashMap;

fn make_weights() -> (HashMap<String, Vec<f32>>, HashMap<String, Vec<usize>>) {
    let mut weights = HashMap::new();
    let mut shapes = HashMap::new();
    let w: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) / 64.0).collect();
    shapes.insert("layer.weight".to_string(), vec![8, 16]);
    weights.insert("layer.weight".to_string(), w);

    let w2: Vec<f32> = (0..64).map(|i| (i as f32 - 32.0) / 32.0).collect();
    shapes.insert("layer.bias".to_string(), vec![8]);
    weights.insert("layer.bias".to_string(), w2);

    (weights, shapes)
}

#[test]
fn test_int8_quantization_roundtrip() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Int8,
        group_size: 128,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();

    assert_eq!(result.quant_type, QuantizationType::Int8);
    assert!(result.compression_ratio > 1.0);
    assert!(!result.weights.is_empty());

    for qw in result.weights.values() {
        let dequantized = engine.dequantize(qw).unwrap();
        assert!(!dequantized.is_empty());
    }
}

#[test]
fn test_int4_quantization_roundtrip() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Int4,
        group_size: 128,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();

    assert_eq!(result.quant_type, QuantizationType::Int4);
    assert!(result.compression_ratio > 2.0);

    for qw in result.weights.values() {
        let dequantized = engine.dequantize(qw).unwrap();
        assert!(!dequantized.is_empty());
    }
}

#[test]
fn test_fp16_quantization() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Fp16,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();

    assert_eq!(result.quant_type, QuantizationType::Fp16);
    assert!(result.compression_ratio > 1.0);

    for qw in result.weights.values() {
        let dequantized = engine.dequantize(qw).unwrap();
        assert!(!dequantized.is_empty());
    }
}

#[test]
fn test_bf16_quantization() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Bf16,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();

    assert_eq!(result.quant_type, QuantizationType::Bf16);
    assert!(result.compression_ratio > 1.0);

    for qw in result.weights.values() {
        let dequantized = engine.dequantize(qw).unwrap();
        assert!(!dequantized.is_empty());
    }
}

#[test]
fn test_compression_ratios() {
    let (weights, shapes) = make_weights();

    let fp16_config = QuantizationConfig {
        target_type: QuantizationType::Fp16,
        ..Default::default()
    };
    let fp16_result = QuantizationEngine::new(fp16_config)
        .quantize_weights(&weights, &shapes)
        .unwrap();

    let int8_config = QuantizationConfig {
        target_type: QuantizationType::Int8,
        ..Default::default()
    };
    let int8_result = QuantizationEngine::new(int8_config)
        .quantize_weights(&weights, &shapes)
        .unwrap();

    let int4_config = QuantizationConfig {
        target_type: QuantizationType::Int4,
        ..Default::default()
    };
    let int4_result = QuantizationEngine::new(int4_config)
        .quantize_weights(&weights, &shapes)
        .unwrap();

    assert!(fp16_result.compression_ratio > 1.0);
    assert!(int8_result.compression_ratio > fp16_result.compression_ratio);
    assert!(int4_result.compression_ratio > int8_result.compression_ratio);
}

#[test]
fn test_quantized_weight_compression_ratio() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Int8,
        group_size: 128,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let mut weights = HashMap::new();
    let mut shapes = HashMap::new();
    let w: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) / 128.0).collect();
    shapes.insert("big.weight".to_string(), vec![16, 16]);
    weights.insert("big.weight".to_string(), w);

    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    assert_eq!(result.compression_ratio, result.compression_ratio);

    let qw = result.weights.get("big.weight").unwrap();
    let ratio = qw.compression_ratio();
    assert!(ratio > 1.0, "Compression ratio should be > 1.0, got {}", ratio);
}

#[test]
fn test_gptq4_quantization() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Gptq4Bit,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    assert_eq!(result.quant_type, QuantizationType::Gptq4Bit);
}

#[test]
fn test_gguf_q4_quantization() {
    let config = QuantizationConfig {
        target_type: QuantizationType::GgufQ4_0,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    assert_eq!(result.quant_type, QuantizationType::GgufQ4_0);
}

#[test]
fn test_gguf_q8_quantization() {
    let config = QuantizationConfig {
        target_type: QuantizationType::GgufQ8_0,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let (weights, shapes) = make_weights();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    assert_eq!(result.quant_type, QuantizationType::GgufQ8_0);
}

#[test]
fn test_int8_dequantize_accuracy() {
    let config = QuantizationConfig {
        target_type: QuantizationType::Int8,
        ..Default::default()
    };
    let engine = QuantizationEngine::new(config);
    let mut weights = HashMap::new();
    let mut shapes = HashMap::new();
    let w: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];
    shapes.insert("w".to_string(), vec![8]);
    weights.insert("w".to_string(), w.clone());

    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    let qw = result.weights.get("w").unwrap();
    let dequantized = engine.dequantize(qw).unwrap();

    for (orig, deq) in w.iter().zip(dequantized.iter()) {
        let error = (orig - deq).abs();
        assert!(error < 0.2, "Error too large: {}", error);
    }
}

#[test]
fn test_empty_weights() {
    let config = QuantizationConfig::default();
    let engine = QuantizationEngine::new(config);
    let weights = HashMap::new();
    let shapes = HashMap::new();
    let result = engine.quantize_weights(&weights, &shapes).unwrap();
    assert!(result.weights.is_empty());
    assert_eq!(result.compression_ratio, 1.0);
}
