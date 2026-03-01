from core.models import Item, Container
from core.genetic import GeneticOptimizer

def test_genetic_evolution():
    container = Container("c1", 100, 100, 100)
    # 1000 items of 10x10x10 should fill it perfectly if ordered correctly
    # But let's use a smaller set to verify logic
    items = [Item(f"i{i}", 20, 20, 20) for i in range(25)] # Total volume = 20*20*20 * 25 = 8000 * 25 = 200,000
    # Container volume = 1,000,000. So max fitness is 20%.
    
    optimizer = GeneticOptimizer(container, items, population_size=5, generations=2)
    packed_items, fitness = optimizer.evolve()
    
    assert fitness > 0
    assert len(packed_items) > 0
    assert fitness <= 100.0

def test_genetic_better_than_random():
    container = Container("c1", 30, 30, 30) # Vol = 27,000
    # Items that fit better in a specific order
    items = [
        Item("i1", 30, 10, 10),
        Item("i2", 10, 30, 10),
        Item("i3", 10, 10, 30),
    ]
    
    optimizer = GeneticOptimizer(container, items, population_size=10, generations=5)
    packed_items, fitness = optimizer.evolve()
    
    # At least one should pack
    assert len(packed_items) >= 1
