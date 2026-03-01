import math
import random
import copy
import time
from typing import List, Tuple, Dict, Optional
from .models import Item, Container, Rotation
from .engine_v2 import Level2Engine

class MCTSNode:
    def __init__(self, items_remaining: List[Item], container: Container, parent=None):
        self.items_remaining = items_remaining
        self.container = container
        self.parent = parent
        self.children: List['MCTSNode'] = []
        self.visits = 0
        self.total_reward = 0.0
        self.action = None # The item index chosen from items_remaining

    def is_fully_expanded(self) -> bool:
        return len(self.children) == len(self.items_remaining)

    def best_child(self, exploration_weight=1.41) -> 'MCTSNode':
        """Upper Confidence Bound for Trees (UCT)."""
        return max(
            self.children,
            key=lambda c: (c.total_reward / c.visits) + exploration_weight * math.sqrt(math.log(self.visits) / c.visits)
        )

class MonteCarloOptimizer:
    """
    Intelligent Search Engine using Monte Carlo Tree Search.
    Mimics 'thinking' by simulating multiple possible futures before placing an item.
    """
    def __init__(self, base_container: Container, items: List[Item], time_limit=2.0):
        self.base_container = base_container
        self.items = items
        self.time_limit = time_limit

    def _simulate(self, node: MCTSNode) -> float:
        """Rollout: Fast completion of packing using a heuristic (Level 2 engine)."""
        temp_container = copy.deepcopy(node.container)
        temp_items = copy.deepcopy(node.items_remaining)
        random.shuffle(temp_items)
        
        engine = Level2Engine(temp_container)
        engine.pack(temp_items)
        
        # Reward = % of total volume filled
        return (sum(it.volume() for it in temp_container.items) / self.base_container.volume()) * 100

    def _backpropagate(self, node: MCTSNode, reward: float):
        while node:
            node.visits += 1
            node.total_reward += reward
            node = node.parent

    def _expand(self, node: MCTSNode) -> MCTSNode:
        # Choose an action (item) not yet expanded
        tried_actions = {c.action for c in node.children}
        for i in range(len(node.items_remaining)):
            if i not in tried_actions:
                new_items = [it for j, it in enumerate(node.items_remaining) if i != j]
                # Pre-calculate a placement for the chosen item using Level 2 logic
                temp_c = copy.deepcopy(node.container)
                engine = Level2Engine(temp_c)
                engine.pack([copy.deepcopy(node.items_remaining[i])])
                
                child = MCTSNode(new_items, temp_c, parent=node)
                child.action = i
                node.children.append(child)
                return child
        return node

    def search(self) -> List[Item]:
        root = MCTSNode(self.items, self.base_container)
        start_time = time.time()

        while time.time() - start_time < self.time_limit:
            # 1. Selection
            curr = root
            while curr.is_fully_expanded() and curr.children:
                curr = curr.best_child()
            
            # 2. Expansion
            if curr.items_remaining:
                curr = self._expand(curr)
            
            # 3. Simulation (Rollout)
            reward = self._simulate(curr)
            
            # 4. Backpropagation
            self._backpropagate(curr, reward)

        # Final sequence: pick the most visited paths
        best_path_items = []
        curr = root
        # We'll just return the best next state or a greedy sequence based on search
        # To keep it simple for now, we'll run one last simulation from the root's best child path
        while curr.children:
            curr = max(curr.children, key=lambda c: c.visits)
            # Find the item added in this step
            if curr.parent:
                added_item = curr.parent.items_remaining[curr.action]
                best_path_items.append(added_item)
        
        # If the search didn't finish the sequence, append remaining
        packed_ids = {it.id for it in best_path_items}
        remaining = [it for it in self.items if it.id not in packed_ids]
        final_sequence = best_path_items + remaining
        
        # Actually pack it into a final container
        final_c = Container("MCTS_Result", self.base_container.width, self.base_container.height, self.base_container.depth)
        engine = Level2Engine(final_c)
        engine.pack(final_sequence)
        
        return final_c.items
