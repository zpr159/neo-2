use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::error::CliResult;

pub async fn run(system: &Arc<NeoSystem>, query: &str) -> CliResult<()> {
    println!("Reasoning: {query}");
    println!();

    match &system.reasoning {
        Some(reasoner) => {
            let request = neo_reasoning::ReasoningRequest::new(query.to_string());
            match reasoner.start_session(request.clone()).await {
                Ok(session_id) => {
                    match reasoner.execute_session(session_id, request).await {
                        Ok(response) => {
                            println!("Conclusion:   {}", response.conclusion);
                            println!("Confidence:   {:.3}", response.confidence);
                            println!("Strategy:     {}", response.strategy_used);
                            println!("Depth:        {}", response.reasoning_depth);
                            println!("Latency:      {}ms", response.latency_ms);
                            if let Some(ref explanation) = response.explanation {
                                println!();
                                println!("Explanation:");
                                println!("  {explanation}");
                            }
                        }
                        Err(e) => {
                            println!("Reasoning error: {e}");
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to start reasoning session: {e}");
                }
            }
        }
        None => {
            println!("Reasoning engine is not available.");
        }
    }
    Ok(())
}
