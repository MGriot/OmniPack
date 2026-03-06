from typing import List, Tuple, Set, Optional
from .models import Item, Container, Rotation
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep

class Level1Engine:
    def __init__(self, container: Container, stability_factor=1.0):
        self.container = container
        self.stability_factor = stability_factor
        self.extreme_points: Set[ExtremePoint] = set()
        for part in container.parts:
            self.extreme_points.add(ExtremePoint(part.dx, part.dy, part.dz))

    def _get_overlap_area(self, ax1, ay1, ax2, ay2, bx1, by1, bx2, by2):
        ix1, iy1 = max(ax1, bx1), max(ay1, by1)
        ix2, iy2 = min(ax2, bx2), min(ay2, by2)
        return max(0, ix2 - ix1) * max(0, iy2 - iy1) if ix1 < ix2 and iy1 < iy2 else 0

    def _calculate_total_load_on_box(self, box_pos, box_dim, box_weight, all_boxes):
        """Recursively calculates load on a specific box."""
        tx, ty, tz = box_pos
        tw, th, td = box_dim
        target_top = tz + td
        total_above = 0.0
        
        for i, (opos, odim, oweight, omax) in enumerate(all_boxes):
            ox, oy, oz = opos
            ow, oh, od = odim
            
            if abs(oz - target_top) < 0.001:
                overlap = self._get_overlap_area(tx, ty, tx+tw, ty+th, ox, oy, ox+ow, oy+oh)
                if overlap > 0:
                    proportion = overlap / (ow * oh)
                    # This other box's weight + what's on it
                    other_total = oweight + self._calculate_total_load_on_box(opos, odim, oweight, all_boxes)
                    total_above += other_total * proportion
        return total_above

    def pack(self, items: List[Item], **kwargs) -> List[Item]:
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        z_multiplier = 1.0 + (self.stability_factor * 4999.0)

        for item in sorted_items:
            best_fit = None
            
            # Expand existing items for load calculation
            all_boxes = []
            for ex in self.container.items:
                ex_parts = ex.get_rotated_parts()
                total_vol = ex.volume()
                for (px, py, pz), (pw, ph, pd) in ex_parts:
                    p_pos = (ex.position[0] + px, ex.position[1] + py, ex.position[2] + pz)
                    p_weight = ex.weight * ((pw*ph*pd)/total_vol) if total_vol > 0 else 0
                    all_boxes.append((p_pos, (pw, ph, pd), p_weight, ex.max_stack_weight))

            for rot in item.allowed_rotations:
                item.rotation = rot
                valid_eps = get_valid_ep(self.container, item, self.extreme_points)
                
                for ep in valid_eps:
                    can_place = True
                    item.position = ep.to_tuple()
                    item_parts = item.get_rotated_parts()
                    
                    # Simple Support & Weight Check for Level 1
                    for (px, py, pz), (pw, ph, pd) in item_parts:
                        ax, ay, az = ep.x + px, ep.y + py, ep.z + pz
                        part_weight = item.weight * ((pw*ph*pd)/item.volume()) if item.volume() > 0 else 0
                        
                        for b_pos, b_dim, b_weight, b_max in all_boxes:
                            bx, by, bz = b_pos
                            bw, bh, bd = b_dim
                            if abs(az - (bz + bd)) < 0.001:
                                overlap = self._get_overlap_area(ax, ay, ax+pw, ay+ph, bx, by, bx+bw, by+bh)
                                if overlap > 0:
                                    current_load = self._calculate_total_load_on_box(b_pos, b_dim, b_weight, all_boxes)
                                    added_load = part_weight * (overlap / (pw * ph))
                                    if (current_load + added_load) > b_max + 0.001:
                                        can_place = False; break
                        if not can_place: break
                    
                    if not can_place: continue
                    
                    score = (ep.z * z_multiplier, ep.y, ep.x)
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
