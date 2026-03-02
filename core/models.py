from dataclasses import dataclass, field
from typing import List, Tuple, Optional
from enum import IntEnum

class Rotation(IntEnum):
    W_H_D = 0 # (w, h, d)
    H_W_D = 1 # (h, w, d)
    H_D_W = 2 # (h, d, w)
    D_H_W = 3 # (d, h, w)
    D_W_H = 4 # (d, w, h)
    W_D_H = 5 # (w, d, h)

@dataclass
class Item:
    id: str
    width: float
    height: float
    depth: float
    weight: float = 0.0
    max_stack_weight: float = 1000000.0 
    rotation: Rotation = Rotation.W_H_D
    position: Tuple[float, float, float] = (0.0, 0.0, 0.0)
    group_id: Optional[str] = None # NEW: Persist grouping metadata

    def get_dimension(self) -> Tuple[float, float, float]:
        """Returns the dimensions based on current rotation."""
        w, h, d = self.width, self.height, self.depth
        if self.rotation == Rotation.W_H_D: return (w, h, d)
        if self.rotation == Rotation.H_W_D: return (h, w, d)
        if self.rotation == Rotation.H_D_W: return (h, d, w)
        if self.rotation == Rotation.D_H_W: return (d, h, w)
        if self.rotation == Rotation.D_W_H: return (d, w, h)
        if self.rotation == Rotation.W_D_H: return (w, d, h)
        return (w, h, d)

    def volume(self) -> float:
        return self.width * self.height * self.depth

    def to_dict(self):
        w, h, d = self.get_dimension()
        return {
            "id": self.id,
            "group_id": self.group_id,
            "position": {"x": self.position[0], "y": self.position[1], "z": self.position[2]},
            "dimensions": {"w": w, "h": h, "d": d},
            "original_dim": {"w": self.width, "h": self.height, "d": self.depth},
            "rotation": self.rotation.name,
            "weight": self.weight,
            "max_stack_weight": self.max_stack_weight
        }

@dataclass
class Container:
    id: str
    width: float
    height: float
    depth: float
    max_weight: float = 0.0
    items: List[Item] = field(default_factory=list)

    def volume(self) -> float:
        return self.width * self.height * self.depth

    def remaining_volume(self) -> float:
        used_volume = sum(item.volume() for item in self.items)
        return self.volume() - used_volume

    def to_dict(self):
        return {
            "id": self.id,
            "dimensions": {"w": self.width, "h": self.height, "d": self.depth},
            "items": [item.to_dict() for item in self.items],
            "utilization": (1.0 - (self.remaining_volume() / self.volume())) * 100
        }
