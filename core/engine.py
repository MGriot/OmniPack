from typing import List, Tuple, Set, Optional
from .models import Item, Container, Rotation
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep

class Level1Engine:
    def __init__(self, container: Container):
        self.container = container
        self.extreme_points: Set[ExtremePoint] = {ExtremePoint(0, 0, 0)}

    def _get_overlap_area(self, ax1, ay1, ax2, ay2, bx1, by1, bx2, by2):
        ix1, iy1 = max(ax1, bx1), max(ay1, by1)
        ix2, iy2 = min(ax2, bx2), min(ay2, by2)
        return max(0, ix2 - ix1) * max(0, iy2 - iy1) if ix1 < ix2 and iy1 < iy2 else 0

    def _calculate_total_load_on(self, target_idx: int, items: List[Item]) -> float:
        """Calculates total weight (own + above) resting on items[target_idx]."""
        target = items[target_idx]
        tx, ty, tz = target.position
        tw, th, td = target.get_dimension()
        target_top = tz + td
        total_above = 0.0
        
        for i, item in enumerate(items):
            if i == target_idx: continue
            ix, iy, iz = item.position
            iw, ih, id_ = item.get_dimension()
            
            # If item is directly on top of target
            if abs(iz - target_top) < 0.001:
                overlap = self._get_overlap_area(tx, ty, tx+tw, ty+th, ix, iy, ix+iw, iy+ih)
                if overlap > 0:
                    proportion = overlap / (iw * ih)
                    # Proportion of (item's weight + everything on top of it)
                    item_total_load = item.weight + self._calculate_total_load_on(i, items)
                    total_above += item_total_load * proportion
        return total_above

    def pack(self, items: List[Item]) -> List[Item]:
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        for item in sorted_items:
            best_fit = None
            for rot in Rotation:
                item.rotation = rot
                valid_eps = get_valid_ep(self.container, item, self.extreme_points)
                
                for ep in valid_eps:
                    can_place = True
                    w, h, d = item.get_dimension()
                    
                    # Check every existing item if it would support the new item
                    for i, other in enumerate(self.container.items):
                        ox, oy, oz = other.position
                        ow, oh, od = other.get_dimension()
                        
                        # If new item is on top of 'other'
                        if abs(ep.z - (oz + od)) < 0.001:
                            overlap = self._get_overlap_area(ep.x, ep.y, ep.x+w, ep.y+h, ox, oy, ox+ow, oy+oh)
                            if overlap > 0:
                                current_load = self._calculate_total_load_on(i, self.container.items)
                                added_weight = item.weight * (overlap / (w * h))
                                if (current_load + added_weight) > other.max_stack_weight:
                                    can_place = False
                                    break
                    
                    if not can_place: continue
                    # EXTREME FLOOR PRIORITY
                    # Multiply Z by 1000 to ensure floor spots are always chosen over stacking
                    score = (ep.z * 1000.0, ep.y, ep.x)
                    if best_fit is None or score < best_fit[2]:
                        best_fit = (ep, rot, score)
            
            if best_fit:
                ep, rot, _ = best_fit
                item.position = ep.to_tuple()
                item.rotation = rot
                self.container.items.append(item)
                new_eps = generate_extreme_points(self.container, item)
                self.extreme_points.remove(ep)
                for nep in new_eps: self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
        return unpacked_items
