import random
import copy
from typing import List, Tuple
from .models import Item, Container
from .engine_v2 import Level2Engine

class GeneticOptimizer:
    def __init__(self, container: Container, items: List[Item], population_size=20, generations=10):
        self.container = container
        self.items = items
        self.population_size = population_size
        self.generations = generations

    def _create_individual(self) -> List[int]:
        """An individual is a permutation of item indices."""
        indices = list(range(len(self.items)))
        random.shuffle(indices)
        return indices

    def _evaluate(self, individual: List[int]) -> float:
        """Fitness function: Total packed volume."""
        # Reset container for simulation
        sim_container = Container(
            self.container.id, 
            self.container.width, 
            self.container.height, 
            self.container.depth
        )
        # Order items according to the individual's genome
        ordered_items = [copy.deepcopy(self.items[i]) for i in individual]
        
        engine = Level2Engine(sim_container)
        engine.pack(ordered_items)
        
        # Fitness = percentage of volume filled
        return (sum(item.volume() for item in sim_container.items) / self.container.volume()) * 100

    def _crossover(self, parent1: List[int], parent2: List[int]) -> List[int]:
        """Ordered crossover (OX1) for permutations."""
        size = len(parent1)
        start, end = sorted([random.randrange(size) for _ in range(2)])
        
        child = [-1] * size
        child[start:end] = parent1[start:end]
        
        # Fill remaining from parent2
        p2_idx = 0
        for i in range(size):
            if child[i] == -1:
                while parent2[p2_idx] in child:
                    p2_idx += 1
                child[i] = parent2[p2_idx]
        return child

    def _mutate(self, individual: List[int], mutation_rate=0.1):
        """Swap mutation."""
        if random.random() < mutation_rate:
            idx1, idx2 = random.sample(range(len(individual)), 2)
            individual[idx1], individual[idx2] = individual[idx2], individual[idx1]

    def evolve(self) -> Tuple[List[Item], float]:
        """Runs the genetic evolution."""
        population = [self._create_individual() for _ in range(self.population_size)]
        
        best_genome = None
        best_fitness = -1.0

        for gen in range(self.generations):
            # Evaluate population
            scored_pop = []
            for genome in population:
                fitness = self._evaluate(genome)
                scored_pop.append((genome, fitness))
                if fitness > best_fitness:
                    best_fitness = fitness
                    best_genome = genome
            
            # Sort by fitness
            scored_pop.sort(key=lambda x: x[1], reverse=True)
            
            # Selection (Elitism + Tournament/Top)
            new_population = [p[0] for p in scored_pop[:2]] # Keep top 2
            
            while len(new_population) < self.population_size:
                p1, p2 = random.sample(scored_pop[:10], 2) # Sample from top 10
                child = self._crossover(p1[0], p2[0])
                self._mutate(child)
                new_population.append(child)
                
            population = new_population

        # Final pack with the best found sequence
        final_container = Container(
            self.container.id, 
            self.container.width, 
            self.container.height, 
            self.container.depth
        )
        final_items = [copy.deepcopy(self.items[i]) for i in best_genome]
        engine = Level2Engine(final_container)
        engine.pack(final_items)
        
        return final_container.items, best_fitness
