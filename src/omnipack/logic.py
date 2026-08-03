import json
import os
import random
from .core.models import Item, Container, LoadingStrategy, Rotation, PackingVersus, ShapeType
from .core.multi_container import MultiContainerEngine

# Data directory logic
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.environ.get("OMNIPACK_DATA_DIR", BASE_DIR)
if not os.path.exists(DATA_DIR):
    try: os.makedirs(DATA_DIR, exist_ok=True)
    except: pass

CATALOG_PATH = os.path.join(DATA_DIR, "catalog.json")

async def get_catalog_data():
    if not os.path.exists(CATALOG_PATH): return {"items": []}
    with open(CATALOG_PATH, "r") as f:
        try: return json.load(f)
        except: return {"items": []}

async def save_to_catalog_data(entry: dict):
    catalog = await get_catalog_data()
    existing_idx = next((i for i, item in enumerate(catalog["items"]) if item["name"] == entry["name"]), None)
    if existing_idx is not None: catalog["items"][existing_idx] = entry
    else: catalog["items"].append(entry)
    with open(CATALOG_PATH, "w") as f: json.dump(catalog, f, indent=4)
    return {"message": "Saved to catalog", "name": entry["name"]}

async def delete_from_catalog_data(name: str):
    catalog = await get_catalog_data()
    catalog["items"] = [item for item in catalog["items"] if item["name"] != name]
    with open(CATALOG_PATH, "w") as f: json.dump(catalog, f, indent=4)
    return {"message": "Deleted from catalog", "name": name}

async def run_pack_logic(payload: dict):
    container_in = payload.get("container", {})
    items_in = payload.get("items", [])
    c_st_name = container_in.get("shape_type", "BOX").upper()
    c_st = ShapeType[c_st_name] if c_st_name in ShapeType.__members__ else ShapeType.BOX
    
    base_container = Container(
        container_in.get("id", "C"), 
        container_in.get("width", 100), container_in.get("height", 100), container_in.get("depth", 100),
        shape_type=c_st
    )
    
    versus = PackingVersus.LONGITUDINAL
    pv_name = payload.get("packing_versus", "LONGITUDINAL").upper()
    if pv_name in PackingVersus.__members__: versus = PackingVersus[pv_name]

    iterations = int(payload.get("iterations", 20))
    mode = payload.get("mode", "level2")
    stability_factor = float(payload.get("stability_factor", 1.0))
    enable_mixing = payload.get("enable_mixing", True)
    enable_strategy = payload.get("enable_strategy", True)
    enable_rotation = payload.get("enable_rotation", True)
    
    grasp_k = int(payload.get("grasp_k", 3 if iterations > 1 else 1))

    all_scenarios = []
    for i in range(iterations):
        final_list = []
        mix_pool = []
        rigid_units = []
        
        for inp in items_in:
            if not enable_rotation: arots = [Rotation.W_H_D]
            elif inp.get("allowed_rotations") is not None: arots = [Rotation(r) for r in inp["allowed_rotations"]]
            else: arots = [r for r in Rotation]

            strat = LoadingStrategy.NONE
            if enable_strategy:
                s_name = inp.get("strategy", "NONE").upper()
                if s_name == "FIFO": strat = LoadingStrategy.FIFO
                elif s_name == "LIFO": strat = LoadingStrategy.LIFO
            
            i_st_name = inp.get("shape_type", "BOX").upper()
            i_st = ShapeType[i_st_name] if i_st_name in ShapeType.__members__ else ShapeType.BOX

            qty = inp.get("quantity", 1)
            group_items = []
            for q in range(qty):
                it = Item(
                    id=f"{inp['id']}_{q}", width=inp["width"], height=inp["height"], depth=inp["depth"], 
                    weight=inp.get("weight", 0.0), max_stack_weight=inp.get("max_stack_weight", 1000000.0),
                    group_id=inp.get("group_id"), strategy=strat, stop_id=inp.get("stop_id", 0),
                    allowed_rotations=arots, shape_type=i_st
                )
                if enable_mixing and inp.get("allow_mixing") is not False: mix_pool.append(it)
                else: group_items.append(it)
            if group_items: rigid_units.append(group_items)
        
        random.shuffle(mix_pool)
        all_units = [[it] for it in mix_pool] + rigid_units
        if i > 0: random.shuffle(all_units)
            
        for unit in all_units:
            for it in unit: final_list.append(it)

        # Disturbance increases with iterations to explore more variety
        disturb = (i / iterations) * 0.2 if iterations > 1 else 0.0
        
        multi_engine = MultiContainerEngine(base_container, versus=versus)
        containers = multi_engine.pack_all(
            final_list, mode=mode, stability_factor=stability_factor,
            grasp_k=grasp_k, random_disturbance=disturb
        )
        
        total_vol = sum(c.volume() for c in containers)
        used_vol = sum(sum(it.volume() for it in c.items) for c in containers)
        utilization = (used_vol / total_vol) * 100 if total_vol > 0 else 0
        
        all_scenarios.append({
            "id": f"Scenario_{i+1}",
            "containers": [c.to_dict() for c in containers],
            "total_containers": len(containers),
            "global_utilization": utilization
        })

    all_scenarios.sort(key=lambda x: x["global_utilization"], reverse=True)
    unique_scenarios = []
    seen_utils = set()
    for s in all_scenarios:
        u_rounded = round(s["global_utilization"], 4)
        if u_rounded not in seen_utils:
            unique_scenarios.append(s)
            seen_utils.add(u_rounded)
            if len(unique_scenarios) >= 5: break

    return {
        "suggestions": unique_scenarios,
        "best_utilization": unique_scenarios[0]["global_utilization"] if unique_scenarios else 0
    }
