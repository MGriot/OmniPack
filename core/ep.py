from typing import List, Tuple, Set
from .models import Item, Container

class ExtremePoint:
    def __init__(self, x: float, y: float, z: float):
        self.x = x
        self.y = y
        self.z = z

    def to_tuple(self) -> Tuple[float, float, float]:
        return (self.x, self.y, self.z)

    def __eq__(self, other):
        if not isinstance(other, ExtremePoint): return False
        return abs(self.x - other.x) < 0.001 and abs(self.y - other.y) < 0.001 and abs(self.z - other.z) < 0.001

    def __hash__(self):
        return hash((round(self.x, 3), round(self.y, 3), round(self.z, 3)))

def generate_extreme_points(container: Container, item: Item) -> List[ExtremePoint]:
    """
    Advanced Extreme Points generation.
    When an item is placed, it creates new potential corners by projecting 
    its boundaries until they hit another item or the container wall.
    """
    w, h, d = item.get_dimension()
    x, y, z = item.position
    
    # Standard corners
    raw_pts = [
        ExtremePoint(x + w, y, z),
        ExtremePoint(x, y + h, z),
        ExtremePoint(x, y, z + d)
    ]
    
    # In a full EP implementation, we would project these points to all items.
    # For Level 2, we ensure that points are correctly constrained by the container.
    valid_pts = []
    for p in raw_pts:
        if (p.x < container.width and 
            p.y < container.height and 
            p.z < container.depth):
            valid_pts.append(p)
            
    return valid_pts

def get_valid_ep(container: Container, item: Item, eps: Set[ExtremePoint]) -> List[ExtremePoint]:
    """Checks if the item fits at the given EPs without collision or exceeding boundaries."""
    valid_eps = []
    w, h, d = item.get_dimension()
    
    for ep in eps:
        if (ep.x + w <= container.width and 
            ep.y + h <= container.height and 
            ep.z + d <= container.depth):
            
            collision = False
            for other in container.items:
                ow, oh, od = other.get_dimension()
                ox, oy, oz = other.position
                
                # AABB Collision with a small tolerance
                if (ep.x < ox + ow - 0.001 and ep.x + w > ox + 0.001 and
                    ep.y < oy + oh - 0.001 and ep.y + h > oy + 0.001 and
                    ep.z < oz + od - 0.001 and ep.z + d > oz + 0.001):
                    collision = True
                    break
            
            if not collision:
                valid_eps.append(ep)
                
    return valid_eps
