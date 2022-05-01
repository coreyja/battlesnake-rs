use battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11;
use decorum::{Infinite, Real, N64};
use rand::prelude::ThreadRng;
use tracing::info;

use super::*;

pub struct MctsSnake<T> {
    game: T,
}

impl<T> MctsSnake<T> {
    pub fn new(game: T) -> Self {
        Self { game }
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

            let snake = MctsSnake::new(game);

            Box::new(snake)
        } else {
            let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(game);

            Box::new(snake)
        }
    }
    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
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
            + YouDeterminableGame,
    > BattlesnakeAI for MctsSnake<T>
{
    fn make_move(&self) -> Result<MoveOutput> {
        let mut rng = rand::thread_rng();

        let mut root_node = Node::new(self.game.clone());

        root_node.expand();

        let total_number_of_iterations = 1;

        // This will eventually be in the loop

        let mut next_leaf_node = root_node.next_leaf_node(total_number_of_iterations);

        // If next_leaf_node HAS been visited, then we expand it
        if next_leaf_node.number_of_visits > 0 {
            next_leaf_node.expand();
            next_leaf_node = next_leaf_node.next_leaf_node(total_number_of_iterations);
        }

        //Now we do a simulation for this leaf node
        let score = next_leaf_node.simulate(&mut rng);
        dbg!(&score);

        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }

    fn end(&self) {
        info!("Mcts has ended");
    }
}

#[derive(Debug, PartialEq)]
struct Node<T> {
    game_state: T,
    total_score: N64,
    number_of_visits: usize,
    children: Option<Vec<Node<T>>>,
}

#[derive(Debug)]
pub struct Instrument {}
impl SimulatorInstruments for Instrument {
    fn observe_simulation(&self, _duration: std::time::Duration) {
        //No-oping here
    }
}

impl<T> Node<T> {
    fn new(game_state: T) -> Self {
        Self {
            game_state,
            total_score: N64::from(0.0),
            number_of_visits: 0,
            children: None,
        }
    }
}

impl<T> Node<T>
where
    T: SimulableGame<Instrument, 4>
        + SnakeIDGettableGame
        + RandomReasonableMovesGame
        + Clone
        + VictorDeterminableGame
        + YouDeterminableGame,
{
    fn simulate(&self, rng: &mut ThreadRng) -> N64 {
        // TODO: This clone might not be the best
        let mut current_state = self.game_state.clone();

        while !current_state.is_over() {
            let random_moves: Vec<_> = current_state
                .random_reasonable_move_for_each_snake(rng)
                .map(|(sid, mv)| (sid, [mv]))
                .collect();

            let next_state = {
                let mut simulation_result =
                    current_state.simulate_with_moves(&Instrument {}, random_moves);

                // TODO: This unwrap might NOT be safe
                simulation_result.next().unwrap().1.clone()
            };

            current_state = next_state;
        }

        let you_id = current_state.you_id();
        match current_state.get_winner() {
            Some(sid) => {
                if &sid == you_id {
                    N64::from(1.0)
                } else {
                    N64::from(-1.0)
                }
            }
            None => N64::from(0.0),
        }
    }

    fn has_been_expanded(&self) -> bool {
        self.children.is_some()
    }

    fn ucb1_score(&self, total_number_of_iterations: usize) -> N64 {
        let constant: N64 = N64::from(2.0);

        if self.number_of_visits == 0 {
            return N64::INFINITY;
        }

        let number_of_visits = N64::from(self.number_of_visits as f64);

        let average_score = self.total_score / number_of_visits;
        let total_number_of_iterations = N64::from(total_number_of_iterations as f64);
        let ln_total_number_of_iterations = total_number_of_iterations.ln();

        let right_hand_side = constant * (ln_total_number_of_iterations / number_of_visits).sqrt();

        average_score + right_hand_side
    }

    fn next_leaf_node(&mut self, total_number_of_iterations: usize) -> &mut Node<T> {
        let mut best_node = self;

        while !best_node.is_leaf() {
            debug_assert!(best_node.has_been_expanded());

            best_node = best_node
                .best_child(total_number_of_iterations)
                .expect("We are not a leaf node so we should have a best child");
        }

        best_node
    }

    fn is_leaf(&self) -> bool {
        self.children.is_none() || self.children.as_ref().unwrap().is_empty()
    }

    fn best_child(&mut self, total_number_of_iterations: usize) -> Option<&mut Node<T>> {
        debug_assert!(self.has_been_expanded());
        let children = self.children.as_mut().unwrap();

        // TODO: Get a total number of iterations here
        children
            .iter_mut()
            .max_by_key(|child| child.ucb1_score(total_number_of_iterations))
    }

    fn expand(&mut self) {
        debug_assert!(!self.has_been_expanded());

        let snakes = self.game_state.get_snake_ids();

        let next_states = self.game_state.simulate(&Instrument {}, snakes);

        // TODO: Keep the actions around here somehow so that we know which direction to move based
        // on the nodes we 'like'

        let children: Vec<Node<T>> = next_states
            .into_iter()
            .map(|(_, game_state)| Node::new(game_state))
            .collect();

        self.children = Some(children);
    }
}

#[cfg(test)]
mod test {
    use decorum::Infinite;

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
        let n = Node {
            game_state: game,
            total_score: N64::from(10.0),
            number_of_visits: 1,
            children: None,
        };

        assert_eq!(n.ucb1_score(1), N64::from(10.0));
        assert!(n.ucb1_score(2) > 11.6);
        assert!(n.ucb1_score(2) < 11.7);
    }
}