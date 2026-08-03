import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.core import __version__

def test_version():
    assert __version__ == "0.1.0"
