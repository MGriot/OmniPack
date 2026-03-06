import traceback
import random
import copy
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional, Union
from core.models import Item, Container
from core.engine import Level1Engine
from core.engine_v2 import Level2Engine
from core.genetic import GeneticOptimizer
from core.mcts import MonteCarloOptimizer
from core.multi_container import MultiContainerEngine

from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse

import os

app = FastAPI(title="OmniPack API")

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception):
    trace = traceback.format_exc()
    print(f"ERROR: {exc}\n{trace}")
    return JSONResponse(
        status_code=500,
        content={"detail": str(exc), "trace": trace}
    )

# Get absolute path to viewer.html
VIEWER_PATH = os.path.join(os.path.dirname(__file__), "viewer.html")

# Serve the visualizer at the root URL
@app.get("/", include_in_schema=False)
async def get_viewer():
    if not os.path.exists(VIEWER_PATH):
        return JSONResponse(status_code=404, content={"detail": f"viewer.html not found at {VIEWER_PATH}"})
    return FileResponse(VIEWER_PATH)

@app.get("/api-status")
async def api_status():
    return {
        "message": "OmniPack API is running.",
        "engines": ["level1", "level2", "genetic", "mcts"],
        "status": "stable"
    }

class PartInput(BaseModel):
    dx: float
    dy: float
    dz: float
    width: float
    height: float
    depth: float
    shape_type: str = "BOX"

class ItemInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float
    weight: float = 0.0
    max_stack_weight: float = 1000000.0
    allow_mixing: bool = True 
    group_id: Optional[str] = None
    strategy: str = "NONE" 
    stop_id: int = 0 
    allowed_rotations: Optional[List[int]] = None 
    shape_type: str = "BOX" # NEW: Direct shape selection
    parts: Optional[List[PartInput]] = None 

class ContainerInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float
    shape_type: str = "BOX" # NEW: Direct container shape selection
    parts: Optional[List[PartInput]] = None 

class PackingRequest(BaseModel):
    container: ContainerInput
    items: List[ItemInput]
    mode: str = "level2" 
    strategy: str = "minimize_out" 
    iterations: int = 20 
    stability_factor: float = 1.0
    grasp_k: int = 1 
    packing_versus: str = "LONGITUDINAL" 

@app.post("/pack")
async def pack_items_api(request: PackingRequest):
    from core.models import LoadingStrategy, Rotation, PackingVersus, ShapePart, ShapeType
    
    c_st = ShapeType[request.container.shape_type.upper()] if request.container.shape_type.upper() in ShapeType.__members__ else ShapeType.BOX
    
    custom_container_parts = []
    if request.container.parts:
        for p in request.container.parts:
            st = ShapeType[p.shape_type.upper()] if p.shape_type.upper() in ShapeType.__members__ else ShapeType.BOX
            custom_container_parts.append(ShapePart(p.dx, p.dy, p.dz, p.width, p.height, p.depth, st))

    base_container = Container(
        request.container.id, 
        request.container.width, 
        request.container.height, 
        request.container.depth,
        shape_type=c_st,
        parts=custom_container_parts
    )
    
    # Map versus
    versus = PackingVersus.LONGITUDINAL
    if request.packing_versus.upper() == "LATERAL": versus = PackingVersus.LATERAL
    elif request.packing_versus.upper() == "FLOOR_FIRST": versus = PackingVersus.FLOOR_FIRST

    all_scenarios = []
    
    for i in range(request.iterations):
        final_list = []
        
        mix_pool = []
        rigid_groups = {} 
        
        for inp in request.items:
            strat = LoadingStrategy.NONE
            if inp.strategy.upper() == "FIFO": strat = LoadingStrategy.FIFO
            elif inp.strategy.upper() == "LIFO": strat = LoadingStrategy.LIFO
            
            # Map rotations
            if inp.allowed_rotations is not None:
                arots = [Rotation(r) for r in inp.allowed_rotations]
            else:
                arots = [r for r in Rotation]

            i_st = ShapeType[inp.shape_type.upper()] if inp.shape_type.upper() in ShapeType.__members__ else ShapeType.BOX

            custom_parts = []
            if inp.parts:
                for p in inp.parts:
                    st = ShapeType[p.shape_type.upper()] if p.shape_type.upper() in ShapeType.__members__ else ShapeType.BOX
                    custom_parts.append(ShapePart(p.dx, p.dy, p.dz, p.width, p.height, p.depth, st))

            it = Item(
                id=inp.id, width=inp.width, height=inp.height, depth=inp.depth, 
                weight=inp.weight, max_stack_weight=inp.max_stack_weight,
                group_id=inp.group_id, strategy=strat, stop_id=inp.stop_id,
                allowed_rotations=arots,
                shape_type=i_st,
                parts=custom_parts
            )
            g_id = inp.group_id if inp.group_id else inp.id.rsplit('_', 1)[0]
            
            if inp.allow_mixing:
                mix_pool.append([it]) 
            else:
                if g_id not in rigid_groups: rigid_groups[g_id] = []
                rigid_groups[g_id].append(it)
        
        all_units = mix_pool + list(rigid_groups.values())
        
        # Only shuffle if it's NOT the first iteration (keep original order once)
        if i > 0:
            random.shuffle(all_units)
            
        for unit in all_units:
            for it in unit:
                final_list.append(it)

        multi_engine = MultiContainerEngine(base_container)
        containers = multi_engine.pack_all(
            final_list, 
            strategy=request.strategy, 
            mode=request.mode, 
            stability_factor=request.stability_factor,
            grasp_k=request.grasp_k
        )
        
        total_vol = sum(c.volume() for c in containers)
        used_vol = sum(sum(it.volume() for it in c.items) for c in containers)
        utilization = (used_vol / total_vol) * 100
        
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
