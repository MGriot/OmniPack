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
    FIFO = 1  # First-In, First-Out (Back to Front)
    LIFO = 2  # Last-In, First-Out (Front to Back)

class PackingVersus(IntEnum):
    LONGITUDINAL = 0 # (Z, Y, X) - Fill Depth then Height then Width
    LATERAL = 1      # (X, Y, Z) - Fill Width then Height then Depth (Side-to-Side)
    FLOOR_FIRST = 2  # (Y, Z, X) - Fill Floor completely before stacking

class ShapeType(IntEnum):
    BOX = 0
    SPHERE = 1
    CYLINDER = 2
    CONE = 3
    TETRAHEDRON = 4

@dataclass
class ShapePart:
    dx: float
    dy: float
    dz: float
    width: float
    height: float
    depth: float
    shape_type: ShapeType = ShapeType.BOX

    def to_dict(self):
        return {
            "offset": {"x": self.dx, "y": self.dy, "z": self.dz},
            "dimensions": {"w": self.width, "h": self.height, "d": self.depth},
            "shape_type": self.shape_type.name
        }

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
    parts: List[ShapePart] = field(default_factory=list)

    def __post_init__(self):
        if not self.parts:
            self.parts = [ShapePart(0, 0, 0, self.width, self.height, self.depth, self.shape_type)]

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

    def get_rotated_parts(self) -> List[Tuple[Tuple[float, float, float], Tuple[float, float, float], ShapeType]]:
        """
        Returns a list of (offset, dimensions, shape_type) for each part, correctly rotated.
        """
        rotated_parts = []
        for part in self.parts:
            pw, ph, pd = part.width, part.height, part.depth
            px, py, pz = part.dx, part.dy, part.dz
            st = part.shape_type
            
            if self.rotation == Rotation.W_H_D: 
                rx, ry, rz = px, py, pz
                rw, rh, rd = pw, ph, pd
            elif self.rotation == Rotation.H_W_D: 
                rx, ry, rz = py, px, pz
                rw, rh, rd = ph, pw, pd
            elif self.rotation == Rotation.H_D_W: 
                rx, ry, rz = pz, px, py
                rw, rh, rd = pd, pw, ph
            elif self.rotation == Rotation.D_H_W: 
                rx, ry, rz = pz, py, px
                rw, rh, rd = pd, ph, pw
            elif self.rotation == Rotation.D_W_H: 
                rx, ry, rz = py, pz, px
                rw, rh, rd = ph, pd, pw
            elif self.rotation == Rotation.W_D_H: 
                rx, ry, rz = px, pz, py
                rw, rh, rd = pw, pd, ph
            else:
                rx, ry, rz = px, py, pz
                rw, rh, rd = pw, ph, pd
                
            rotated_parts.append(((rx, ry, rz), (rw, rh, rd), st))
        return rotated_parts

    def volume(self) -> float:
        # Simplified volume: sum of bounding boxes of parts
        return sum(p.width * p.height * p.depth for p in self.parts)

    def to_dict(self):
        bw, bh, bd = self.get_dimension()
        rotated_parts_info = []
        for (offset, dim, st) in self.get_rotated_parts():
            rotated_parts_info.append({
                "offset": {"x": offset[0], "y": offset[1], "z": offset[2]},
                "dimensions": {"w": dim[0], "h": dim[1], "d": dim[2]},
                "shape_type": st.name
            })
            
        return {
            "id": self.id,
            "group_id": self.group_id,
            "stop_id": self.stop_id,
            "strategy": self.strategy.name,
            "position": {"x": self.position[0], "y": self.position[1], "z": self.position[2]},
            "dimensions": {"w": bw, "h": bh, "d": bd},
            "bounding_box": {"w": bw, "h": bh, "d": bd},
            "original_dim": {"w": self.width, "h": self.height, "d": self.depth},
            "parts": rotated_parts_info,
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
    shape_type: ShapeType = ShapeType.BOX
    parts: List[ShapePart] = field(default_factory=list)

    def __post_init__(self):
        # If no parts are defined, default to a single part matching the container dimensions
        if not self.parts:
            self.parts = [ShapePart(0, 0, 0, self.width, self.height, self.depth, self.shape_type)]

    def volume(self) -> float:
        return sum(p.width * p.height * p.depth for p in self.parts)

    def remaining_volume(self) -> float:
        used_volume = sum(item.volume() for item in self.items)
        return self.volume() - used_volume

    def calculate_accessibility(self) -> float:
        """
        Calculates the percentage of items that are accessible at their intended stop.
        An item for Stop N is blocked if an item for Stop M (where M >= N) is in front
        of it or on top of it. Items for Stop K (where K < N) are assumed to be
        already discharged and thus do not block.
        """
        if not self.items: return 100.0
        
        accessible_count = 0
        for i, item in enumerate(self.items):
            ix, iy, iz = item.position
            iw, ih, id_ = item.get_dimension()
            
            is_blocked = False
            for j, other in enumerate(self.items):
                if i == j: continue
                # Items for earlier stops don't block (they are gone)
                if other.stop_id < item.stop_id: continue
                
                ox, oy, oz = other.position
                ow, oh, od = other.get_dimension()
                
                # Check if 'other' is between 'item' and the door (higher Z)
                if oz >= iz + id_ - 0.001:
                    # XY overlap check
                    if (ix < ox + ow - 0.001 and ix + iw > ox + 0.001 and
                        iy < oy + oh - 0.001 and iy + ih > oy + 0.001):
                        is_blocked = True
                        break
                
                # Check if 'other' is on top of 'item' (higher Y)
                if oy >= iy + ih - 0.001:
                    # XZ overlap check
                    if (ix < ox + ow - 0.001 and ix + iw > ox + 0.001 and
                        iz < oz + od - 0.001 and iz + id_ > oz + 0.001):
                        is_blocked = True
                        break
            
            if not is_blocked:
                accessible_count += 1
                
        return (accessible_count / len(self.items)) * 100

    def to_dict(self):
        return {
            "id": self.id,
            "dimensions": {"w": self.width, "h": self.height, "d": self.depth},
            "parts": [p.to_dict() for p in self.parts],
            "items": [item.to_dict() for item in self.items],
            "utilization": (1.0 - (self.remaining_volume() / self.volume())) * 100 if self.volume() > 0 else 0,
            "accessibility": self.calculate_accessibility()
        }
