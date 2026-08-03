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
    FIFO = 1 # First In First Out (from back to front)
    LIFO = 2 # Last In First Out (from front to back)

class PackingVersus(IntEnum):
    LONGITUDINAL = 0 # Fill depth first
    LATERAL = 1      # Fill width first
    FLOOR_FIRST = 2  # Fill surface area first
    WALL_BUILDING = 3 
    CORNER_FIRST = 4

class ShapeType(IntEnum):
    BOX = 0
    SPHERE = 1
    CYLINDER = 2
    TETRAHEDRON = 3

@dataclass
class Item:
    id: str
    width: float
    height: float
    depth: float
    weight: float = 0.0
    max_stack_weight: float = 1000000.0
    group_id: Optional[str] = None
    strategy: LoadingStrategy = LoadingStrategy.NONE
    stop_id: int = 0
    allowed_rotations: List[Rotation] = field(default_factory=lambda: [r for r in Rotation])
    shape_type: ShapeType = ShapeType.BOX
    
    # Internal state
    position: Tuple[float, float, float] = (0, 0, 0)
    rotation: Rotation = Rotation.W_H_D

    def volume(self) -> float:
        return self.width * self.height * self.depth

    def get_dimension(self) -> Tuple[float, float, float]:
        w, h, d = self.width, self.height, self.depth
        if self.rotation == Rotation.W_H_D: return (w, h, d)
        if self.rotation == Rotation.H_W_D: return (h, w, d)
        if self.rotation == Rotation.H_D_W: return (h, d, w)
        if self.rotation == Rotation.D_H_W: return (d, h, w)
        if self.rotation == Rotation.D_W_H: return (d, w, h)
        if self.rotation == Rotation.W_D_H: return (w, d, h)
        return (w, h, d)

@dataclass
class Container:
    id: str
    width: float
    height: float
    depth: float
    max_weight: float = 1000000.0
    shape_type: ShapeType = ShapeType.BOX
    items: List[Item] = field(default_factory=list)
    metadata: dict = field(default_factory=dict)

    def volume(self) -> float:
        return self.width * self.height * self.depth

    def remaining_volume(self) -> float:
        return self.volume() - sum(item.volume() for item in self.items)

    def volume_utilization(self) -> float:
        if not self.items: return 0.0
        used = sum(it.volume() for it in self.items)
        return (used / self.volume()) * 100

    def calculate_accessibility(self) -> float:
        """
        Stop-aware accessibility: an item is blocked if another item sitting
        closer to the door (higher Z) needs a LATER stop (higher stop_id) and
        overlaps it in X-Y - i.e. it must be moved before this item can come out.
        Score is the percentage of items that are NOT blocked.
        """
        if not self.items: return 100.0

        blocked = set()
        sorted_by_z = sorted(self.items, key=lambda x: x.position[2], reverse=True)

        for i, item in enumerate(sorted_by_z):
            for other in sorted_by_z[i+1:]:
                if other.stop_id < item.stop_id:
                    iw, ih, _ = item.get_dimension()
                    ow, oh, _ = other.get_dimension()
                    if (item.position[0] < other.position[0] + ow and item.position[0] + iw > other.position[0] and
                        item.position[1] < other.position[1] + oh and item.position[1] + ih > other.position[1]):
                        blocked.add(id(other))

        return ((len(self.items) - len(blocked)) / len(self.items)) * 100.0

    def calculate_stats(self):
        used_vol = sum(it.volume() for it in self.items)
        total_weight = sum(it.weight for it in self.items)
        
        # Center of Mass calculation
        com_x, com_y, com_z = 0.0, 0.0, 0.0
        if total_weight > 0:
            for it in self.items:
                w, h, d = it.get_dimension()
                com_x += (it.position[0] + w/2) * it.weight
                com_y += (it.position[1] + h/2) * it.weight
                com_z += (it.position[2] + d/2) * it.weight
            com_x /= total_weight
            com_y /= total_weight
            com_z /= total_weight

        return {
            "volume_utilization": (used_vol / self.volume()) * 100 if self.volume() > 0 else 0,
            "weight_utilization": (total_weight / self.max_weight) * 100 if self.max_weight > 0 else 0,
            "total_weight": total_weight,
            "center_of_mass": {"x": com_x, "y": com_y, "z": com_z},
            "stability_score": 100.0, # Placeholder
            "item_count": len(self.items)
        }

    def to_dict(self):
        stats = self.calculate_stats()
        return {
            "id": self.id,
            "dimensions": {"w": self.width, "h": self.height, "d": self.depth},
            "items": [
                {
                    "id": it.id,
                    "position": {"x": it.position[0], "y": it.position[1], "z": it.position[2]},
                    "dimensions": {
                        "w": it.get_dimension()[0],
                        "h": it.get_dimension()[1],
                        "d": it.get_dimension()[2]
                    },
                    "rotation": int(it.rotation),
                    "shape_type": it.shape_type.name
                } for it in self.items
            ],
            "utilization": stats["volume_utilization"],
            "accessibility": self.calculate_accessibility(),
            "shape_type": self.shape_type.name,
            "stats": stats,
            "metadata": self.metadata
        }
