use neo_inference::generation::engine::GenerationEngine;

#[test]
fn test_greedy_decode() {
    let logits = vec![0.1, 0.5, 0.3, 0.05, 0.05];
    let id = GenerationEngine::greedy_decode(&logits, 5);
    assert_eq!(id, 1);
}

#[test]
fn test_greedy_decode_single_token() {
    let logits = vec![0.1, 0.9];
    let id = GenerationEngine::greedy_decode(&logits, 2);
    assert_eq!(id, 1);
}

#[test]
fn test_greedy_decode_empty_logits() {
    let logits = vec![];
    let id = GenerationEngine::greedy_decode(&logits, 5);
    assert_eq!(id, 0);
}

#[test]
fn test_top_k_sample() {
    let mut logits = vec![0.1, 0.9, 0.5, 0.01, 0.01];
    let id = GenerationEngine::top_k_sample(&mut logits, 2);
    assert!(id == 1 || id == 2);
}

#[test]
fn test_top_k_sample_k_larger_than_vocab() {
    let mut logits = vec![0.1, 0.9, 0.5];
    let id = GenerationEngine::top_k_sample(&mut logits, 100);
    assert!(id < 3);
}

#[test]
fn test_top_p_sample() {
    let mut logits = vec![0.1, 0.9, 0.5, 0.01];
    let id = GenerationEngine::top_p_sample(&mut logits, 0.9);
    assert!(id < 4);
}

#[test]
fn test_top_p_sample_low_threshold() {
    let mut logits = vec![0.1, 0.9, 0.5, 0.01];
    let id = GenerationEngine::top_p_sample(&mut logits, 0.5);
    assert!(id < 4);
}

#[test]
fn test_apply_temperature() {
    let mut logits = vec![2.0, 4.0, 6.0];
    GenerationEngine::apply_temperature(&mut logits, 2.0);
    assert!((logits[0] - 1.0).abs() < 0.001);
    assert!((logits[1] - 2.0).abs() < 0.001);
    assert!((logits[2] - 3.0).abs() < 0.001);
}

#[test]
fn test_apply_temperature_noop() {
    let mut logits = vec![1.0, 2.0, 3.0];
    let original = logits.clone();
    GenerationEngine::apply_temperature(&mut logits, 1.0);
    assert_eq!(logits, original);
}

#[test]
fn test_apply_temperature_zero() {
    let mut logits = vec![1.0, 2.0, 3.0];
    let original = logits.clone();
    GenerationEngine::apply_temperature(&mut logits, 0.0);
    assert_eq!(logits, original);
}

#[test]
fn test_apply_repetition_penalty() {
    let mut logits = vec![1.0, 2.0, 3.0, 4.0];
    let generated = vec![1, 3];
    GenerationEngine::apply_repetition_penalty(&mut logits, &generated, 2.0);
    assert!((logits[1] - 1.0).abs() < 0.001);
    assert!((logits[3] - 2.0).abs() < 0.001);
    assert!((logits[0] - 1.0).abs() < 0.001);
    assert!((logits[2] - 3.0).abs() < 0.001);
}

#[test]
fn test_apply_repetition_penalty_noop() {
    let mut logits = vec![1.0, 2.0, 3.0];
    let original = logits.clone();
    GenerationEngine::apply_repetition_penalty(&mut logits, &[0, 1], 1.0);
    assert_eq!(logits, original);
}

#[test]
fn test_check_stop_sequences() {
    let stop = vec!["STOP".to_string(), "END".to_string()];
    assert!(GenerationEngine::check_stop_sequences("hello STOP world", &stop));
    assert!(GenerationEngine::check_stop_sequences("the END", &stop));
    assert!(!GenerationEngine::check_stop_sequences("hello world", &stop));
}

#[test]
fn test_check_stop_sequences_empty() {
    let stop: Vec<String> = vec![];
    assert!(!GenerationEngine::check_stop_sequences("hello", &stop));
}

#[test]
fn test_check_stop_token() {
    let stops = vec![0, 3, 100];
    assert!(GenerationEngine::check_stop_token(3, &stops));
    assert!(GenerationEngine::check_stop_token(100, &stops));
    assert!(!GenerationEngine::check_stop_token(5, &stops));
}

#[test]
fn test_check_stop_token_empty() {
    let stops: Vec<u32> = vec![];
    assert!(!GenerationEngine::check_stop_token(5, &stops));
}

#[test]
fn test_apply_presence_penalty() {
    let mut logits = vec![1.0, 2.0, 3.0, 4.0];
    let generated = vec![1, 1, 3];
    GenerationEngine::apply_presence_penalty(&mut logits, &generated, 0.5);
    assert!((logits[0] - 1.0).abs() < 0.001);
    assert!((logits[1] - 1.5).abs() < 0.001);
    assert!((logits[2] - 3.0).abs() < 0.001);
    assert!((logits[3] - 3.5).abs() < 0.001);
}

#[test]
fn test_apply_frequency_penalty() {
    let mut logits = vec![1.0, 2.0, 3.0];
    let generated = vec![1, 1, 1, 2];
    GenerationEngine::apply_frequency_penalty(&mut logits, &generated, 0.1);
    assert!((logits[0] - 1.0).abs() < 0.001);
    assert!((logits[1] - 1.7).abs() < 0.001);
    assert!((logits[2] - 2.9).abs() < 0.001);
}

#[test]
fn test_beam_search_decode() {
    let logits = vec![0.1, 0.9, 0.5];
    let mut beams: Vec<(Vec<u32>, f32)> = Vec::new();
    GenerationEngine::beam_search_decode(&logits, 3, 2, &mut beams);
    assert_eq!(beams.len(), 2);
    assert_eq!(beams[0].0.len(), 1);
}
