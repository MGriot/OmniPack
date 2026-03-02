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

app = FastAPI(title="OmniPack API")

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

@app.get("/")
async def root():
    return {
        "message": "OmniPack API is running.",
        "engines": ["level1", "level2", "genetic", "mcts"],
        "status": "stable"
    }

class ItemInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float
    weight: float = 0.0
    max_stack_weight: float = 1000000.0
    allow_mixing: bool = True 
    group_id: Optional[str] = None

class ContainerInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float

class PackingRequest(BaseModel):
    container: ContainerInput
    items: List[ItemInput]
    mode: str = "level2" 
    strategy: str = "minimize_out" 
    iterations: int = 20 

@app.post("/pack")
async def pack_items_api(request: PackingRequest):
    base_container = Container(
        request.container.id, 
        request.container.width, 
        request.container.height, 
        request.container.depth
    )
    
    all_scenarios = []
    
    for i in range(request.iterations):
        final_list = []
        if i == 0:
            for inp in request.items:
                final_list.append(Item(
                    id=inp.id, width=inp.width, height=inp.height, depth=inp.depth, 
                    weight=inp.weight, max_stack_weight=inp.max_stack_weight,
                    group_id=inp.group_id
                ))
        else:
            mix_pool = []
            rigid_groups = {} 
            
            for inp in request.items:
                it = Item(
                    id=inp.id, width=inp.width, height=inp.height, depth=inp.depth, 
                    weight=inp.weight, max_stack_weight=inp.max_stack_weight,
                    group_id=inp.group_id
                )
                # Grouping key: prefer group_id, then id type
                g_id = inp.group_id if inp.group_id else inp.id.rsplit('_', 1)[0]
                
                if inp.allow_mixing:
                    mix_pool.append([it]) 
                else:
                    if g_id not in rigid_groups: rigid_groups[g_id] = []
                    rigid_groups[g_id].append(it)
            
            all_units = mix_pool + list(rigid_groups.values())
            random.shuffle(all_units)
            for unit in all_units:
                for it in unit:
                    final_list.append(it)

        multi_engine = MultiContainerEngine(base_container)
        containers = multi_engine.pack_all(final_list, strategy=request.strategy, mode=request.mode)
        
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
