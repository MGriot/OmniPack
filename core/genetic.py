import random
import copy
from typing import List, Tuple
from .models import Item, Container
from .engine import Level1Engine
from .engine_v2 import Level2Engine

class GeneticOptimizer:
    def __init__(self, container: Container, items: List[Item], population_size=20, generations=10, mode="level2"):
        self.container = container
        self.items = items
        self.population_size = population_size
        self.generations = generations
        self.mode = mode

    def _get_engine(self, container: Container):
        if self.mode == "level1": return Level1Engine(container)
        return Level2Engine(container)

    def _create_individual(self) -> List[int]:
        indices = list(range(len(self.items)))
        random.shuffle(indices)
        return indices

    def _evaluate(self, individual: List[int]) -> float:
        sim_container = Container(self.container.id, self.container.width, self.container.height, self.container.depth)
        ordered_items = [copy.deepcopy(self.items[i]) for i in individual]
        engine = self._get_engine(sim_container)
        engine.pack(ordered_items)
        return (sum(item.volume() for item in sim_container.items) / self.container.volume()) * 100

    def _crossover(self, parent1: List[int], parent2: List[int]) -> List[int]:
        size = len(parent1)
        start, end = sorted([random.randrange(size) for _ in range(2)])
        child = [-1] * size
        child[start:end] = parent1[start:end]
        p2_idx = 0
        for i in range(size):
            if child[i] == -1:
                while parent2[p2_idx] in child: p2_idx += 1
                child[i] = parent2[p2_idx]
        return child

    def _mutate(self, individual: List[int], mutation_rate=0.1):
        if len(individual) < 2: return
        if random.random() < mutation_rate:
            idx1, idx2 = random.sample(range(len(individual)), 2)
            individual[idx1], individual[idx2] = individual[idx2], individual[idx1]

    def evolve(self) -> Tuple[List[Item], float]:
        population = [self._create_individual() for _ in range(self.population_size)]
        best_genome, best_fitness = None, -1.0

        for gen in range(self.generations):
            scored_pop = []
            for genome in population:
                fitness = self._evaluate(genome)
                scored_pop.append((genome, fitness))
                if fitness > best_fitness: best_fitness, best_genome = fitness, genome
            scored_pop.sort(key=lambda x: x[1], reverse=True)
            new_population = [p[0] for p in scored_pop[:2]]
            while len(new_population) < self.population_size:
                p1, p2 = random.sample(scored_pop[:10], 2)
                child = self._crossover(p1[0], p2[0])
                self._mutate(child)
                new_population.append(child)
            population = new_population

        final_container = Container(self.container.id, self.container.width, self.container.height, self.container.depth)
        final_items = [copy.deepcopy(self.items[i]) for i in best_genome]
        engine = self._get_engine(final_container)
        engine.pack(final_items)
        return final_container.items, best_fitness
