import traceback
import os
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel
from typing import List, Optional
from fastapi.responses import FileResponse

# Import from our new clean logic module
from .logic import run_pack_logic, get_catalog_data, save_to_catalog_data, delete_from_catalog_data

# Determine base path relative to this file
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
VIEWER_PATH = os.path.join(BASE_DIR, "viewer.html")

app = FastAPI(title="OmniPack API")

# Serve vendored static assets (e.g. vendor/babylon.js) that viewer.html
# references with a relative path.
app.mount("/vendor", StaticFiles(directory=os.path.join(BASE_DIR, "vendor")), name="vendor")

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
        "status": "stable"
    }

@app.get("/catalog")
async def get_catalog():
    return await get_catalog_data()

@app.post("/catalog")
async def save_to_catalog(entry: dict):
    return await save_to_catalog_data(entry)

@app.delete("/catalog/{name}")
async def delete_from_catalog(name: str):
    return await delete_from_catalog_data(name)

@app.post("/pack")
async def pack_items_api(request: dict):
    # We accept a raw dict from the API to match the run_pack_logic interface
    return await run_pack_logic(request)
