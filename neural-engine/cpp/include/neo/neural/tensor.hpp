#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>

namespace neo::neural {

enum class DType : std::uint8_t {
    Float16 = 0,
    Float32 = 1,
    Float64 = 2,
    Int8 = 3,
    Int16 = 4,
    Int32 = 5,
    Int64 = 6,
    Bool = 7
};

[[nodiscard]] const char* to_string(DType dtype) noexcept;
[[nodiscard]] DType dtype_from_string(const std::string& str);
[[nodiscard]] std::size_t dtype_size(DType dtype) noexcept;

using Shape = std::vector<std::size_t>;

class Tensor {
public:
    Tensor(Shape shape, DType dtype);
    ~Tensor() = default;

    Tensor(const Tensor& other);
    Tensor& operator=(const Tensor& other);
    Tensor(Tensor&& other) noexcept;
    Tensor& operator=(Tensor&& other) noexcept;

    static Tensor zeros(Shape shape, DType dtype);
    static Tensor ones(Shape shape, DType dtype);
    static Tensor filled(Shape shape, DType dtype, float value);

    [[nodiscard]] const Shape& shape() const noexcept;
    [[nodiscard]] DType dtype() const noexcept;
    [[nodiscard]] std::size_t ndim() const noexcept;
    [[nodiscard]] std::size_t numel() const noexcept;
    [[nodiscard]] std::size_t byte_size() const noexcept;

    [[nodiscard]] const uint8_t* data() const noexcept;
    [[nodiscard]] uint8_t* data_mut() noexcept;

    [[nodiscard]] Tensor reshape(Shape new_shape) const;
    [[nodiscard]] Tensor transpose() const;
    [[nodiscard]] Tensor slice(std::size_t dim, std::size_t start, std::size_t end) const;

    void fill(float value);
    void zero();

    [[nodiscard]] float at_float(const std::vector<std::size_t>& indices) const;
    void set_float(const std::vector<std::size_t>& indices, float value);

private:
    std::unique_ptr<uint8_t[]> data_;
    Shape shape_;
    DType dtype_;
    std::size_t size_{0};

    [[nodiscard]] std::size_t compute_offset(const std::vector<std::size_t>& indices) const;
};

} // namespace neo::neural
