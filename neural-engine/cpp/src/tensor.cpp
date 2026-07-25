#include <neo/neural/tensor.hpp>
#include <neo/core/error.hpp>
#include <algorithm>
#include <cstring>
#include <numeric>
#include <stdexcept>

namespace neo::neural {

const char* to_string(DType dtype) noexcept {
    switch (dtype) {
        case DType::Float16: return "Float16";
        case DType::Float32: return "Float32";
        case DType::Float64: return "Float64";
        case DType::Int8: return "Int8";
        case DType::Int16: return "Int16";
        case DType::Int32: return "Int32";
        case DType::Int64: return "Int64";
        case DType::Bool: return "Bool";
    }
    return "Unknown";
}

DType dtype_from_string(const std::string& str) {
    if (str == "Float16" || str == "float16" || str == "f16") return DType::Float16;
    if (str == "Float32" || str == "float32" || str == "f32") return DType::Float32;
    if (str == "Float64" || str == "float64" || str == "f64") return DType::Float64;
    if (str == "Int8" || str == "int8" || str == "i8") return DType::Int8;
    if (str == "Int16" || str == "int16" || str == "i16") return DType::Int16;
    if (str == "Int32" || str == "int32" || str == "i32") return DType::Int32;
    if (str == "Int64" || str == "int64" || str == "i64") return DType::Int64;
    if (str == "Bool" || str == "bool") return DType::Bool;
    throw std::invalid_argument("Unknown dtype: " + str);
}

std::size_t dtype_size(DType dtype) noexcept {
    switch (dtype) {
        case DType::Float16: return 2;
        case DType::Float32: return 4;
        case DType::Float64: return 8;
        case DType::Int8: return 1;
        case DType::Int16: return 2;
        case DType::Int32: return 4;
        case DType::Int64: return 8;
        case DType::Bool: return 1;
    }
    return 0;
}

Tensor::Tensor(Shape shape, DType dtype)
    : shape_(std::move(shape)), dtype_(dtype) {
    std::size_t total = 1;
    for (auto dim : shape_) {
        total *= dim;
    }
    size_ = total;
    data_ = std::make_unique<uint8_t[]>(size_ * dtype_size(dtype_));
    std::memset(data_.get(), 0, size_ * dtype_size(dtype_));
}

Tensor::Tensor(const Tensor& other)
    : shape_(other.shape_), dtype_(other.dtype_), size_(other.size_) {
    const std::size_t bytes = size_ * dtype_size(dtype_);
    data_ = std::make_unique<uint8_t[]>(bytes);
    std::memcpy(data_.get(), other.data_.get(), bytes);
}

Tensor& Tensor::operator=(const Tensor& other) {
    if (this == &other) return *this;
    shape_ = other.shape_;
    dtype_ = other.dtype_;
    size_ = other.size_;
    const std::size_t bytes = size_ * dtype_size(dtype_);
    data_ = std::make_unique<uint8_t[]>(bytes);
    std::memcpy(data_.get(), other.data_.get(), bytes);
    return *this;
}

Tensor::Tensor(Tensor&& other) noexcept
    : data_(std::move(other.data_)), shape_(std::move(other.shape_)),
      dtype_(other.dtype_), size_(other.size_) {
    other.size_ = 0;
}

Tensor& Tensor::operator=(Tensor&& other) noexcept {
    if (this == &other) return *this;
    data_ = std::move(other.data_);
    shape_ = std::move(other.shape_);
    dtype_ = other.dtype_;
    size_ = other.size_;
    other.size_ = 0;
    return *this;
}

Tensor Tensor::zeros(Shape shape, DType dtype) {
    Tensor t(std::move(shape), dtype);
    t.zero();
    return t;
}

Tensor Tensor::ones(Shape shape, DType dtype) {
    Tensor t(std::move(shape), dtype);
    t.fill(1.0f);
    return t;
}

Tensor Tensor::filled(Shape shape, DType dtype, float value) {
    Tensor t(std::move(shape), dtype);
    t.fill(value);
    return t;
}

const Shape& Tensor::shape() const noexcept {
    return shape_;
}

DType Tensor::dtype() const noexcept {
    return dtype_;
}

std::size_t Tensor::ndim() const noexcept {
    return shape_.size();
}

std::size_t Tensor::numel() const noexcept {
    return size_;
}

std::size_t Tensor::byte_size() const noexcept {
    return size_ * dtype_size(dtype_);
}

const uint8_t* Tensor::data() const noexcept {
    return data_.get();
}

uint8_t* Tensor::data_mut() noexcept {
    return data_.get();
}

Tensor Tensor::reshape(Shape new_shape) const {
    std::size_t new_size = 1;
    for (auto dim : new_shape) {
        new_size *= dim;
    }
    if (new_size != size_) {
        throw neo::core::Error(
            neo::core::NEO_ERR_GENERAL,
            "Cannot reshape tensor of size " + std::to_string(size_) +
            " to shape with " + std::to_string(new_size) + " elements",
            "Tensor::reshape"
        );
    }
    Tensor result(std::move(new_shape), dtype_);
    std::memcpy(result.data_.get(), data_.get(), byte_size());
    return result;
}

Tensor Tensor::transpose() const {
    if (shape_.size() < 2) {
        return *this;
    }
    Shape new_shape(shape_.rbegin(), shape_.rend());
    Tensor result(new_shape, dtype_);

    if (shape_.size() == 2) {
        for (std::size_t i = 0; i < shape_[0]; ++i) {
            for (std::size_t j = 0; j < shape_[1]; ++j) {
                std::vector<std::size_t> src_idx = {i, j};
                std::vector<std::size_t> dst_idx = {j, i};
                result.set_float(dst_idx, at_float(src_idx));
            }
        }
    } else {
        std::memcpy(result.data_.get(), data_.get(), byte_size());
    }
    return result;
}

Tensor Tensor::slice(std::size_t dim, std::size_t start, std::size_t end) const {
    if (dim >= shape_.size()) {
        throw neo::core::Error(
            neo::core::NEO_ERR_GENERAL,
            "Dimension out of range: " + std::to_string(dim),
            "Tensor::slice"
        );
    }
    if (end > shape_[dim] || start >= end) {
        throw neo::core::Error(
            neo::core::NEO_ERR_GENERAL,
            "Invalid slice range [" + std::to_string(start) + ", " + std::to_string(end) + ")",
            "Tensor::slice"
        );
    }

    Shape new_shape = shape_;
    new_shape[dim] = end - start;
    Tensor result(new_shape, dtype_);

    const std::size_t elem_size = dtype_size(dtype_);
    const std::size_t outer = size_ / (shape_[dim] * elem_size);
    const std::size_t inner = elem_size;

    std::size_t dst_offset = 0;
    for (std::size_t o = 0; o < outer; ++o) {
        std::size_t src_offset = (o * shape_[dim] + start) * inner;
        std::size_t copy_bytes = (end - start) * inner;
        std::memcpy(result.data_.get() + dst_offset, data_.get() + src_offset, copy_bytes);
        dst_offset += copy_bytes;
    }
    return result;
}

void Tensor::fill(float value) {
    const std::size_t es = dtype_size(dtype_);
    for (std::size_t i = 0; i < size_; ++i) {
        uint8_t* ptr = data_.get() + i * es;
        switch (dtype_) {
            case DType::Float32: {
                float v = value;
                std::memcpy(ptr, &v, sizeof(float));
                break;
            }
            case DType::Float64: {
                double v = static_cast<double>(value);
                std::memcpy(ptr, &v, sizeof(double));
                break;
            }
            case DType::Int32: {
                int32_t v = static_cast<int32_t>(value);
                std::memcpy(ptr, &v, sizeof(int32_t));
                break;
            }
            case DType::Int64: {
                int64_t v = static_cast<int64_t>(value);
                std::memcpy(ptr, &v, sizeof(int64_t));
                break;
            }
            case DType::Bool: {
                uint8_t v = value != 0.0f ? 1 : 0;
                std::memcpy(ptr, &v, sizeof(uint8_t));
                break;
            }
            default: {
                std::memset(ptr, 0, es);
                break;
            }
        }
    }
}

void Tensor::zero() {
    std::memset(data_.get(), 0, byte_size());
}

float Tensor::at_float(const std::vector<std::size_t>& indices) const {
    const std::size_t offset = compute_offset(indices);
    const std::size_t es = dtype_size(dtype_);
    const uint8_t* ptr = data_.get() + offset * es;

    switch (dtype_) {
        case DType::Float32: {
            float v;
            std::memcpy(&v, ptr, sizeof(float));
            return v;
        }
        case DType::Float64: {
            double v;
            std::memcpy(&v, ptr, sizeof(double));
            return static_cast<float>(v);
        }
        case DType::Int32: {
            int32_t v;
            std::memcpy(&v, ptr, sizeof(int32_t));
            return static_cast<float>(v);
        }
        case DType::Int64: {
            int64_t v;
            std::memcpy(&v, ptr, sizeof(int64_t));
            return static_cast<float>(v);
        }
        case DType::Bool: {
            return *ptr != 0 ? 1.0f : 0.0f;
        }
        default:
            return 0.0f;
    }
}

void Tensor::set_float(const std::vector<std::size_t>& indices, float value) {
    const std::size_t offset = compute_offset(indices);
    const std::size_t es = dtype_size(dtype_);
    uint8_t* ptr = data_.get() + offset * es;

    switch (dtype_) {
        case DType::Float32: {
            std::memcpy(ptr, &value, sizeof(float));
            break;
        }
        case DType::Float64: {
            double v = static_cast<double>(value);
            std::memcpy(ptr, &v, sizeof(double));
            break;
        }
        case DType::Int32: {
            int32_t v = static_cast<int32_t>(value);
            std::memcpy(ptr, &v, sizeof(int32_t));
            break;
        }
        case DType::Int64: {
            int64_t v = static_cast<int64_t>(value);
            std::memcpy(ptr, &v, sizeof(int64_t));
            break;
        }
        case DType::Bool: {
            uint8_t v = value != 0.0f ? 1 : 0;
            std::memcpy(ptr, &v, sizeof(uint8_t));
            break;
        }
        default:
            break;
    }
}

std::size_t Tensor::compute_offset(const std::vector<std::size_t>& indices) const {
    if (indices.size() != shape_.size()) {
        throw neo::core::Error(
            neo::core::NEO_ERR_GENERAL,
            "Index dimension mismatch: expected " + std::to_string(shape_.size()) +
            " but got " + std::to_string(indices.size()),
            "Tensor::compute_offset"
        );
    }
    std::size_t offset = 0;
    std::size_t stride = 1;
    for (int i = static_cast<int>(shape_.size()) - 1; i >= 0; --i) {
        offset += indices[i] * stride;
        stride *= shape_[i];
    }
    return offset;
}

} // namespace neo::neural
