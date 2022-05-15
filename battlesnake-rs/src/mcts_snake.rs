use std::{
    borrow::Cow,
    cell::RefCell,
    convert::TryInto,
    sync::atomic::{AtomicUsize, Ordering},
};

use atomic_float::AtomicF64;
use battlesnake_game_types::{
    compact_representation::WrappedCellBoard4Snakes11x11, wire_representation::NestedGame,
};
use decorum::N64;
use rand::prelude::ThreadRng;
use tracing::info;
use typed_arena::Arena;

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

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
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

impl<
        T: Clone
            + SimulableGame<Instrument, 4>
            + PartialEq
            + RandomReasonableMovesGame
            + VictorDeterminableGame
            + YouDeterminableGame
            + 'static,
    > BattlesnakeAI for MctsSnake<T>
{
    #[tracing::instrument(
        level = "info",
        skip_all,
        fields(total_number_of_iterations, total_score,)
    )]
    fn make_move(&self) -> Result<MoveOutput> {
        let current_span = tracing::Span::current();

        let arena = Arena::new();

        let mut rng = rand::thread_rng();

        let root_node: &mut Node<T> = arena.alloc(Node::new(self.game.clone()));

        root_node.expand(&arena);

        let mut total_number_of_iterations = 0;

        let start = std::time::Instant::now();

        const NETWORK_LATENCY_PADDING: i64 = 100;
        let max_duration = self.game_info.timeout - NETWORK_LATENCY_PADDING;

        while start.elapsed().as_millis() < max_duration.try_into().unwrap() {
            total_number_of_iterations += 1;

            let mut next_leaf_node = root_node.next_leaf_node(total_number_of_iterations);

            next_leaf_node = {
                let borrowed = next_leaf_node;

                // If next_leaf_node HAS been visited, then we expand it
                if borrowed.number_of_visits.load(Ordering::Relaxed) > 0 {
                    borrowed.expand(&arena);

                    borrowed.next_leaf_node(total_number_of_iterations)
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

        let best_child = root_node.highest_average_score_child();
        let chosen_move = best_child
            .expect("The root should have a child")
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
struct Node<'arena, T> {
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

impl<'arena, T> Node<'arena, T>
where
    T: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame
        + RandomReasonableMovesGame
        + Clone
        + VictorDeterminableGame
        + YouDeterminableGame,
{
    fn simulate(&self, rng: &mut ThreadRng) -> f64 {
        let mut current_state: Cow<T> = Cow::Borrowed(&self.game_state);

        while !current_state.is_over() {
            let random_moves: Vec<_> = current_state
                .random_reasonable_move_for_each_snake(rng)
                .map(|(sid, mv)| (sid, [mv]))
                .collect();

            let next_state = {
                let mut simulation_result =
                    current_state.simulate_with_moves(&Instrument {}, random_moves);

                // TODO: This unwrap might NOT be safe
                simulation_result.next().unwrap().1
            };

            current_state = Cow::Owned(next_state);
        }

        let you_id = current_state.you_id();
        match current_state.get_winner() {
            Some(sid) => {
                if &sid == you_id {
                    1.0
                } else {
                    -1.0
                }
            }
            None => 0.0,
        }
    }

    fn has_been_expanded(&self) -> bool {
        self.children.borrow().is_some()
    }

    fn ucb1_score(&self, total_number_of_iterations: usize) -> f64 {
        let constant = 2.0;

        // TODO: This should be fine when we are single threaded
        // But if/when we get to multi-threaded, we might want to think about if this wants
        // to use the same visits value like this.
        // Or do we need to re-load it for each usage?
        let number_of_visits = self.number_of_visits.load(Ordering::Relaxed);
        let total_score = self.total_score.load(Ordering::Relaxed);

        if number_of_visits == 0 {
            return f64::INFINITY;
        }

        let number_of_visits = number_of_visits as f64;

        let average_score = total_score / number_of_visits;
        let total_number_of_iterations = total_number_of_iterations as f64;

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

    fn next_leaf_node(&'arena self, total_number_of_iterations: usize) -> &'arena Node<'arena, T> {
        let mut best_node: &'arena Node<'arena, T> = self;

        while !best_node.is_leaf() {
            debug_assert!(best_node.has_been_expanded());

            let next = best_node
                .next_child_to_explore(total_number_of_iterations)
                .expect("We are not a leaf node so we should have a best child");
            best_node = next;
        }

        best_node
    }

    fn is_leaf(&self) -> bool {
        self.children.borrow().is_none() || self.children.borrow().as_ref().unwrap().is_empty()
    }

    fn next_child_to_explore(&self, total_number_of_iterations: usize) -> Option<&'arena Node<T>> {
        debug_assert!(self.has_been_expanded());
        let borrowed = self.children.borrow();
        let children = borrowed.as_ref().unwrap();

        children
            .iter()
            .cloned()
            .max_by_key(|child| N64::from(child.ucb1_score(total_number_of_iterations)))
    }

    fn highest_average_score_child(&self) -> Option<&'arena Node<T>> {
        let borrowed = self.children.borrow();
        let children = borrowed.as_ref().unwrap();

        children
            .iter()
            .cloned()
            .max_by_key(|child| child.average_score().map(N64::from))
    }

    fn expand(&'arena self, arena: &'arena Arena<Node<'arena, T>>) {
        debug_assert!(!self.has_been_expanded());

        if self.game_state.is_over() {
            return;
        }

        let snakes = self.game_state.get_snake_ids();

        let next_states = self.game_state.simulate(&Instrument {}, snakes);

        let allocated_nodes = arena.alloc_extend(next_states.map(|(actions, game_state)| {
            let r#move = actions.own_move();
            Node::new_with_parent(game_state, self, r#move)
        }));

        let children = allocated_nodes.iter().collect();

        debug_assert!(self.children.borrow().is_none());

        self.children.replace(Some(children));
    }

    fn backpropagate(&self, score: f64) {
        self.number_of_visits.fetch_add(1, Ordering::Relaxed);
        self.total_score.fetch_add(score, Ordering::Relaxed);

        if let Some(tree_context) = &self.tree_context {
            tree_context.parent.borrow().backpropagate(score)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ucb1_empty_score() {
        let fixture = include_str!("../fixtures/start_of_game.json");
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);
        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let n = Node::new(game);

        assert_eq!(n.ucb1_score(1), f64::INFINITY);
        assert_eq!(n.ucb1_score(0), f64::INFINITY);
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

        n.backpropagate(10.0);

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

        child.backpropagate(10.0);

        assert_eq!(child.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(child.total_score.load(Ordering::Relaxed), 10.0);

        assert_eq!(root.number_of_visits.load(Ordering::Relaxed), 1);
        assert_eq!(root.total_score.load(Ordering::Relaxed), 10.0);

        let other_child = Node::new_with_parent(game, &root, Move::Down);
        other_child.backpropagate(20.0);

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
}
