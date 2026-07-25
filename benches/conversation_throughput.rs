use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use neo_core::conversation::context_ranker::ContextRanker;
use neo_core::conversation::context_merger::ContextMerger;
use neo_core::conversation::prompt_builder::PromptBuilder;
use neo_core::conversation::response_validator::ResponseValidator;
use neo_core::conversation::evidence::{Evidence, EvidenceCollection, EvidenceSource};
use neo_core::conversation::types::*;
use neo_core::conversation::config::RankingConfig;
use neo_core::conversation::retrieval_coordinator::{CognitiveContext, UnifiedContextForSerialization};
use neo_core::language::types::{GenerationResponse, FinishReason, TokenUsage, ToolDefinition, FunctionDefinition};
use neo_core::observability::metrics::MetricsCollector;
use neo_core::time::Timestamp;
use std::collections::HashMap;
use std::time::Duration;

fn make_evidence(source: EvidenceSource, content: &str, confidence: f32) -> Evidence {
    Evidence::new(source, confidence, content, "benchmark")
        .with_relevance(confidence * 0.9)
}

fn make_ranking_config() -> RankingConfig {
    RankingConfig {
        semantic_weight: 0.3,
        recency_weight: 0.15,
        importance_weight: 0.15,
        confidence_weight: 0.15,
        source_reliability_weight: 0.1,
        user_relevance_weight: 0.05,
        task_relevance_weight: 0.1,
        max_items: 100,
    }
}

fn make_evidence_items(n: usize) -> Vec<Evidence> {
    let sources = [
        EvidenceSource::Memory,
        EvidenceSource::KnowledgeGraph,
        EvidenceSource::WorldModel,
        EvidenceSource::Reasoning,
        EvidenceSource::Planning,
        EvidenceSource::Executive,
        EvidenceSource::Agent,
        EvidenceSource::Tool,
        EvidenceSource::UserInput,
        EvidenceSource::ConversationHistory,
    ];
    (0..n)
        .map(|i| {
            let source = sources[i % sources.len()].clone();
            let confidence = 0.3 + (i as f32 * 0.05).min(0.7);
            make_evidence(
                source,
                &format!("Evidence item {} about context assembly and retrieval operations", i),
                confidence,
            )
        })
        .collect()
}

fn make_cognitive_context(evidence_count: usize) -> CognitiveContext {
    let evidence = make_evidence_items(evidence_count);
    let keywords = vec!["context".into(), "assembly".into(), "retrieval".into()];
    let ranker = ContextRanker::new(make_ranking_config());
    let ranked = ranker.rank(evidence, &keywords, "benchmark context assembly");

    CognitiveContext {
        ranked_evidence: ranked,
        unified: UnifiedContextForSerialization {
            evidence_count,
            average_confidence: 0.6,
            source_coverage: HashMap::new(),
            contradictions_detected: 0,
        },
        executive_context: None,
        planning_context: None,
        reasoning_context: None,
        memory_context: None,
        knowledge_context: None,
        world_model_context: None,
        agent_context: None,
        workflow_context: None,
        confidence: 0.75,
    }
}

fn make_conversation_context(message_count: usize) -> ConversationContext {
    let mut ctx = ConversationContext::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    ctx.intent = Some(Intent::Coding);
    ctx.urgency = Urgency::Normal;
    ctx.reasoning_depth = ReasoningDepth::Normal;
    for i in 0..message_count {
        if i % 2 == 0 {
            ctx.push_message(ConversationMessage::user(format!("User message {}", i)));
        } else {
            ctx.push_message(ConversationMessage::assistant(format!("Assistant response {}", i)));
        }
    }
    ctx
}

fn bench_context_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_ranking");
    for size in [5, 20, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let evidence = make_evidence_items(size);
            let keywords: Vec<String> = vec!["context".into(), "assembly".into(), "retrieval".into(), "benchmark".into()];
            let ranker = ContextRanker::new(make_ranking_config());
            b.iter(|| ranker.rank(evidence.clone(), &keywords, "benchmark context assembly"));
        });
    }
    group.finish();
}

fn bench_context_merging(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_merging");
    for size in [5, 20, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let evidence = make_evidence_items(size);
            let merger = ContextMerger::new();
            b.iter(|| merger.merge(evidence.clone()));
        });
    }
    group.finish();
}

fn bench_prompt_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("prompt_building");
    for msg_count in [5, 20, 50] {
        group.bench_with_input(BenchmarkId::from_parameter(msg_count), &msg_count, |b, &msg_count| {
            let conv_ctx = make_conversation_context(msg_count);
            let cog_ctx = make_cognitive_context(10);
            let tools = vec![
                ToolDefinition {
                    tool_type: "function".into(),
                    function: FunctionDefinition {
                        name: "search".into(),
                        description: "Search knowledge".into(),
                        parameters: serde_json::json!({"type": "object"}),
                    },
                },
            ];
            let builder = PromptBuilder::new();
            b.iter(|| builder.build(&conv_ctx, &cog_ctx, &tools, &[]));
        });
    }
    group.finish();
}

fn bench_response_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_validation");
    for text_len in [50, 500, 2000] {
        group.bench_with_input(BenchmarkId::from_parameter(text_len), &text_len, |b, &text_len| {
            let text = "This is a benchmark response with some context. ".repeat(text_len / 48 + 1);
            let text = &text[..text_len];
            let response = GenerationResponse {
                id: uuid::Uuid::new_v4(),
                text: text.to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage::default(),
                latency: Duration::from_millis(100),
                provider: "benchmark".into(),
                model: "test".into(),
                confidence: Some(0.8),
                warnings: vec![],
                tool_calls: None,
                metadata: HashMap::new(),
                created_at: Timestamp::now(),
            };
            let conv_ctx = make_conversation_context(5);
            let validator = ResponseValidator::new();
            b.iter(|| validator.validate(&response, &conv_ctx, Some(ResponseFormat::Markdown)));
        });
    }
    group.finish();
}

fn bench_metrics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_collection");

    group.bench_function("record_metrics", |b| {
        let collector = MetricsCollector::new();
        b.iter(|| {
            collector.record_conversation();
            collector.record_message_latency(42.5);
            collector.set_cpu_usage(75.0);
            collector.set_memory_usage(1024 * 1024);
            collector.record_tool_execution();
            collector.record_language_request(true);
            collector.record_first_token_latency(15.0);
            collector.record_memory_retrieval(8.0);
            collector.record_reasoning_latency(25.0);
            collector.record_workflow_completion(100.0);
            collector.record_task_completion();
        });
    });

    group.bench_function("collect_snapshot", |b| {
        let collector = MetricsCollector::new();
        for _ in 0..100 {
            collector.record_conversation();
            collector.record_message_latency(42.5);
        }
        b.iter(|| collector.collect("benchmark-node".into()));
    });

    group.finish();
}

fn bench_evidence_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("evidence_creation");

    group.bench_function("single_evidence", |b| {
        b.iter(|| {
            Evidence::new(EvidenceSource::Memory, 0.8, "test content for evidence", "benchmark")
                .with_relevance(0.7)
                .with_explanation("benchmark explanation")
                .with_reference("ref://source1")
        });
    });

    group.bench_function("evidence_collection_100", |b| {
        b.iter(|| {
            let mut collection = EvidenceCollection::new();
            for i in 0..100 {
                collection.push(make_evidence(
                    EvidenceSource::Memory,
                    &format!("Evidence item {}", i),
                    0.5 + (i as f32 * 0.005),
                ));
            }
            let _ = collection.average_confidence();
            let _ = collection.sorted_by_confidence();
            collection
        });
    });

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    group.bench_function("evidence_serialize_json", |b| {
        let evidence = make_evidence_items(10);
        b.iter(|| serde_json::to_vec(&evidence).unwrap());
    });

    group.bench_function("evidence_deserialize_json", |b| {
        let evidence = make_evidence_items(10);
        let json = serde_json::to_vec(&evidence).unwrap();
        b.iter(|| {
            let _: Vec<Evidence> = serde_json::from_slice(&json).unwrap();
        });
    });

    group.bench_function("metrics_serialize_json", |b| {
        let collector = MetricsCollector::new();
        for _ in 0..50 {
            collector.record_conversation();
            collector.record_message_latency(42.5);
        }
        let metrics = collector.collect("node1".into());
        b.iter(|| serde_json::to_vec(&metrics).unwrap());
    });

    group.bench_function("metrics_deserialize_json", |b| {
        let collector = MetricsCollector::new();
        for _ in 0..50 {
            collector.record_conversation();
            collector.record_message_latency(42.5);
        }
        let metrics = collector.collect("node1".into());
        let json = serde_json::to_vec(&metrics).unwrap();
        b.iter(|| {
            let _: neo_core::observability::metrics::AggregatedMetrics =
                serde_json::from_slice(&json).unwrap();
        });
    });

    group.bench_function("conversation_context_serialize", |b| {
        let ctx = make_conversation_context(20);
        b.iter(|| serde_json::to_vec(&ctx).unwrap());
    });

    group.bench_function("conversation_context_deserialize", |b| {
        let ctx = make_conversation_context(20);
        let json = serde_json::to_vec(&ctx).unwrap();
        b.iter(|| {
            let _: ConversationContext = serde_json::from_slice(&json).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_context_ranking,
    bench_context_merging,
    bench_prompt_building,
    bench_response_validation,
    bench_metrics_collection,
    bench_evidence_creation,
    bench_serialization,
);
criterion_main!(benches);
