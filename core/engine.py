from typing import List, Tuple, Set, Optional
from .models import Item, Container, Rotation
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep

class Level1Engine:
    def __init__(self, container: Container):
        self.container = container
        self.extreme_points: Set[ExtremePoint] = {ExtremePoint(0, 0, 0)}

    def pack(self, items: List[Item]) -> List[Item]:
        """
        Packs items using Best-Fit Decreasing heuristic with Extreme Points.
        1. Sort items by volume (Decreasing).
        2. For each item, try all 6 rotations.
        3. For each rotation, find the 'best' Extreme Point.
        """
        # Sort items by volume descending
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        for item in sorted_items:
            best_fit = None # (EP, Rotation, Score)
            
            # Try all 6 rotations
            for rot in Rotation:
                item.rotation = rot
                valid_eps = get_valid_ep(self.container, item, self.extreme_points)
                
                for ep in valid_eps:
                    # Simple score: minimize remaining Z, then Y, then X (Bottom-Left-Back)
                    score = (ep.z, ep.y, ep.x)
                    if best_fit is None or score < best_fit[2]:
                        best_fit = (ep, rot, score)
            
            if best_fit:
                ep, rot, _ = best_fit
                item.position = ep.to_tuple()
                item.rotation = rot
                
                # Place item
                self.container.items.append(item)
                
                # Update Extreme Points
                new_eps = generate_extreme_points(self.container, item)
                self.extreme_points.remove(ep)
                for nep in new_eps:
                    self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
                
        return unpacked_items

def pack_items(container: Container, items: List[Item]) -> Tuple[Container, List[Item]]:
    engine = Level1Engine(container)
    unpacked = engine.pack(items)
    return container, unpacked
