use battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11;
use decorum::{Infinite, Real, N64};
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

impl<T: Clone + SimulableGame<Instrument, 4>> BattlesnakeAI for MctsSnake<T> {
    fn make_move(&self) -> Result<MoveOutput> {
        let _ = &self.game;
        let mut root_node = Node::new(self.game.clone());

        dbg!(&root_node.children);
        root_node.expand();
        dbg!(&root_node.children.as_ref().unwrap().len());

        let _best = root_node.best_child();

        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }

    fn end(&self) {
        info!("Mcts has ended");
    }
}

#[derive(Debug)]
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
    T: SimulableGame<Instrument, 4> + SnakeIDGettableGame,
{
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

    fn best_child(&self) -> Option<&Node<T>> {
        debug_assert!(self.has_been_expanded());
        let children = self.children.as_ref().unwrap();

        // TODO: Get a total number of iterations here
        children.iter().max_by_key(|child| child.ucb1_score(1))
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
