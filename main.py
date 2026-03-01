from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Optional
from core.models import Item, Container
from core.engine import Level1Engine
from core.engine_v2 import Level2Engine
from core.genetic import GeneticOptimizer

app = FastAPI(title="OmniPack API")

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

@app.post("/pack")
async def pack_items_api(request: PackingRequest):
    # Convert input to internal models
    container = Container(
        request.container.id, 
        request.container.width, 
        request.container.height, 
        request.container.depth
    )
    items = [
        Item(i.id, i.width, i.height, i.depth) for i in request.items
    ]

    if request.mode == "level1":
        engine = Level1Engine(container)
        unpacked = engine.pack(items)
    elif request.mode == "level2":
        engine = Level2Engine(container)
        unpacked = engine.pack(items)
    elif request.mode == "genetic":
        optimizer = GeneticOptimizer(container, items, population_size=10, generations=5)
        packed_items, fitness = optimizer.evolve()
        # The optimizer already placed items in a container instance internally, 
        # but for consistency with the API, we'll return the container.to_dict()
        # Note: GeneticOptimizer.evolve() returns (packed_items, fitness)
        # We need to ensure the container reflects these items.
        container.items = packed_items
    else:
        raise HTTPException(status_code=400, detail="Invalid mode. Choose level1, level2, or genetic.")

    return {
        "container": container.to_dict(),
        "unpacked_count": len(items) - len(container.items) if request.mode != "genetic" else "N/A (GA optimized)"
    }

@app.get("/")
async def root():
    return {"message": "OmniPack API is running. Use /pack to optimize space."}
