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

class LoadingStrategy(IntEnum):
    NONE = 0
    FIFO = 1
    LIFO = 2

class ShapeType(IntEnum):
    BOX = 0
    SPHERE = 1
    CYLINDER = 2
    TETRAHEDRON = 3

class PackingVersus(IntEnum):
    LONGITUDINAL = 0 # (Z, Y, X) - Default
    LATERAL = 1      # (X, Y, Z)
    FLOOR_FIRST = 2  # (Y, Z, X)

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
    group_id: Optional[str] = None 
    strategy: LoadingStrategy = LoadingStrategy.NONE 
    stop_id: int = 0 
    allowed_rotations: List[Rotation] = field(default_factory=lambda: [r for r in Rotation])
    shape_type: ShapeType = ShapeType.BOX

    def get_dimension(self) -> Tuple[float, float, float]:
        """Returns the bounding box dimensions based on current rotation."""
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
            "dimensions": {"w": w, "h": h, "d": d},
            "position": {"x": self.position[0], "y": self.position[1], "z": self.position[2]},
            "weight": self.weight,
            "max_stack_weight": self.max_stack_weight,
            "rotation": int(self.rotation),
            "strategy": int(self.strategy),
            "stop_id": self.stop_id,
            "shape_type": self.shape_type.name
        }

@dataclass
class Container:
    id: str
    width: float
    height: float
    depth: float
    max_weight: float = 0.0
    items: List[Item] = field(default_factory=list)
    shape_type: ShapeType = ShapeType.BOX

    def volume(self) -> float:
        return self.width * self.height * self.depth

    def remaining_volume(self) -> float:
        return self.volume() - sum(item.volume() for item in self.items)

    def calculate_accessibility(self) -> float:
        """
        Calculates stop-aware accessibility.
        Higher score means items for early stops are less blocked by items for late stops.
        """
        if not self.items: return 100.0
        
        penalty = 0.0
        sorted_by_z = sorted(self.items, key=lambda x: x.position[2], reverse=True) # Door is at Z=Depth
        
        for i, item in enumerate(sorted_by_z):
            # Check if this item blocks any item with a LOWER stop_id (earlier discharge)
            for other in sorted_by_z[i+1:]:
                if other.stop_id < item.stop_id:
                    # Check X-Y overlap
                    iw, ih, _ = item.get_dimension()
                    ow, oh, _ = other.get_dimension()
                    if (item.position[0] < other.position[0] + ow and item.position[0] + iw > other.position[0] and
                        item.position[1] < other.position[1] + oh and item.position[1] + ih > other.position[1]):
                        penalty += 1.0
        
        return max(0.0, 100.0 - (penalty / len(self.items)) * 10.0)

    def to_dict(self):
        return {
            "id": self.id,
            "dimensions": {"w": self.width, "h": self.height, "d": self.depth},
            "items": [item.to_dict() for item in self.items],
            "utilization": (1.0 - (self.remaining_volume() / self.volume())) * 100 if self.volume() > 0 else 0,
            "accessibility": self.calculate_accessibility(),
            "shape_type": self.shape_type.name
        }
