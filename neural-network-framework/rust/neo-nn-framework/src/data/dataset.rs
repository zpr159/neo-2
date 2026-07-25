use crate::autograd::ADTensor;
use crate::error::NnResult;

pub trait Dataset {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn get(&self, index: usize) -> NnResult<(ADTensor, ADTensor)>;
}

pub struct TensorDataset {
    data: Vec<ADTensor>,
    targets: Vec<ADTensor>,
}

impl TensorDataset {
    pub fn new(data: Vec<ADTensor>, targets: Vec<ADTensor>) -> NnResult<Self> {
        if data.len() != targets.len() {
            return Err(crate::error::NnError::InvalidInput(
                "Data and targets must have the same length".to_string(),
            ));
        }
        Ok(Self { data, targets })
    }
}

impl Dataset for TensorDataset {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn get(&self, index: usize) -> NnResult<(ADTensor, ADTensor)> {
        if index >= self.data.len() {
            return Err(crate::error::NnError::InvalidInput(format!(
                "Index {} out of bounds for dataset of length {}",
                index,
                self.data.len()
            )));
        }
        Ok((self.data[index].clone(), self.targets[index].clone()))
    }
}

pub struct IterableDataset<I> {
    iter: I,
}

impl<I> IterableDataset<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}
