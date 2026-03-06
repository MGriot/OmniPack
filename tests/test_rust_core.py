import pytest
import omnipack_core

def test_rust_hello():
    result = omnipack_core.hello_rust()
    print(f"\nRust says: {result}")
    assert result == "OmniPack Rust Core Online"

if __name__ == "__main__":
    # Manual check
    print(omnipack_core.hello_rust())
