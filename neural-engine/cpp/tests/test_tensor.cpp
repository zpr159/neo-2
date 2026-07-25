#include <gtest/gtest.h>
#include <neo/neural/tensor.hpp>
#include <neo/neural/device.hpp>

using namespace neo::neural;

TEST(TensorTest, DefaultConstruction) {
    Tensor t({2, 3}, DType::Float32);
    EXPECT_EQ(t.ndim(), 2u);
    EXPECT_EQ(t.numel(), 6u);
    EXPECT_EQ(t.dtype(), DType::Float32);
    EXPECT_EQ(t.shape(), (Shape{2, 3}));
}

TEST(TensorTest, ByteSize) {
    Tensor t({4, 4}, DType::Float32);
    EXPECT_EQ(t.byte_size(), 64u);

    Tensor t2({4, 4}, DType::Float64);
    EXPECT_EQ(t2.byte_size(), 128u);

    Tensor t3({8}, DType::Int8);
    EXPECT_EQ(t3.byte_size(), 8u);
}

TEST(TensorTest, Zeros) {
    Tensor t = Tensor::zeros({3, 3}, DType::Float32);
    EXPECT_FLOAT_EQ(t.at_float({0, 0}), 0.0f);
    EXPECT_FLOAT_EQ(t.at_float({2, 2}), 0.0f);
}

TEST(TensorTest, FillAndAccess) {
    Tensor t = Tensor::zeros({2, 3}, DType::Float32);
    t.set_float({0, 0}, 1.0f);
    t.set_float({1, 2}, 5.5f);
    EXPECT_FLOAT_EQ(t.at_float({0, 0}), 1.0f);
    EXPECT_FLOAT_EQ(t.at_float({1, 2}), 5.5f);
    EXPECT_FLOAT_EQ(t.at_float({0, 1}), 0.0f);
}

TEST(TensorTest, Reshape) {
    Tensor t = Tensor::zeros({2, 6}, DType::Float32);
    t.set_float({0, 0}, 1.0f);
    t.set_float({1, 5}, 9.0f);

    Tensor reshaped = t.reshape({3, 4});
    EXPECT_EQ(reshaped.shape(), (Shape{3, 4}));
    EXPECT_EQ(reshaped.numel(), 6u);
}

TEST(TensorTest, ReshapeInvalid) {
    Tensor t = Tensor::zeros({2, 3}, DType::Float32);
    EXPECT_THROW(t.reshape({2, 4}), neo::core::Error);
}

TEST(TensorTest, Transpose) {
    Tensor t({2, 3}, DType::Float32);
    t.set_float({0, 0}, 1.0f);
    t.set_float({0, 1}, 2.0f);
    t.set_float({1, 0}, 3.0f);

    Tensor transposed = t.transpose();
    EXPECT_EQ(transposed.shape(), (Shape{3, 2}));
    EXPECT_FLOAT_EQ(transposed.at_float({0, 0}), 1.0f);
    EXPECT_FLOAT_EQ(transposed.at_float({1, 0}), 2.0f);
    EXPECT_FLOAT_EQ(transposed.at_float({0, 1}), 3.0f);
}

TEST(TensorTest, CopySemantics) {
    Tensor original({2, 2}, DType::Float32);
    original.set_float({0, 0}, 42.0f);

    Tensor copy = original;
    EXPECT_FLOAT_EQ(copy.at_float({0, 0}), 42.0f);
    copy.set_float({0, 0}, 0.0f);
    EXPECT_FLOAT_EQ(original.at_float({0, 0}), 42.0f);
}

TEST(TensorTest, MoveSemantics) {
    Tensor original({2, 2}, DType::Float32);
    original.set_float({1, 1}, 7.0f);

    Tensor moved = std::move(original);
    EXPECT_FLOAT_EQ(moved.at_float({1, 1}), 7.0f);
}

TEST(TensorTest, Ones) {
    Tensor t = Tensor::ones({3}, DType::Float32);
    EXPECT_FLOAT_EQ(t.at_float({0}), 1.0f);
    EXPECT_FLOAT_EQ(t.at_float({2}), 1.0f);
}

TEST(DeviceTest, DefaultCPU) {
    Device dev = Device::cpu();
    EXPECT_EQ(dev.type, DeviceType::CPU);
    EXPECT_EQ(dev.name, "cpu");
    EXPECT_TRUE(dev.is_available());
    EXPECT_FALSE(dev.is_gpu());
}

TEST(DeviceTest, DetectAll) {
    auto devices = Device::detect_all();
    EXPECT_GE(devices.size(), 1u);
    EXPECT_EQ(devices[0].type, DeviceType::CPU);
}

TEST(DeviceTest, DeviceTypeString) {
    EXPECT_STREQ(to_string(DeviceType::CPU), "CPU");
    EXPECT_STREQ(to_string(DeviceType::CUDA), "CUDA");
    EXPECT_STREQ(to_string(DeviceType::Metal), "Metal");
    EXPECT_STREQ(to_string(DeviceType::Vulkan), "Vulkan");
}

TEST(DeviceTest, MemoryUsage) {
    Device dev;
    dev.memory_total = 1000;
    dev.memory_available = 750;
    EXPECT_FLOAT_EQ(dev.memory_usage_percent(), 25.0f);
}

TEST(DTypeTest, DtypeSize) {
    EXPECT_EQ(dtype_size(DType::Float16), 2u);
    EXPECT_EQ(dtype_size(DType::Float32), 4u);
    EXPECT_EQ(dtype_size(DType::Float64), 8u);
    EXPECT_EQ(dtype_size(DType::Int8), 1u);
    EXPECT_EQ(dtype_size(DType::Bool), 1u);
}

TEST(DTypeTest, DtypeString) {
    EXPECT_STREQ(to_string(DType::Float32), "Float32");
    EXPECT_STREQ(to_string(DType::Int64), "Int64");
}
