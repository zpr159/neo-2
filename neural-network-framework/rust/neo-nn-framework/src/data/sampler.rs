use rand::seq::SliceRandom;

pub trait Sampler {
    fn indices(&self) -> Vec<usize>;
}

pub struct SequentialSampler {
    len: usize,
}

impl SequentialSampler {
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl Sampler for SequentialSampler {
    fn indices(&self) -> Vec<usize> {
        (0..self.len).collect()
    }
}

pub struct RandomSampler {
    len: usize,
}

impl RandomSampler {
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl Sampler for RandomSampler {
    fn indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.len).collect();
        let mut rng = rand::thread_rng();
        indices.shuffle(&mut rng);
        indices
    }
}

pub struct DistributedSampler {
    len: usize,
    rank: usize,
    num_replicas: usize,
}

impl DistributedSampler {
    pub fn new(len: usize, rank: usize, num_replicas: usize) -> Self {
        Self { len, rank, num_replicas }
    }
}

impl Sampler for DistributedSampler {
    fn indices(&self) -> Vec<usize> {
        let mut all: Vec<usize> = (0..self.len).collect();
        let mut rng = rand::thread_rng();
        all.shuffle(&mut rng);
        all.into_iter().enumerate()
            .filter(|(i, _)| i % self.num_replicas == self.rank)
            .map(|(_, idx)| idx)
            .collect()
    }
}
