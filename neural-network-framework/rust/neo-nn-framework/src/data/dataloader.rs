use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::data::dataset::Dataset;
use crate::data::sampler::{SequentialSampler, RandomSampler, Sampler};

pub struct DataLoader<D: Dataset> {
    dataset: D,
    batch_size: usize,
    shuffle: bool,
    indices: Vec<usize>,
    current: usize,
}

impl<D: Dataset> DataLoader<D> {
    pub fn new(dataset: D, batch_size: usize, shuffle: bool) -> Self {
        let len = dataset.len();
        let indices = if shuffle {
            RandomSampler::new(len).indices()
        } else {
            SequentialSampler::new(len).indices()
        };
        Self { dataset, batch_size, shuffle, indices, current: 0 }
    }

    pub fn iter(&mut self) -> DataLoaderIter<'_, D> {
        self.current = 0;
        if self.shuffle {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            self.indices.shuffle(&mut rng);
        }
        DataLoaderIter { loader: self }
    }

    pub fn len(&self) -> usize {
        (self.dataset.len() + self.batch_size - 1) / self.batch_size
    }

    pub fn is_empty(&self) -> bool {
        self.dataset.is_empty()
    }
}

pub struct DataLoaderIter<'a, D: Dataset> {
    loader: &'a mut DataLoader<D>,
}

impl<'a, D: Dataset> Iterator for DataLoaderIter<'a, D> {
    type Item = NnResult<(ADTensor, ADTensor)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.loader.current >= self.loader.indices.len() {
            return None;
        }

        let batch_start = self.loader.current;
        let batch_end = (batch_start + self.loader.batch_size).min(self.loader.indices.len());

        if batch_start >= self.loader.indices.len() {
            return None;
        }

        let mut data_batch = Vec::new();
        let mut target_batch = Vec::new();

        for &idx in &self.loader.indices[batch_start..batch_end] {
            match self.loader.dataset.get(idx) {
                Ok((data, target)) => {
                    data_batch.push(data);
                    target_batch.push(target);
                }
                Err(e) => return Some(Err(e)),
            }
        }

        self.loader.current = batch_end;

        match concat_tensors(&data_batch) {
            Ok(data) => match concat_tensors(&target_batch) {
                Ok(target) => Some(Ok((data, target))),
                Err(e) => Some(Err(e)),
            },
            Err(e) => Some(Err(e)),
        }
    }
}

fn concat_tensors(tensors: &[ADTensor]) -> NnResult<ADTensor> {
    if tensors.is_empty() {
        return Err(crate::error::NnError::InvalidInput("Cannot concat empty list".to_string()));
    }

    let ndim = tensors[0].ndim();
    let shape = tensors[0].shape().dims();
    let mut all_data = Vec::new();

    for t in tensors {
        all_data.extend(t.to_vec_f64()?);
    }

    let batch_size = tensors.len();
    let mut new_dims = vec![batch_size];
    new_dims.extend_from_slice(shape);

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&all_data, neo_neural_engine::shape::Shape::new(new_dims)),
        false,
    ))
}
