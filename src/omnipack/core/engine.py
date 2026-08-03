from typing import List, Tuple, Set, Optional
from .models import Item, Container, Rotation, LoadingStrategy, PackingVersus
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep

class Level1Engine:
    def __init__(self, container: Container, stability_factor=1.0, versus: PackingVersus = PackingVersus.LONGITUDINAL):
        self.container = container
        self.stability_factor = stability_factor
        self.versus = versus
        self.extreme_points: Set[ExtremePoint] = set()
        self.extreme_points.add(ExtremePoint(0, 0, 0))

    def pack(self, items: List[Item], **kwargs) -> List[Item]:
        # Simple Best-Fit Decreasing
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        for item in sorted_items:
            best_ep = None
            best_rot = None
            
            # Simple check for each EP and Rotation
            for ep in self.extreme_points:
                for rot in item.allowed_rotations:
                    item.rotation = rot
                    w, h, d = item.get_dimension()
                    
                    if (ep.x + w <= self.container.width and
                        ep.y + h <= self.container.height and
                        ep.z + d <= self.container.depth):
                        
                        # Collision check
                        collision = False
                        for other in self.container.items:
                            ow, oh, od = other.get_dimension()
                            ox, oy, oz = other.position
                            if not (ep.x + w <= ox or ep.x >= ox + ow or
                                    ep.y + h <= oy or ep.y >= oy + oh or
                                    ep.z + d <= oz or ep.z >= oz + od):
                                collision = True; break
                        
                        if not collision:
                            best_ep = ep
                            best_rot = rot
                            break
                if best_ep: break
            
            if best_ep:
                item.position = best_ep.to_tuple()
                item.rotation = best_rot
                self.container.items.append(item)
                self.extreme_points.remove(best_ep)
                for nep in generate_extreme_points(self.container, item): self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
        return unpacked_items
