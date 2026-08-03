from omnipack.backend import app

if __name__ == "__main__":
    import uvicorn
    # This entry point is for local dev/testing via python main.py
    uvicorn.run(app, host="127.0.0.1", port=8000)
