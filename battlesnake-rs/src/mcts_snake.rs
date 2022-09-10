use std::{
    borrow::Cow,
    cell::RefCell,
    convert::TryInto,
    fs::OpenOptions,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
};

use atomic_float::AtomicF64;
use battlesnake_minimax::paranoid::WrappedScorable;
use decorum::{Infinite, Real, N64};
use dotavious::{Dot, Edge, GraphBuilder};
use itertools::Itertools;
use rand::prelude::ThreadRng;
use tracing::info;
pub use typed_arena::Arena;
use types::{
    compact_representation::WrappedCellBoard4Snakes11x11, wire_representation::NestedGame,
};

use crate::flood_fill::spread_from_head_arcade_maze::{Scores, SpreadFromHead};

use super::*;

pub struct MctsSnake<T> {
    game: T,
    game_info: NestedGame,
}

impl<T> MctsSnake<T> {
    pub fn new(game: T, game_info: NestedGame) -> Self {
        Self { game, game_info }
    }
}

pub struct MctsSnakeFactory;

impl BattlesnakeFactory for MctsSnakeFactory {
    fn name(&self) -> String {
        "mcts".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let id_map = build_snake_id_map(&game);

        if game_info.ruleset.name == "wrapped" {
            let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(game, game_info);

            Box::new(snake)
        } else {
            let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(game, game_info);

            Box::new(snake)
        }
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#fc0398".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            ..Default::default()
        }
    }
}

impl<BoardType> MctsSnake<BoardType>
where
    BoardType: Clone
        + SimulableGame<Instrument, 4>
        + PartialEq
        + RandomReasonableMovesGame
        + VictorDeterminableGame
        + YouDeterminableGame
        + 'static,
    BoardType: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame<SnakeIDType = SnakeId>
        + RandomReasonableMovesGame
        + SpreadFromHead<u8, 4>
        + Clone
        + VictorDeterminableGame
        + HazardQueryableGame
        + YouDeterminableGame,
{
    #[tracing::instrument(
        level = "info",
        skip_all,
        fields(total_number_of_iterations, total_score,)
    )]
    fn mcts<'arena>(
        &self,
        while_condition: &dyn Fn(&Node<BoardType>, usize) -> bool,
        arena: &'arena mut Arena<Node<'arena, BoardType>>,
    ) -> &'arena Node<'arena, BoardType> {
        let current_span = tracing::Span::current();

        let mut rng = rand::thread_rng();

        let cloned = self.game.clone();
        let root_node: &mut Node<BoardType> = arena.alloc(Node::new(cloned));

        root_node.expand(arena);

        let mut total_number_of_iterations = 0;

        while while_condition(root_node, total_number_of_iterations) {
            total_number_of_iterations += 1;

            let mut next_leaf_node = root_node.next_leaf_node(total_number_of_iterations);

            next_leaf_node = {
                // If next_leaf_node HAS been visited, then we expand it
                if next_leaf_node.number_of_visits.load(Ordering::Relaxed) > 0
                    && !next_leaf_node.has_been_expanded()
                {
                    next_leaf_node.expand(arena);

                    next_leaf_node.next_leaf_node(total_number_of_iterations)
                } else {
                    next_leaf_node
                }
            };

            //Now we do a simulation for this leaf node
            let score = next_leaf_node.simulate(&mut rng);

            //We now need to backpropagate the score
            next_leaf_node.backpropagate(score);
        }

        // We are outside the loop now and need to pick the best move
        current_span.record("total_number_of_iterations", &total_number_of_iterations);
        current_span.record(
            "total_score",
            &root_node.total_score.load(Ordering::Relaxed),
        );

        root_node
    }

    pub fn mcts_bench<'arena>(
        &self,
        max_iterations: usize,
        arena: &'arena mut Arena<Node<'arena, BoardType>>,
    ) -> &'arena Node<'arena, BoardType> {
        let while_condition = |_root: &Node<BoardType>, total_number_of_iterations: usize| {
            total_number_of_iterations < max_iterations
        };

        self.mcts(&while_condition, arena)
    }

    pub fn graph_move<'arena>(
        &self,
        arena: &'arena mut Arena<Node<'arena, BoardType>>,
    ) -> Result<MoveOutput> {
        info!(player_count =? self.game.get_snake_ids(), "Graphing MCTS");
        let start = std::time::Instant::now();

        const NETWORK_LATENCY_PADDING: i64 = 100;
        let max_duration = self.game_info.timeout - NETWORK_LATENCY_PADDING;

        let while_condition = |root_node: &Node<BoardType>, total_number_of_iterations: usize| {
            if total_number_of_iterations % 64 == 0 {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(format!("/Users/coreyja/Projects/battlesnake-rs/tmp/iteration_{total_number_of_iterations}.dot"))
                    .unwrap();
                file.write_all(
                    format!("{}", root_node.graph(total_number_of_iterations)).as_bytes(),
                )
                .unwrap();
            }

            start.elapsed().as_millis() < max_duration.try_into().unwrap()
        };

        let root_node = self.mcts(&while_condition, arena);

        let best_child = root_node
            .highest_average_score_child()
            .expect("The root should have a child");
        let chosen_move = best_child
            .tree_context
            .as_ref()
            .expect("We found the best child of the root node, so it _should_ have a tree_context")
            .r#move;

        Ok(MoveOutput {
            r#move: format!("{}", chosen_move),
            shout: None,
        })
    }
}

impl<BoardType> BattlesnakeAI for MctsSnake<BoardType>
where
    BoardType: Clone
        + SimulableGame<Instrument, 4>
        + PartialEq
        + RandomReasonableMovesGame
        + VictorDeterminableGame
        + YouDeterminableGame
        + 'static,
    BoardType: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame<SnakeIDType = SnakeId>
        + RandomReasonableMovesGame
        + SpreadFromHead<u8, 4>
        + Clone
        + VictorDeterminableGame
        + HazardQueryableGame
        + YouDeterminableGame,
{
    fn make_move(&self) -> Result<MoveOutput> {
        let start = std::time::Instant::now();

        const NETWORK_LATENCY_PADDING: i64 = 100;
        let max_duration = self.game_info.timeout - NETWORK_LATENCY_PADDING;

        let while_condition = |_root_node: &Node<BoardType>, _total_number_of_iterations: usize| {
            start.elapsed().as_millis() < max_duration.try_into().unwrap()
        };

        let mut arena = Arena::new();
        let root_node = self.mcts(&while_condition, &mut arena);

        let best_child = root_node
            .highest_average_score_child()
            .expect("The root should have a child");
        let chosen_move = best_child
            .tree_context
            .as_ref()
            .expect("We found the best child of the root node, so it _should_ have a tree_context")
            .r#move;

        Ok(MoveOutput {
            r#move: format!("{}", chosen_move),
            shout: None,
        })
    }

    fn end(&self) {
        info!("Mcts has ended");
    }
}

#[derive(Debug)]
struct TreeContext<'arena, T> {
    parent: RefCell<&'arena Node<'arena, T>>,
    r#move: Move,
}

#[derive(Debug)]
pub struct Node<'arena, T> {
    game_state: T,
    total_score: AtomicF64,
    number_of_visits: AtomicUsize,
    children: RefCell<Option<Vec<&'arena Node<'arena, T>>>>,
    tree_context: Option<TreeContext<'arena, T>>,
}

#[derive(Debug)]
pub struct Instrument {}
impl SimulatorInstruments for Instrument {
    fn observe_simulation(&self, _duration: std::time::Duration) {
        //No-oping here
    }
}

impl<'arena, T> Node<'arena, T> {
    fn new(game_state: T) -> Self {
        Self {
            game_state,
            total_score: AtomicF64::new(0.0),
            number_of_visits: AtomicUsize::new(0),
            children: RefCell::new(None),
            tree_context: None,
        }
    }

    fn new_with_parent(game_state: T, parent: &'arena Self, r#move: Move) -> Self {
        Self {
            game_state,
            total_score: AtomicF64::new(0.0),
            number_of_visits: AtomicUsize::new(0),
            children: RefCell::new(None),
            tree_context: Some(TreeContext {
                parent: RefCell::new(parent),
                r#move,
            }),
        }
    }
}

impl<'arena, BoardType> WrappedScorable<BoardType, N64> for Node<'arena, BoardType>
where
    BoardType: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame<SnakeIDType = SnakeId>
        + RandomReasonableMovesGame
        + SpreadFromHead<u8, 4>
        + Clone
        + VictorDeterminableGame
        + HazardQueryableGame
        + YouDeterminableGame,
{
    fn score(&self, node: &BoardType) -> N64 {
        let scores = if node.get_hazard_damage().is_positive() {
            Scores {
                food: 5,
                hazard: 1,
                empty: 5,
            }
        } else {
            Scores {
                food: 5,
                hazard: 5,
                empty: 1,
            }
        };

        let me = node.you_id();

        if node.is_over() {
            match node.get_winner() {
                Some(sid) => {
                    if &sid == me {
                        1.0
                    } else {
                        -1.0
                    }
                }
                None => 0.0,
            }
            .into()
        } else {
            let square_counts = node.squares_per_snake_with_scores(5, scores);

            let my_space: f64 = square_counts[me.as_usize()] as f64;
            let total_space: f64 = square_counts.iter().sum::<u16>() as f64;

            N64::from(my_space / total_space)
        }
    }
}

impl<'arena, BoardType> Node<'arena, BoardType>
where
    BoardType: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame
        + RandomReasonableMovesGame
        + Clone
        + VictorDeterminableGame
        + YouDeterminableGame,
    Node<'arena, BoardType>: WrappedScorable<BoardType, N64>,
{
    fn simulate(&self, rng: &mut ThreadRng) -> N64 {
        let mut current_state: Cow<BoardType> = Cow::Borrowed(&self.game_state);
        let mut number_of_iterations = 0;

        while number_of_iterations < 100 && !current_state.is_over() {
            number_of_iterations += 1;

            let random_moves = current_state
                .random_reasonable_move_for_each_snake(rng)
                .map(|(sid, mv)| (sid, [mv]));

            let next_state = {
                let mut simulation_result =
                    current_state.simulate_with_moves(&Instrument {}, random_moves);

                // TODO: This unwrap might NOT be safe
                simulation_result.next().unwrap().1
            };

            current_state = Cow::Owned(next_state);
        }

        self.score(current_state.as_ref())
    }

    fn has_been_expanded(&self) -> bool {
        self.children.borrow().is_some()
    }

    fn ucb1_score(&self, total_number_of_iterations: usize) -> N64 {
        let constant: N64 = 2.0.into();

        // TODO: This should be fine when we are single threaded
        // But if/when we get to multi-threaded, we might want to think about if this wants
        // to use the same visits value like this.
        // Or do we need to re-load it for each usage?
        let number_of_visits = self.number_of_visits.load(Ordering::Relaxed);
        let total_score = self.total_score.load(Ordering::Relaxed);
        let total_score: N64 = total_score.into();

        if number_of_visits == 0 {
            return N64::INFINITY;
        }

        let number_of_visits = number_of_visits as f64;
        let number_of_visits: N64 = number_of_visits.into();

        let average_score = total_score / number_of_visits;
        let total_number_of_iterations: N64 = (total_number_of_iterations as f64).into();

        let ln_total_number_of_iterations = total_number_of_iterations.ln();

        let right_hand_side = constant * (ln_total_number_of_iterations / number_of_visits).sqrt();

        average_score + right_hand_side
    }

    fn average_score(&self) -> Option<f64> {
        let number_of_visits = self.number_of_visits.load(Ordering::Relaxed);
        let total_score = self.total_score.load(Ordering::Relaxed);

        if number_of_visits == 0 {
            return None;
        }

        let number_of_visits = number_of_visits as f64;

        let average_score = total_score / number_of_visits;
        Some(average_score)
    }

    fn next_leaf_node(
        &'arena self,
        total_number_of_iterations: usize,
    ) -> &'arena Node<'arena, BoardType> {
        let mut best_node: &'arena Node<'arena, BoardType> = self;

        while best_node.has_been_expanded() {
            if let Some(next) = best_node.next_child_to_explore(total_number_of_iterations) {
                best_node = next;
            } else {
                break;
            }
        }

        best_node
    }

    fn next_child_to_explore(
        &self,
        total_number_of_iterations: usize,
    ) -> Option<&'arena Node<BoardType>> {
        debug_assert!(self.has_been_expanded());
        let borrowed = self.children.borrow();
        let children = borrowed
            .as_ref()
            .expect("We debug asserts that we are expanded already");

        children
            .iter()
            .cloned()
            .max_by_key(|child| child.ucb1_score(total_number_of_iterations))
    }

    fn highest_average_score_child(&self) -> Option<&'arena Node<BoardType>> {
        debug_assert!(self.has_been_expanded());
        let borrowed = self.children.borrow();
        let children = borrowed
            .as_ref()
            .expect("We debug asserts that we are expanded already");

        children
            .iter()
            .cloned()
            .max_by_key(|child| child.average_score().map(N64::from))
    }

    fn expand(&'arena self, arena: &'arena Arena<Node<'arena, BoardType>>) {
        debug_assert!(!self.has_been_expanded());

        if self.game_state.is_over() {
            self.children.replace(Some(vec![]));

            return;
        }

        let snakes = self.game_state.get_snake_ids();

        let next_states = self.game_state.simulate(&Instrument {}, snakes);

        let mut opponent_moves: [Option<Vec<(OtherAction<4>, BoardType)>>; 4] = Default::default();

        for (actions, game_state) in next_states {
            let own_move = actions.own_move();
            if opponent_moves[own_move.as_index()].is_none() {
                opponent_moves[own_move.as_index()] = Some(vec![]);
            }
            opponent_moves[own_move.as_index()]
                .as_mut()
                .unwrap()
                .push((actions.other_moves(), game_state));
        }

        let mut children: Vec<&'arena _> = Vec::with_capacity(4);
        for (own_move, next_states) in opponent_moves
            .into_iter()
            .enumerate()
            .filter_map(|(own_move, next_states)| next_states.map(|n| (own_move, n)))
        {
            let r#move = Move::from_index(own_move);
            // TODO: Passing `game_state` here is WRONG
            // Really self move nodes can't have a game state, since it depends on the opponent
            // moves too. We are keeping the 'old' one around here since our types can't model
            // the real shape of the tree
            let new_node: &'arena _ =
                arena.alloc(Node::new_with_parent(self.game_state.clone(), self, r#move));
            children.push(new_node);

            let new_node_children: Vec<&'arena _> = next_states
                .into_iter()
                .map(|next_state| {
                    let newer_node =
                        arena.alloc(Node::new_with_parent(next_state.1, new_node, r#move));

                    &*newer_node
                })
                .collect_vec();

            new_node.children.replace(Some(new_node_children));
        }

        debug_assert!(self.children.borrow().is_none());

        self.children.replace(Some(children));
    }

    fn backpropagate(&self, score: N64) {
        self.number_of_visits.fetch_add(1, Ordering::Relaxed);
        self.total_score.fetch_add(score.into(), Ordering::Relaxed);

        if let Some(tree_context) = &self.tree_context {
            tree_context.parent.borrow().backpropagate(score)
        }
    }

    fn graph(&self, total_number_of_iterations: usize) -> Dot {
        let mut builder = GraphBuilder::new_named_directed("example");
        self.graph_with(&mut builder, 0, vec![], total_number_of_iterations);

        let graph = builder.build().unwrap();
        Dot { graph }
    }

    // Takes in a builder and adds itself and all children as nodes in the graph
    // Returns a string that corresponds to the name of the current node
    fn graph_with<'a>(
        &self,
        builder: &mut GraphBuilder<'a>,
        depth: usize,
        child_id: Vec<usize>,
        total_number_of_iterations: usize,
    ) -> String {
        let me_id: String = format!(
            "Depth: {depth}\nChild ID: {:?}\nMove: {:?}\nTotal Score: {:?}\nVisits: {:?}\nUCB1: {}\nAvg Score: {:?}",
            child_id,
            self.tree_context.as_ref().map(|t| t.r#move),
            self.total_score,
            self.number_of_visits,
            self.ucb1_score(total_number_of_iterations),
            self.average_score(),
        );

        builder.add_node(dotavious::Node::new(me_id.as_str()));

        let borrow = self.children.borrow();
        let children = borrow.as_ref();

        if let Some(children) = children {
            for (i, child) in children.iter().enumerate() {
                let mut new_child_id = child_id.clone();
                new_child_id.push(i);
                let child_id =
                    child.graph_with(builder, depth + 1, new_child_id, total_number_of_iterations);

                builder.add_edge(Edge::new(me_id.as_str(), child_id.as_str()));
            }
        }

        me_id
    }
}

#[cfg(test)]
mod test {

    use decorum::Infinite;
    use itertools::Itertools;
    use types::compact_representation::standard::CellBoard4Snakes11x11;

    use super::*;

    #[test]
    fn test_ucb1_empty_score() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let n = Node::new(game);

        assert_eq!(n.ucb1_score(1), N64::INFINITY);
        assert_eq!(n.ucb1_score(0), N64::INFINITY);
    }

    #[test]
    fn test_ucb1_non_empty() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let n = Node::new(game);
        n.number_of_visits.store(1, Ordering::Relaxed);
        n.total_score.store(10.0, Ordering::Relaxed);

        assert_eq!(n.ucb1_score(1), 10.0);
        assert!(n.ucb1_score(2) > 11.6);
        assert!(n.ucb1_score(2) < 11.7);
    }

    #[test]
    fn test_average_empty_score() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let n = Node::new(game);

        assert_eq!(n.average_score(), None);
    }

    #[test]
    fn test_average_non_empty() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let n = Node::new(game);
        n.number_of_visits.store(1, Ordering::Relaxed);
        n.total_score.store(10.0, Ordering::Relaxed);

        assert_eq!(n.average_score(), Some(10.0));

        n.number_of_visits.store(2, Ordering::Relaxed);
        n.total_score.store(25.0, Ordering::Relaxed);

        assert_eq!(n.average_score(), Some(12.5));
    }

    #[test]
    fn test_backpropagate_root() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let n = Node::new(game);

        n.backpropagate(10.0.into());

        assert_eq!(n.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(n.total_score.load(Ordering::Relaxed), 10.0);
    }

    #[test]
    fn test_backpropagate_first_child() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let root = Node::new(game);

        let child = Node::new_with_parent(game, &root, Move::Up);

        child.backpropagate(10.0.into());

        assert_eq!(child.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(child.total_score.load(Ordering::Relaxed), 10.0);

        assert_eq!(root.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(root.total_score.load(Ordering::Relaxed), 10.0);

        let other_child = Node::new_with_parent(game, &root, Move::Down);
        other_child.backpropagate(20.0.into());

        assert_eq!(other_child.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(other_child.total_score.load(Ordering::Relaxed), 20.0);

        assert_eq!(root.number_of_visits.load(Ordering::Relaxed), 2);
        assert_eq!(root.total_score.load(Ordering::Relaxed), 30.0);
    }

    #[test]
    fn test_board_repr() {
        // This test was a sanity check to make sure the Board knew I died when running into my
        // tail
        // For somet reason MCTS is sometimes saying thats the best move, even though its instant
        // death for me
        let fixture = include_str!("../fixtures/check_board_doubled_up.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let you_id = game.you_id();
        let body = game.get_snake_body_vec(you_id);

        assert_eq!(body.len(), 5);

        let other_id = game
            .get_snake_ids()
            .into_iter()
            .find(|&id| id != *you_id)
            .unwrap();

        let result: Vec<_> = game
            .simulate_with_moves(
                &Instrument {},
                [(*you_id, vec![Move::Up]), (other_id, vec![Move::Down])],
            )
            .collect();
        assert_eq!(result.len(), 1);
        let (_, new_state) = result[0];

        assert!(new_state.is_over());
        assert_eq!(new_state.get_winner(), Some(other_id));
    }

    fn test_fixture(fixture: &'static str, allowed_moves: Vec<Move>) {
        let game = serde_json::from_str::<Game>(fixture).unwrap();

        let game_info = game.game.clone();
        let id_map = build_snake_id_map(&game);
        let max_duration = game_info.timeout - NETWORK_LATENCY_PADDING;

        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let snake = MctsSnake::new(game, game_info);

        let start = std::time::Instant::now();

        const NETWORK_LATENCY_PADDING: i64 = 400;

        let while_condition = |_root_node: &Node<_>, _total_number_of_iterations: usize| {
            start.elapsed().as_millis() < max_duration.try_into().unwrap()
        };
        let mut arena = Arena::new();
        let root_node = snake.mcts(&while_condition, &mut arena);

        let best_child = root_node
            .highest_average_score_child()
            .expect("The root should have a child");
        let chosen_move = best_child
            .tree_context
            .as_ref()
            .expect("We found the best child of the root node, so it _should_ have a tree_context")
            .r#move;

        let borrowed = root_node.children.borrow();
        let children = borrowed.as_ref().unwrap();
        dbg!(children
            .iter()
            .map(|n| (
                n.average_score(),
                n.number_of_visits.load(Ordering::Relaxed),
                n.tree_context.as_ref().unwrap().r#move,
                n.children
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|n| (
                        n.average_score(),
                        n.number_of_visits.load(Ordering::Relaxed),
                        n.tree_context.as_ref().unwrap().r#move,
                    ))
                    .collect_vec()
            ))
            .collect_vec());

        assert!(
            allowed_moves.contains(&chosen_move),
            "{chosen_move} was not in the allowed set of moves: {allowed_moves:?}"
        );
    }

    #[test]
    fn test_move_45e7de53_bca5_4fa3_8771_d9914ed141bb() {
        let fixture = include_str!("../../fixtures/45e7de53-bca5-4fa3-8771-d9914ed141bb.json");

        test_fixture(fixture, vec![Move::Right]);
    }

    #[test]
    fn test_move_65401e8f_a92a_445f_9617_94770044e117() {
        let fixture = include_str!("../../fixtures/65401e8f-a92a-445f-9617-94770044e117.json");

        test_fixture(fixture, vec![Move::Right, Move::Left, Move::Down]);
    }
    #[test]
    fn test_move_df732ab7_7e22_41d8_b651_95bb912e45ab() {
        let fixture = include_str!("../../fixtures/df732ab7-7e22-41d8-b651-95bb912e45ab.json");

        test_fixture(fixture, vec![Move::Right, Move::Up, Move::Down]);
    }

    #[test]
    fn test_move_d9841bf6_c34f_42fb_8818_dfd5d5a09b4a_125() {
        let fixture = include_str!("../../fixtures/d9841bf6-c34f-42fb-8818-dfd5d5a09b4a_125.json");
        // let game = serde_json::from_str::<Game>(fixture).unwrap();

        // let game_info = game.game.clone();
        // let id_map = build_snake_id_map(&game);

        // let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        // let snake = MctsSnake::new(game, game_info);

        // let mut arena = Arena::new();
        // snake.graph_move(&mut arena).unwrap();
        test_fixture(fixture, vec![Move::Right, Move::Up, Move::Left]);
    }

    #[test]
    fn test_move_af943832_1b3b_4795_9e35_081f71959aee_108() {
        let fixture = include_str!("../../fixtures/af943832-1b3b-4795-9e35-081f71959aee_108.json");

        test_fixture(fixture, vec![Move::Right]);
    }
}
