#![allow(dead_code)]

use std::collections::HashMap;

use battlesnake_game_types::{
    types::{Action, Move, SimulableGame, SnakeIDGettableGame},
    wire_representation::Game,
};
use battlesnake_minimax::Instruments;
use parking_lot::RwLock;
use petgraph::{stable_graph::StableDiGraph, visit::EdgeRef};

use self::expand_minimax::ExpandScore;

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

#[derive(Debug)]
struct Node<GameType> {
    game: GameType,
    expanded: Option<ExpandScore>,
}

type GameTreeIndexType = u32;
type NodeIndex = petgraph::graph::NodeIndex<GameTreeIndexType>;
type EdgeIndex = petgraph::graph::EdgeIndex<GameTreeIndexType>;

#[derive(Debug)]
struct GameTree<GameType, const MAX_SNAKES: usize>
where
    GameType: SnakeIDGettableGame + std::fmt::Debug,
{
    graph: RwLock<StableDiGraph<Node<GameType>, Action<MAX_SNAKES>, GameTreeIndexType>>,
    current_root_and_turn: (NodeIndex, usize),
    id_map: HashMap<String, GameType::SnakeIDType>,
}

#[derive(Debug, PartialEq, Eq)]
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
            graph: RwLock::new(graph),
            current_root_and_turn: (root, turn),
            id_map,
        }
    }

    pub fn expand_node(
        &self,
        node_index: NodeIndex,
    ) -> Result<Vec<(Action<MAX_SNAKES>, NodeIndex, EdgeIndex)>, ExpandError> {
        {
            let graph = self.graph.read();
            if graph.edges(node_index).count() > 0 {
                return Err(ExpandError::AlreadyExpanded);
            }
        }

        let new_nodes = {
            let graph = self.graph.read();
            let node = graph
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

            new_nodes
        };

        let action_and_indexes: Vec<_> = new_nodes
            .into_iter()
            .map(|(action, node)| {
                let (edge_index, new_node_index) = {
                    let mut graph = self.graph.write();
                    let new_node_index = graph.add_node(node);
                    let edge_index = graph.add_edge(node_index, new_node_index, action);

                    (edge_index, new_node_index)
                };

                (action, new_node_index, edge_index)
            })
            .collect();

        Ok(action_and_indexes)
    }

    pub fn move_for_current_turn(&self) -> Result<Move, &'static str> {
        let (root_index, _turn) = &self.current_root_and_turn;

        let graph = self.graph.read();
        let best_edge = graph
            .edges(*root_index)
            .max_by_key(|edge| self.graph.read()[edge.target()].expanded)
            .ok_or("Game is over")?;

        Ok(best_edge.weight().own_move())
    }
}

mod expand_minimax {
    use std::cmp::Reverse;

    use battlesnake_game_types::types::{SnakeId, VictorDeterminableGame};

    use super::*;

    enum InSubTree {
        True,
        False { last_parent: Option<NodeIndex> },
    }

    fn in_subtree<GameType: std::fmt::Debug + SnakeIDGettableGame, const MAX_SNAKES: usize>(
        graph: &GameTree<GameType, MAX_SNAKES>,
        parent_index: NodeIndex,
        potential_child_index: NodeIndex,
    ) -> InSubTree {
        let mut last = None;
        let mut current = Some(potential_child_index);

        while let Some(c) = current {
            if c == parent_index {
                return InSubTree::True;
            }

            let graph = graph.graph.read();
            let mut parents = graph.edges_directed(c, petgraph::EdgeDirection::Incoming);

            debug_assert!(
              parents.clone().count() <= 1,
              "There are more parents for this node than expected, thats strange as we should have a tree structure here",
            );

            last = current;
            current = parents.next().map(|e| e.source());
        }

        InSubTree::False { last_parent: last }
    }

    fn expand_tree_iterative_deepened<
        GameType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + SimulableGame<Instruments, MAX_SNAKES>
            + VictorDeterminableGame,
        const MAX_SNAKES: usize,
    >(
        graph: GameTree<GameType, MAX_SNAKES>,
    ) -> Result<!, RecurseError> {
        let mut current_depth = 1;
        let current = graph.current_root_and_turn.0;

        loop {
            expand_tree_recursive(&graph, current, 0, current_depth)?;

            current_depth += 1;
        }
    }

    type Depth = u32;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum ExpandScore {
        Lose(Reverse<Depth>),
        Scored(Depth),
        Tie(Depth),
        Win(Depth),
    }

    impl ExpandScore {
        const fn max() -> Self {
            ExpandScore::Win(u32::MAX)
        }

        const fn min() -> Self {
            ExpandScore::Lose(Reverse(u32::MAX))
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum RecurseError {
        NotInSubtree { last_parent: Option<NodeIndex> },
        NodeNotFound,
        ExpandError(ExpandError),
    }

    fn expand_tree_recursive<
        GameType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + SimulableGame<Instruments, MAX_SNAKES>
            + VictorDeterminableGame,
        const MAX_SNAKES: usize,
    >(
        game_tree: &GameTree<GameType, MAX_SNAKES>,
        current: NodeIndex,
        depth: Depth,
        max_depth: Depth,
    ) -> Result<ExpandScore, RecurseError> {
        if let InSubTree::False { last_parent } =
            in_subtree(game_tree, game_tree.current_root_and_turn.0, current)
        {
            return Err(RecurseError::NotInSubtree { last_parent });
        }
        let (is_over, winner) = {
            let graph = game_tree.graph.read();
            let current_node = graph
                .node_weight(current)
                .ok_or(RecurseError::NodeNotFound)?;

            (current_node.game.is_over(), current_node.game.get_winner())
        };

        if depth == max_depth || is_over {
            if is_over {
                match winner {
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

        let is_expanded = {
            let graph = game_tree.graph.read();
            graph.edges(current).next().is_none()
        };

        if !is_over && is_expanded {
            game_tree
                .expand_node(current)
                .map_err(RecurseError::ExpandError)?;
        }

        let mut best_scores: [Option<ExpandScore>; 4] = Default::default();
        let mut walker = game_tree.graph.read().neighbors(current).detach();
        while let Some((edge, neighbor)) = {
            let graph = game_tree.graph.read();
            walker.next(&graph)
        } {
            let weight = game_tree.graph.read()[edge];
            let my_move = weight.own_move();

            let recursed_score: Option<ExpandScore> =
                match expand_tree_recursive(game_tree, neighbor, depth + 1, max_depth) {
                    // If we get an error that this wasn't in the subtree, we don't want to count
                    // this edge in our search, so we mark it as Ok(None)
                    Err(RecurseError::NotInSubtree { last_parent }) => {
                        // TODO: Here we need to decide what to do with last parent. Our child was just not in the sub-tree.
                        // In some cases we want to ignore the error, but in most we probably want to propagate it.
                        //
                        // When we do want to keep running? We want to do that if....
                        // I think it is when last_parent == neighbor. Cause that means we _just_
                        // were cut off, and want to keep looking at the rest of the options.
                        // If the last_parent isn't where we are now, it _must_ be 'above' us in the
                        // recurse tree so we just forward the error along.
                        if last_parent == Some(neighbor) {
                            Ok(None)
                        } else {
                            Err(RecurseError::NotInSubtree { last_parent })?
                        }
                    }
                    Ok(score) => Ok(Some(score)),
                    Err(x) => Err(x),
                }?;

            if let Some(recursed_score) = recursed_score {
                let new_score = if let Some(best_score) = best_scores[my_move.as_index()] {
                    best_score.min(recursed_score)
                } else {
                    recursed_score
                };
                best_scores[my_move.as_index()] = Some(new_score);
            }
        }

        // Here we can maximize
        let best_score = best_scores.iter().filter_map(|x| *x).max().unwrap();

        {
            let mut graph = game_tree.graph.write();
            let current_node = graph
                .node_weight_mut(current)
                .ok_or(RecurseError::NodeNotFound)?;
            current_node.expanded = Some(best_score);
        }

        Ok(best_score)
    }

    #[cfg(test)]
    mod tests {
        use std::{thread, time::Duration};

        use battlesnake_game_types::{
            compact_representation::{
                dimensions::{ArcadeMaze, Fixed},
                StandardCellBoard, WrappedCellBoard,
            },
            types::build_snake_id_map,
        };
        use parking_lot::deadlock;

        use super::*;

        fn check_deadlock() {
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(10));
                let deadlocks = deadlock::check_deadlock();
                if deadlocks.is_empty() {
                    continue;
                }

                println!("{} deadlocks detected", deadlocks.len());
                for (i, threads) in deadlocks.iter().enumerate() {
                    println!("Deadlock #{}", i);
                    for t in threads {
                        println!("Thread Id {:#?}", t.thread_id());
                        println!("{:#?}", t.backtrace());
                    }
                }
            });
        }

        #[test]
        fn test_expand_tree_node_counts() {
            check_deadlock();

            let fixture = include_str!("../fixtures/arcade_2.json");
            let wire: Game = serde_json::from_str(fixture).unwrap();
            let id_map = build_snake_id_map(&wire);
            let game = WrappedCellBoard::<u16, ArcadeMaze, { 19 * 21 }, 4>::convert_from_game(
                wire, &id_map,
            )
            .unwrap();
            let game_tree = GameTree::new(game, id_map, 0);

            let current_root = game_tree.current_root_and_turn.0;

            let expected_node_counts = [1, 2, 3, 4, 5, 6, 8, 10, 15, 30];
            for (current_depth, expected_node_count) in expected_node_counts.iter().enumerate() {
                let r = expand_tree_recursive(&game_tree, current_root, 0, current_depth as u32);

                assert_eq!(r, Ok(ExpandScore::Scored(current_depth as u32)));

                assert_eq!(
                    game_tree.graph.read().node_count(),
                    *expected_node_count,
                    "Depth {}",
                    current_depth
                );
            }

            assert_eq!(
                game_tree
                    .graph
                    .read()
                    .node_weight(game_tree.current_root_and_turn.0)
                    .unwrap()
                    .expanded,
                Some(ExpandScore::Scored(9))
            );
        }

        #[test]
        fn test_cutting_off_tree_leaf() {
            check_deadlock();

            let fixture = include_str!("../fixtures/new_start.json");
            let wire: Game = serde_json::from_str(fixture).unwrap();
            let id_map = build_snake_id_map(&wire);
            let game = StandardCellBoard::<u16, Fixed<11, 11>, { 11 * 11 }, 4>::convert_from_game(
                wire, &id_map,
            )
            .unwrap();
            let mut game_tree = GameTree::new(game, id_map, 0);
            let current_root = game_tree.current_root_and_turn.0;

            expand_tree_recursive(&game_tree, current_root, 0, 2).unwrap();
            // Each snake has 3 possible moves on each turn, so 9 nodes then 81 nodes for 91 total
            // (including the root node)
            assert_eq!(game_tree.graph.read().node_count(), 91);

            let about_to_be_cut_off = game_tree
                .graph
                .read()
                .edges_directed(current_root, petgraph::EdgeDirection::Outgoing)
                .find(|x| {
                    x.weight() == &Action::new([Some(Move::Up), Some(Move::Left), None, None])
                })
                .unwrap()
                .target();

            assert_eq!(game_tree.graph.read().neighbors(current_root).count(), 9);

            // Remove all edges from the root node where I don't move right
            game_tree.graph.write().retain_edges(|graph, edge_index| {
                let edge_weight = graph[edge_index];
                let source = graph.edge_endpoints(edge_index).unwrap().0;

                source != current_root || edge_weight.own_move() == Move::Right
            });

            assert_eq!(game_tree.graph.read().neighbors(current_root).count(), 3);

            {
                let r = expand_tree_recursive(&mut game_tree, about_to_be_cut_off, 1, 3);
                assert_eq!(
                    r,
                    Err(RecurseError::NotInSubtree {
                        last_parent: Some(about_to_be_cut_off)
                    })
                );
            }

            // Confirm that even though we've called expand a few times we aren't actually adding
            // nodes
            assert_eq!(game_tree.graph.read().node_count(), 91);

            let mut walker = game_tree
                .graph
                .read()
                .neighbors(about_to_be_cut_off)
                .detach();
            while let Some((_, n)) = walker.next(&game_tree.graph.read()) {
                let r = expand_tree_recursive(&game_tree, n, 2, 3);

                assert_eq!(
                    r,
                    Err(RecurseError::NotInSubtree {
                        last_parent: Some(about_to_be_cut_off)
                    })
                );
            }

            // Confirm that even though we've called expand a few times we aren't actually adding
            // nodes
            assert_eq!(game_tree.graph.read().node_count(), 91);

            expand_tree_recursive(&mut game_tree, current_root, 0, 3).unwrap();

            // I think I counted this out right but we'll see
            assert_eq!(game_tree.graph.read().node_count(), 334);
        }
    }
}
