use super::*;

pub(crate) fn game() -> Game {
    serde_json::from_str(include_str!("../../../../fixtures/jev-standard.json")).unwrap()
}

fn body(game: &mut Game, positions: &[(i32, i32)]) {
    game.you.body = positions
        .iter()
        .map(|&(x, y)| Position::new(x, y))
        .collect();
    game.you.head = game.you.body[0];
    game.board.snakes[0] = game.you.clone();
}

fn directions(game: &Game) -> Vec<&'static str> {
    candidates(game).iter().map(|c| c.direction).collect()
}

#[test]
fn walls_neck_and_body_are_excluded() {
    let mut g = game();
    body(&mut g, &[(0, 0), (0, 1), (1, 1)]);
    assert_eq!(directions(&g), ["right"]);
    body(&mut g, &[(3, 3), (3, 2), (4, 2), (4, 3), (4, 4)]);
    assert_eq!(directions(&g), ["up", "left"]);
}

#[test]
fn tail_vacates_but_stacked_tail_and_neck_do_not() {
    let mut g = game();
    body(&mut g, &[(3, 3), (3, 2), (4, 2), (4, 3)]);
    assert!(directions(&g).contains(&"right"));
    // Eating duplicates the NEW tail: the old tail still vacates this turn.
    g.board.food.push(Position::new(4, 3));
    assert!(directions(&g).contains(&"right"));
    body(&mut g, &[(3, 3), (3, 2), (4, 2), (4, 3), (4, 3)]);
    assert!(!directions(&g).contains(&"right"));
    body(&mut g, &[(3, 3), (3, 2)]);
    assert!(!directions(&g).contains(&"down"));
}

#[test]
fn wrapped_edges_and_constrictor_tails() {
    let mut g = game();
    body(&mut g, &[(0, 3), (1, 3), (2, 3)]);
    g.game.ruleset.name = "wrapped".into();
    let moves = candidates(&g);
    assert_eq!(
        moves
            .iter()
            .find(|m| m.direction == "left")
            .unwrap()
            .destination,
        Position::new(6, 3)
    );
    body(&mut g, &[(3, 3), (3, 2), (4, 2), (4, 3)]);
    g.game.ruleset.name = "constrictor".into();
    g.you.health = 1;
    assert!(!directions(&g).contains(&"right"));
    assert!(directions(&g).contains(&"up"));
}

#[test]
fn starvation_stacked_hazards_and_food_rescue() {
    let mut g = game();
    g.you.health = 29;
    let up = Position::new(3, 4);
    g.board.hazards = vec![up, up];
    assert!(!directions(&g).contains(&"up"));
    g.board.food.push(up);
    assert_eq!(
        candidates(&g)
            .iter()
            .find(|m| m.direction == "up")
            .unwrap()
            .health_after_move,
        100
    );
    g.you.health = 1;
    assert_eq!(directions(&g), ["up"]);
}

#[test]
fn avoid_equal_or_longer_heads_but_allow_shorter_or_starving_opponents() {
    let mut g = game();
    let mut opponent = g.you.clone();
    opponent.id = "other".into();
    opponent.head = Position::new(3, 5);
    opponent.body = [
        Position::new(3, 5),
        Position::new(3, 6),
        Position::new(4, 6),
    ]
    .into();
    g.board.snakes.push(opponent);
    assert!(!directions(&g).contains(&"up"));
    g.board.snakes[1].body.pop_back();
    assert!(directions(&g).contains(&"up"));
    g.board.snakes[1].body.push_back(Position::new(4, 6));
    g.board.snakes[1].health = 1;
    assert!(directions(&g).contains(&"up"));
}

#[test]
fn hungry_fallback_follows_food_and_trapped_board_still_returns_direction() {
    let mut g = game();
    g.you.health = 20;
    assert_eq!(fallback(&g, &candidates(&g)), "up");
    g.board.width = 1;
    g.board.height = 1;
    body(&mut g, &[(0, 0), (0, 0), (0, 0)]);
    assert!(candidates(&g).is_empty());
    assert_eq!(fallback(&g, &[]), "up");
}
