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
    ix, iy, iz = item.position
    iw, ih, id = item.get_dimension()
    existing = container.items

    potential_pts = [
        [ix, iy, iz + id], # Z first
        [ix, iy + ih, iz], # Y second
        [ix + iw, iy, iz], # X third
        [ix, iy, iz] # LIFO
    ]

    final_pts = []
    for p in potential_pts:
        px, py, pz = p

        max_x = 0.0
        for other in existing:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            if ox + ow <= px + 0.001 and (oy < py + 0.001 and oy + oh > py - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_x = max(max_x, ox + ow)

        max_y = 0.0
        for other in existing:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            if oy + oh <= py + 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_y = max(max_y, oy + oh)

        max_z = 0.0
        for other in existing:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            if oz + od <= pz + 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oy < py + 0.001 and oy + oh > py - 0.001):
                max_z = max(max_z, oz + od)

        min_z = container.depth
        for other in existing:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            if oz >= pz - 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oy < py + 0.001 and oy + oh > py - 0.001):
                min_z = min(min_z, oz)

        final_pts.append(ExtremePoint(px, py, pz))
        if px > max_x: final_pts.append(ExtremePoint(max_x, py, pz))
        if py > max_y: final_pts.append(ExtremePoint(px, max_y, pz))
        if pz > max_z: final_pts.append(ExtremePoint(px, py, max_z))
        if pz < min_z: final_pts.append(ExtremePoint(px, py, min_z))

    valid_pts = []
    seen = set()
    for p in final_pts:
        if p.x > container.width or p.y > container.height or p.z > container.depth: continue
        if p.x < 0 or p.y < 0 or p.z < 0: continue
        p_round = (round(p.x, 3), round(p.y, 3), round(p.z, 3))
        if p_round not in seen:
            valid_pts.append(p)
            seen.add(p_round)
    return valid_pts

def get_valid_ep(container: Container, item: Item, eps: Set[ExtremePoint]) -> List[ExtremePoint]:
    valid_eps = []
    existing = container.items
    cw, ch, cd = container.width, container.height, container.depth
    cst = container.shape_type

    for ep in eps:
        ax, ay, az = ep.to_tuple()
        aw, ah, ad = item.get_dimension()
        ast = item.shape_type

        # In Container Check (Reuse logic from accelerated if possible, but keep it simple here)
        if not (ax >= -0.001 and ax + aw <= cw + 0.001 and
                ay >= -0.001 and ay + ah <= ch + 0.001 and
                az >= -0.001 and az + ad <= cd + 0.001):
            continue

        collision = False
        for other in existing:
            ox, oy, oz = other.position
            ow, oh, od = other.get_dimension()
            ost = other.shape_type

            # Simple Box-Box Early Exit
            if not (ax >= ox + ow - 0.001 or ax + aw <= ox + 0.001 or
                    ay >= oy + oh - 0.001 or ay + ah <= oy + 0.001 or
                    az >= oz + od - 0.001 or az + ad <= oz + 0.001):

                if ast == 0 and ost == 0: collision = True; break
                if ast == 1 and ost == 1: # Sphere-Sphere
                    r1, r2 = aw/2, ow/2
                    dist_sq = (ax+r1-(ox+r2))**2 + (ay+r1-(oy+r2))**2 + (az+r1-(oz+r2))**2
                    if dist_sq < (r1+r2)**2 - 0.001: collision = True; break
                else: # Sphere-Box or Box-Sphere
                    collision = True; break # Conservative

        if not collision:
            valid_eps.append(ep)
    return valid_eps
