"""Neo Plugins — plugin definitions."""
from __future__ import annotations

import uuid
from enum import Enum


class PluginState(Enum):
    Registered = "registered"
    Loaded = "loaded"
    Active = "active"
    Error = "error"
    Unloaded = "unloaded"


class PluginMetadata:
    """Metadata describing a plugin."""

    def __init__(self, name: str, version: str, author: str = "", description: str = "") -> None:
        self.name = name
        self.version = version
        self.author = author
        self.description = description


class Plugin:
    """A loadable, activatable plugin with lifecycle management."""

    def __init__(self, metadata: PluginMetadata) -> None:
        self._id: str = str(uuid.uuid4())
        self._metadata = metadata
        self._state: PluginState = PluginState.Registered

    @property
    def id(self) -> str:
        return self._id

    @property
    def state(self) -> PluginState:
        return self._state

    @property
    def name(self) -> str:
        return self._metadata.name

    def load(self) -> None:
        if self._state == PluginState.Registered:
            self._state = PluginState.Loaded
        elif self._state == PluginState.Unloaded:
            self._state = PluginState.Loaded
        else:
            self._state = PluginState.Error

    def activate(self) -> None:
        if self._state == PluginState.Loaded:
            self._state = PluginState.Active
        else:
            self._state = PluginState.Error

    def deactivate(self) -> None:
        if self._state == PluginState.Active:
            self._state = PluginState.Loaded
        else:
            self._state = PluginState.Error

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "name": self._metadata.name,
            "version": self._metadata.version,
            "author": self._metadata.author,
            "description": self._metadata.description,
            "state": self._state.value,
        }
