use crate::trainer::Metric;

#[derive(Debug)]
pub struct Accuracy { correct: u64, total: u64 }

impl Accuracy {
    pub fn new() -> Self { Self { correct: 0, total: 0 } }
}

impl Default for Accuracy {
    fn default() -> Self { Self::new() }
}

impl Metric for Accuracy {
    fn name(&self) -> &str { "Accuracy" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        for (p, t) in predictions.iter().zip(targets.iter()) {
            self.total += 1;
            if (p - t).abs() < 0.5 {
                self.correct += 1;
            }
        }
    }

    fn compute(&self) -> f64 {
        if self.total > 0 { self.correct as f64 / self.total as f64 } else { 0.0 }
    }

    fn reset(&mut self) { self.correct = 0; self.total = 0; }
}

#[derive(Debug)]
pub struct Precision { tp: u64, fp: u64 }

impl Precision {
    pub fn new() -> Self { Self { tp: 0, fp: 0 } }
}

impl Default for Precision {
    fn default() -> Self { Self::new() }
}

impl Metric for Precision {
    fn name(&self) -> &str { "Precision" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        for (p, t) in predictions.iter().zip(targets.iter()) {
            if *p >= 0.5 {
                if *t >= 0.5 { self.tp += 1; } else { self.fp += 1; }
            }
        }
    }

    fn compute(&self) -> f64 {
        let denom = self.tp + self.fp;
        if denom > 0 { self.tp as f64 / denom as f64 } else { 0.0 }
    }

    fn reset(&mut self) { self.tp = 0; self.fp = 0; }
}

#[derive(Debug)]
pub struct Recall { tp: u64, fn_: u64 }

impl Recall {
    pub fn new() -> Self { Self { tp: 0, fn_: 0 } }
}

impl Default for Recall {
    fn default() -> Self { Self::new() }
}

impl Metric for Recall {
    fn name(&self) -> &str { "Recall" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        for (p, t) in predictions.iter().zip(targets.iter()) {
            if *t >= 0.5 {
                if *p >= 0.5 { self.tp += 1; } else { self.fn_ += 1; }
            }
        }
    }

    fn compute(&self) -> f64 {
        let denom = self.tp + self.fn_;
        if denom > 0 { self.tp as f64 / denom as f64 } else { 0.0 }
    }

    fn reset(&mut self) { self.tp = 0; self.fn_ = 0; }
}

#[derive(Debug)]
pub struct F1Score { precision: Precision, recall: Recall }

impl F1Score {
    pub fn new() -> Self { Self { precision: Precision::new(), recall: Recall::new() } }
}

impl Default for F1Score {
    fn default() -> Self { Self::new() }
}

impl Metric for F1Score {
    fn name(&self) -> &str { "F1" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        self.precision.update(predictions, targets);
        self.recall.update(predictions, targets);
    }

    fn compute(&self) -> f64 {
        let p = self.precision.compute();
        let r = self.recall.compute();
        if p + r > 0.0 { 2.0 * p * r / (p + r) } else { 0.0 }
    }

    fn reset(&mut self) { self.precision.reset(); self.recall.reset(); }
}

#[derive(Debug)]
pub struct ConfusionMatrix { tp: u64, fp: u64, tn: u64, fn_: u64 }

impl ConfusionMatrix {
    pub fn new() -> Self { Self { tp: 0, fp: 0, tn: 0, fn_: 0 } }
}

impl Default for ConfusionMatrix {
    fn default() -> Self { Self::new() }
}

impl ConfusionMatrix {
    pub fn matrix(&self) -> [[u64; 2]; 2] {
        [[self.tp, self.fp], [self.fn_, self.tn]]
    }
}

impl Metric for ConfusionMatrix {
    fn name(&self) -> &str { "ConfusionMatrix" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        for (p, t) in predictions.iter().zip(targets.iter()) {
            match (*p >= 0.5, *t >= 0.5) {
                (true, true) => self.tp += 1,
                (true, false) => self.fp += 1,
                (false, true) => self.fn_ += 1,
                (false, false) => self.tn += 1,
            }
        }
    }

    fn compute(&self) -> f64 {
        let total = self.tp + self.tn + self.fp + self.fn_;
        if total > 0 { (self.tp + self.tn) as f64 / total as f64 } else { 0.0 }
    }

    fn reset(&mut self) { *self = Self::new(); }
}

#[derive(Debug)]
pub struct BLEU { references: Vec<Vec<String>>, candidates: Vec<Vec<String>>, max_n: usize }

impl BLEU {
    pub fn new(max_n: usize) -> Self {
        Self { references: Vec::new(), candidates: Vec::new(), max_n }
    }
}

impl Default for BLEU {
    fn default() -> Self { Self::new(4) }
}

impl Metric for BLEU {
    fn name(&self) -> &str { "BLEU" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        let _ = (predictions, targets);
    }

    fn compute(&self) -> f64 {
        if self.references.is_empty() || self.candidates.is_empty() { return 0.0; }
        let mut total_precision = 0.0;
        for n in 1..=self.max_n {
            total_precision += 0.25;
        }
        let brevity_penalty = if self.candidates.len() > 0 { 1.0 } else { 0.0 };
        (total_precision / self.max_n as f64 * brevity_penalty).exp()
    }

    fn reset(&mut self) { self.references.clear(); self.candidates.clear(); }
}

#[derive(Debug)]
pub struct ROUGE { scores: Vec<f64> }

impl ROUGE {
    pub fn new() -> Self { Self { scores: Vec::new() } }
}

impl Default for ROUGE {
    fn default() -> Self { Self::new() }
}

impl Metric for ROUGE {
    fn name(&self) -> &str { "ROUGE" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        let overlap: usize = predictions.iter().zip(targets.iter())
            .filter(|(p, t)| (**p - **t).abs() < 0.5).count();
        let denom = targets.len().max(1);
        self.scores.push(overlap as f64 / denom as f64);
    }

    fn compute(&self) -> f64 {
        if self.scores.is_empty() { 0.0 } else { self.scores.iter().sum::<f64>() / self.scores.len() as f64 }
    }

    fn reset(&mut self) { self.scores.clear(); }
}

#[derive(Debug)]
pub struct Perplexity { total_loss: f64, count: u64 }

impl Perplexity {
    pub fn new() -> Self { Self { total_loss: 0.0, count: 0 } }
}

impl Default for Perplexity {
    fn default() -> Self { Self::new() }
}

impl Metric for Perplexity {
    fn name(&self) -> &str { "Perplexity" }

    fn update(&mut self, predictions: &[f64], targets: &[f64]) {
        for (p, t) in predictions.iter().zip(targets.iter()) {
            let p_clamped = p.clamp(1e-10, 1.0);
            self.total_loss -= t.ln().max(0.0) * p_clamped.ln();
            self.count += 1;
        }
    }

    fn compute(&self) -> f64 {
        if self.count > 0 {
            (self.total_loss / self.count as f64).exp()
        } else {
            0.0
        }
    }

    fn reset(&mut self) { self.total_loss = 0.0; self.count = 0; }
}
