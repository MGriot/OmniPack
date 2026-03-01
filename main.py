from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional, Union
from core.models import Item, Container
from core.engine import Level1Engine
from core.engine_v2 import Level2Engine
from core.genetic import GeneticOptimizer
from core.multi_container import MultiContainerEngine

app = FastAPI(title="OmniPack API")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class ItemInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float

class ContainerInput(BaseModel):
    id: str
    width: float
    height: float
    depth: float

class PackingRequest(BaseModel):
    container: ContainerInput
    items: List[ItemInput]
    mode: str = "level1" # "level1", "level2", "genetic"
    strategy: str = "minimize_out" # "minimize_out", "optimal_balance"

@app.post("/pack")
async def pack_items_api(request: PackingRequest):
    container = Container(
        request.container.id, 
        request.container.width, 
        request.container.height, 
        request.container.depth
    )
    items = [
        Item(i.id, i.width, i.height, i.depth) for i in request.items
    ]

    # Handle multi-container logic
    multi_engine = MultiContainerEngine(container)
    containers = multi_engine.pack_all(items, strategy=request.strategy)

    return {
        "containers": [c.to_dict() for c in containers],
        "total_containers": len(containers)
    }

@app.get("/")
async def root():
    return {"message": "OmniPack API is running. Use /pack to optimize space."}
