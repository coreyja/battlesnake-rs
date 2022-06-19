#![allow(dead_code)]

use std::collections::HashMap;

use battlesnake_game_types::{
    types::{Action, Move, SimulableGame, SnakeIDGettableGame},
    wire_representation::Game,
};
use battlesnake_minimax::Instruments;
use petgraph::{stable_graph::StableDiGraph, visit::EdgeRef};

struct GameManager;
impl GameManager {
    /// Here we can start the infinite minimax thread
    /// This thread needs to access the shared game state, and likely edit it but maybe only by
    /// adding scores
    ///
    /// The thread should look at the scores from other threads on the shares tree state, and use
    /// that to decide whether the sub-tree is worth exploring or not.
    fn start_game() -> Self {
        todo!();
    }

    /// This function is called when a new Game JSON is available.
    /// This will likely come from the battlesnake engine has have decisions
    /// for each snake.
    ///
    /// We need to diff the last game with this one, to understand what each snake did
    /// With this info we can prune the tree of nodes that weren't chosen and focus our efforts
    /// on parts of the tree that are "valid"
    fn next_turn(&mut self, wire_game: Game) {
        todo!("{}", wire_game);
    }

    /// This function looks at the state of the tree and returns IMMEDIATELY
    /// the current best move. This function is not async and should not be looping for a duration,
    /// we will do that looping outside and only call this function when we want a result
    ///
    /// This will NOT have side-effects, such as prioritizing a portion of the tree
    fn move_for_turn(&self, turn: u32) -> Move {
        todo!("{}", turn);
    }
}

#[derive(Default, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Debug)]
struct ExpandedScore {
    turn: u32,
    depth: u32,
}

#[derive(Debug)]
struct Node<GameType> {
    game: GameType,
    expanded: ExpandedScore,
}

type GameTreeIndexType = u32;
type NodeIndex = petgraph::graph::NodeIndex<GameTreeIndexType>;
type EdgeIndex = petgraph::graph::EdgeIndex<GameTreeIndexType>;

#[derive(Debug)]
struct GameTree<GameType, const MAX_SNAKES: usize>
where
    GameType: SnakeIDGettableGame + std::fmt::Debug,
{
    graph: StableDiGraph<Node<GameType>, Action<MAX_SNAKES>, GameTreeIndexType>,
    current_root_and_turn: (NodeIndex, usize),
    id_map: HashMap<String, GameType::SnakeIDType>,
}

#[derive(Debug)]
enum ExpandError {
    AlreadyExpanded,
    GameIsOver,
    NodeNotFound,
}

enum AddScoreError {
    NodeNotFound,
}

impl<
        GameType: SnakeIDGettableGame + SimulableGame<Instruments, MAX_SNAKES>,
        const MAX_SNAKES: usize,
    > GameTree<GameType, MAX_SNAKES>
{
    pub fn new(
        root_game: GameType,
        id_map: HashMap<String, GameType::SnakeIDType>,
        turn: usize,
    ) -> Self {
        let mut graph = StableDiGraph::new();
        let root = graph.add_node(Node {
            game: root_game,
            expanded: Default::default(),
        });

        Self {
            graph,
            current_root_and_turn: (root, turn),
            id_map,
        }
    }

    pub fn expand_node(
        &mut self,
        node_index: NodeIndex,
    ) -> Result<Vec<(Action<MAX_SNAKES>, NodeIndex, EdgeIndex)>, ExpandError> {
        if self.graph.edges(node_index).count() > 0 {
            return Err(ExpandError::AlreadyExpanded);
        }

        let node = self
            .graph
            .node_weight(node_index)
            .ok_or(ExpandError::NodeNotFound)?;

        let snake_ids = node.game.get_snake_ids();

        // This collect is NOT needless. If we chain iterators we end up borrowing the graph for a
        // mutable and immutable reference at the same time and failing to compile. By collecting
        // to a vec in the middle we don't have lifetime issues
        #[allow(clippy::needless_collect)]
        let new_nodes: Vec<_> = node
            .game
            .simulate(&Instruments {}, snake_ids)
            .map(|(action, new_game)| {
                (
                    action,
                    Node {
                        game: new_game,
                        expanded: Default::default(),
                    },
                )
            })
            .collect();

        let action_and_indexes: Vec<_> = new_nodes
            .into_iter()
            .map(|(action, node)| {
                let new_node_index = self.graph.add_node(node);
                let edge_index = self.graph.add_edge(node_index, new_node_index, action);

                (action, node_index, edge_index)
            })
            .collect();

        Ok(action_and_indexes)
    }

    pub fn move_for_current_turn(&self) -> Result<Move, &'static str> {
        let (root_index, _turn) = &self.current_root_and_turn;

        let best_edge = self
            .graph
            .edges(*root_index)
            .max_by_key(|edge| self.graph[edge.target()].expanded)
            .ok_or("Game is over")?;

        Ok(best_edge.weight().own_move())
    }
}

mod expand_minimax {
    use std::cmp::Reverse;

    use battlesnake_game_types::types::{SnakeId, VictorDeterminableGame};

    use super::*;

    fn expand_tree<
        GameType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + SimulableGame<Instruments, MAX_SNAKES>
            + VictorDeterminableGame,
        const MAX_SNAKES: usize,
    >(
        graph: &mut GameTree<GameType, MAX_SNAKES>,
    ) -> Result<(), ExpandError> {
        let mut current_depth = 1;
        let current = graph.current_root_and_turn.0;

        while current_depth < 7 {
            expand_tree_till(graph, current, 0, current_depth)?;

            current_depth += 1;
        }

        Ok(())
    }

    type Depth = u32;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum ExpandScore {
        Lose(Reverse<Depth>),
        Scored(Depth),
        Tie(Depth),
        Win(Depth),
    }

    impl Default for ExpandScore {
        fn default() -> Self {
            ExpandScore::Scored(0)
        }
    }

    fn expand_tree_till<
        GameType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + SimulableGame<Instruments, MAX_SNAKES>
            + VictorDeterminableGame,
        const MAX_SNAKES: usize,
    >(
        graph: &mut GameTree<GameType, MAX_SNAKES>,
        current: NodeIndex,
        depth: Depth,
        max_depth: Depth,
    ) -> Result<ExpandScore, ExpandError> {
        let current_node = graph
            .graph
            .node_weight(current)
            .ok_or(ExpandError::NodeNotFound)?;

        if depth == max_depth || current_node.game.is_over() {
            if current_node.game.is_over() {
                match current_node.game.get_winner() {
                    Some(SnakeId(0)) => {
                        return Ok(ExpandScore::Win(depth));
                    }
                    Some(_) => {
                        return Ok(ExpandScore::Lose(Reverse(depth)));
                    }
                    None => {
                        return Ok(ExpandScore::Tie(depth));
                    }
                }
            }

            return Ok(ExpandScore::Scored(depth));
        }

        if !current_node.game.is_over() && graph.graph.edges(current).next().is_none() {
            graph.expand_node(current)?;
        }

        let mut best_scores: [ExpandScore; 4] = Default::default();
        let mut walker = graph.graph.neighbors(current).detach();
        while let Some((edge, neighbor)) = walker.next(&graph.graph) {
            let weight = graph.graph[edge];
            let my_move = weight.own_move();

            let recursed_score = expand_tree_till(graph, neighbor, depth + 1, max_depth)?;

            // This is the minimizing
            best_scores[my_move.as_index()] = best_scores[my_move.as_index()].min(recursed_score);
        }

        // Here we can maximize
        let best_score = best_scores.iter().max().unwrap();

        Ok(*best_score)
    }

    #[cfg(test)]
    mod tests {
        use battlesnake_game_types::{
            compact_representation::{dimensions::ArcadeMaze, WrappedCellBoard},
            types::build_snake_id_map,
        };

        use super::*;

        #[test]
        fn test_expand_tree_node_counts() {
            let fixture = include_str!("../fixtures/arcade_2.json");
            let wire: Game = serde_json::from_str(fixture).unwrap();
            let id_map = build_snake_id_map(&wire);
            let game = WrappedCellBoard::<u16, ArcadeMaze, { 19 * 21 }, 4>::convert_from_game(
                wire, &id_map,
            )
            .unwrap();
            let mut game_tree = GameTree::new(game, id_map, 0);

            let current_root = game_tree.current_root_and_turn.0;

            let expected_node_counts = [1, 2, 3, 4, 5, 6, 8, 10, 15, 30];
            for (current_depth, expected_node_count) in expected_node_counts.iter().enumerate() {
                expand_tree_till(&mut game_tree, current_root, 0, current_depth as u32).unwrap();

                assert_eq!(
                    game_tree.graph.node_count(),
                    *expected_node_count,
                    "Depth {}",
                    current_depth
                );
            }
        }
    }
}
