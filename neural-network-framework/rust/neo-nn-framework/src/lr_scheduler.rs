#[derive(Debug, Clone)]
pub enum SchedulerType {
    Constant,
    Step { step_size: u32, gamma: f64 },
    Cosine { total_steps: u32 },
    CosineWarmRestart { total_steps: u32, warm_restart: u32 },
    Linear { total_steps: u32 },
    Polynomial { total_steps: u32, power: f64 },
    OneCycle { total_steps: u32, max_lr: f64 },
    Warmup { warmup_steps: u32, base_lr: f64 },
}

#[derive(Debug)]
pub struct LRScheduler {
    scheduler_type: SchedulerType,
    base_lr: f64,
    current_lr: f64,
    step_count: u32,
}

impl LRScheduler {
    pub fn new(scheduler_type: SchedulerType, base_lr: f64) -> Self {
        Self { scheduler_type, base_lr, current_lr: base_lr, step_count: 0 }
    }

    pub fn step(&mut self) -> f64 {
        self.step_count += 1;
        self.current_lr = match &self.scheduler_type {
            SchedulerType::Constant => self.base_lr,
            SchedulerType::Step { step_size, gamma } => {
                let steps = self.step_count / step_size;
                self.base_lr * gamma.powf(steps as f64)
            }
            SchedulerType::Cosine { total_steps } => {
                let t = self.step_count.min(*total_steps) as f64;
                let total = *total_steps as f64;
                self.base_lr * 0.5 * (1.0 + (std::f64::consts::PI * t / total).cos())
            }
            SchedulerType::CosineWarmRestart { total_steps, warm_restart } => {
                let cycle = self.step_count % warm_restart;
                let t = cycle as f64;
                let total = *warm_restart as f64;
                self.base_lr * 0.5 * (1.0 + (std::f64::consts::PI * t / total).cos())
            }
            SchedulerType::Linear { total_steps } => {
                let t = self.step_count.min(*total_steps) as f64;
                let total = *total_steps as f64;
                self.base_lr * (1.0 - t / total)
            }
            SchedulerType::Polynomial { total_steps, power } => {
                let t = self.step_count.min(*total_steps) as f64;
                let total = *total_steps as f64;
                self.base_lr * (1.0 - t / total).powf(*power)
            }
            SchedulerType::OneCycle { total_steps, max_lr } => {
                let t = self.step_count.min(*total_steps) as f64;
                let total = *total_steps as f64;
                if t < total / 2.0 {
                    self.base_lr + (max_lr - self.base_lr) * (2.0 * t / total)
                } else {
                    max_lr - (max_lr - self.base_lr) * (2.0 * (t - total / 2.0) / total)
                }
            }
            SchedulerType::Warmup { warmup_steps, base_lr } => {
                if self.step_count <= *warmup_steps {
                    base_lr * self.step_count as f64 / *warmup_steps as f64
                } else {
                    *base_lr
                }
            }
        };
        self.current_lr
    }

    pub fn get_lr(&self) -> f64 {
        self.current_lr
    }

    pub fn reset(&mut self) {
        self.step_count = 0;
        self.current_lr = self.base_lr;
    }
}
