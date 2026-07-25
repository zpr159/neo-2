"""Neo Neural Network Layers — layer abstractions and implementations."""
from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class Layer(ABC):
    """Abstract base class for all neural network layers."""

    def __init__(self, name: str | None = None) -> None:
        self.name = name or self.__class__.__name__

    @abstractmethod
    def forward(self, x: Any) -> Any:
        """Compute the forward pass."""

    @abstractmethod
    def backward(self, grad: Any) -> Any:
        """Compute the backward pass."""

    @abstractmethod
    def parameters_count(self) -> int:
        """Return the number of trainable parameters."""

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(name={self.name!r})"


class Dense(Layer):
    """Fully connected layer."""

    def __init__(self, in_features: int, out_features: int, name: str | None = None) -> None:
        super().__init__(name)
        self.in_features = in_features
        self.out_features = out_features

    def forward(self, x: Any) -> Any:
        return x

    def backward(self, grad: Any) -> Any:
        return grad

    def parameters_count(self) -> int:
        return self.in_features * self.out_features + self.out_features

    def __repr__(self) -> str:
        return f"Dense({self.in_features} -> {self.out_features})"


class Conv2d(Layer):
    """2D convolution layer."""

    def __init__(
        self,
        in_channels: int,
        out_channels: int,
        kernel_size: int,
        name: str | None = None,
    ) -> None:
        super().__init__(name)
        self.in_channels = in_channels
        self.out_channels = out_channels
        self.kernel_size = kernel_size

    def forward(self, x: Any) -> Any:
        return x

    def backward(self, grad: Any) -> Any:
        return grad

    def parameters_count(self) -> int:
        return self.in_channels * self.out_channels * self.kernel_size * self.kernel_size + self.out_channels

    def __repr__(self) -> str:
        return f"Conv2d({self.in_channels} -> {self.out_channels}, kernel={self.kernel_size})"


class ReLU(Layer):
    """Rectified Linear Unit activation."""

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name)

    def forward(self, x: Any) -> Any:
        return x

    def backward(self, grad: Any) -> Any:
        return grad

    def parameters_count(self) -> int:
        return 0


class Softmax(Layer):
    """Softmax activation."""

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name)

    def forward(self, x: Any) -> Any:
        return x

    def backward(self, grad: Any) -> Any:
        return grad

    def parameters_count(self) -> int:
        return 0


class LayerNorm(Layer):
    """Layer normalization."""

    def __init__(self, normalized_shape: int, name: str | None = None) -> None:
        super().__init__(name)
        self.normalized_shape = normalized_shape

    def forward(self, x: Any) -> Any:
        return x

    def backward(self, grad: Any) -> Any:
        return grad

    def parameters_count(self) -> int:
        return self.normalized_shape * 2

    def __repr__(self) -> str:
        return f"LayerNorm(shape={self.normalized_shape})"
