from typing import List, Tuple, Set, Optional
from .models import Item, Container, Rotation, PackingVersus
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep

class Level1Engine:
    def __init__(self, container: Container, stability_factor=1.0, versus: PackingVersus = PackingVersus.LONGITUDINAL):
        self.container = container
        self.stability_factor = stability_factor
        self.versus = versus
        self.extreme_points: Set[ExtremePoint] = set()
        self.extreme_points.add(ExtremePoint(0, 0, 0))

    def _get_overlap_area(self, ax1, ay1, ax2, ay2, bx1, by1, bx2, by2):
        ix1, iy1 = max(ax1, bx1), max(ay1, by1)
        ix2, iy2 = min(ax2, bx2), min(ay2, by2)
        return max(0, ix2 - ix1) * max(0, iy2 - iy1) if ix1 < ix2 and iy1 < iy2 else 0

    def _calculate_total_load_on_item(self, item, all_items):
        tx, ty, tz = item.position
        tw, th, td = item.get_dimension()
        target_top = tz + td
        total_above = 0.0
        
        for other in all_items:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            
            if abs(oz - target_top) < 0.001:
                overlap = self._get_overlap_area(tx, ty, tx+tw, ty+th, ox, oy, ox+ow, oy+oh)
                if overlap > 0:
                    proportion = overlap / (ow * oh)
                    total_above += (other.weight + self._calculate_total_load_on_item(other, all_items)) * proportion
        return total_above

    def pack(self, items: List[Item], **kwargs) -> List[Item]:
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []
        z_multiplier = 1.0 + (self.stability_factor * 4999.0)

        for item in sorted_items:
            best_fit = None
            for rot in item.allowed_rotations:
                item.rotation = rot
                valid_eps = get_valid_ep(self.container, item, self.extreme_points)
                
                for ep in valid_eps:
                    item.position = ep.to_tuple()
                    can_place = True
                    
                    # Weight Check
                    for existing in self.container.items:
                        ex_pos = existing.position
                        ex_dim = existing.get_dimension()
                        if abs(item.position[2] - (ex_pos[2] + ex_dim[2])) < 0.001:
                            iw, ih, _ = item.get_dimension()
                            overlap = self._get_overlap_area(item.position[0], item.position[1], item.position[0]+iw, item.position[1]+ih, 
                                                            ex_pos[0], ex_pos[1], ex_pos[0]+ex_dim[0], ex_pos[1]+ex_dim[1])
                            if overlap > 0:
                                current_load = self._calculate_total_load_on_item(existing, self.container.items)
                                added_load = item.weight * (overlap / (iw * ih))
                                if (current_load + added_load) > existing.max_stack_weight + 0.001:
                                    can_place = False; break
                    
                    if not can_place: continue
                    
                    # Apply Versus scoring
                    z_val = ep.z * z_multiplier
                    if self.versus == PackingVersus.LATERAL:
                        # Prioritize X
                        score = (z_val, ep.y, ep.x)
                    elif self.versus == PackingVersus.FLOOR_FIRST:
                        # Prioritize Floor (Y low)
                        score = (ep.y, z_val, ep.x)
                    else: # LONGITUDINAL
                        # Prioritize Z
                        score = (ep.x, ep.y, z_val)

                    if best_fit is None or score < best_fit[2]:
                        best_fit = (ep, rot, score)
            
            if best_fit:
                ep, rot, _ = best_fit
                item.position = ep.to_tuple()
                item.rotation = rot
                self.container.items.append(item)
                for nep in generate_extreme_points(self.container, item): self.extreme_points.add(nep)
                self.extreme_points.remove(ep)
            else:
                unpacked_items.append(item)
        return unpacked_items
