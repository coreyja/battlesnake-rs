use battlesnake_game_types::types::{Move, SnakeIDGettableGame};
use dotavious::{attributes::NodeAttributes, EdgeBuilder, GraphBuilder, NodeBuilder};
use std::{fmt::Debug, sync::atomic::AtomicUsize};
use text_trees::StringTreeNode;

use super::WrappedScore;

#[derive(Debug, Clone)]
/// This is returned from an iteration of the minimax algorithm
/// It contains all the information we generated about the game tree
pub enum MinMaxReturn<
    GameType: SnakeIDGettableGame + Clone + Debug,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
> {
    /// This is a non-leaf node in the game tree
    /// We have information about all the options we looked at as well as the chosen score
    Node {
        /// Whether this node was a maximizing node or not
        is_maximizing: bool,
        /// A 'recursive' look at all the moves under us
        /// This array is sorted by the score of the move, and whether we were in a maximizing or minimizing node
        /// The first element is always the chosen move at this node. It's [MinMaxReturn::score()]
        /// should always equal the score attribute of this node
        options: Vec<(Move, Self)>,
        /// Which snake was moving at this node
        moving_snake_id: GameType::SnakeIDType,
        /// The chosen score
        /// This should always match the score of the first element in [MinimaxReturn.options]
        score: WrappedScore<ScoreType>,

        /// Did this node see an Alpha-Beta Cutoff
        alpha_beta_cutoff: bool,

        /// Depth in the tree
        depth: i64,

        /// Alpha value when node is done
        alpha: WrappedScore<ScoreType>,
        /// Beta value when node is done
        beta: WrappedScore<ScoreType>,
    },
    /// Represents a leaf node in the game tree
    /// This happens when we reach a terminal state (win/lose/tie)
    /// or when we reach the maximum depth
    Leaf {
        #[allow(missing_docs)]
        score: WrappedScore<ScoreType>,
    },
}

impl<GameType, ScoreType> MinMaxReturn<GameType, ScoreType>
where
    GameType: SnakeIDGettableGame + Debug + Clone,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
{
    /// Returns the score for this node
    pub fn score(&self) -> &WrappedScore<ScoreType> {
        match self {
            MinMaxReturn::Node { score, .. } => score,
            MinMaxReturn::Leaf { score } => score,
        }
    }

    /// Returns the score for this node
    pub fn moving_snake_id(&self) -> std::option::Option<&GameType::SnakeIDType> {
        match self {
            MinMaxReturn::Node {
                moving_snake_id, ..
            } => Some(moving_snake_id),
            MinMaxReturn::Leaf { .. } => None,
        }
    }

    /// Returns the direction you should move to maximize the score
    /// If we are a leaf node, this will return None
    ///
    /// We take advantage of the fact that the moves are sorted by score, so we can just return the
    /// first option where our snake is moving
    /// of 'ourself'
    ///
    /// WARNING: If the given snake_id does NOT correspond to 'you' this method may not return the correct
    /// results, as it leans into sorting specific for your snake
    ///
    /// TODO: Fix this API. We only need the you_id to be able to find our move.
    pub fn your_best_move(&self, you_id: &GameType::SnakeIDType) -> Option<Move> {
        self.first_options_for_snake(you_id)
            .and_then(|options| options.first().map(|x| x.0))
    }

    /// Return the first set of move options for the given snake_id
    pub fn first_options_for_snake(
        &self,
        sid: &GameType::SnakeIDType,
    ) -> Option<&Vec<(Move, Self)>> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                ..
            } => {
                if moving_snake_id == sid {
                    Some(options)
                } else {
                    let chosen = options.first()?;
                    chosen.1.first_options_for_snake(sid)
                }
            }
        }
    }

    /// Return the option that matches the given move
    /// Returns None if we are at a leaf or the move does not exist
    pub fn option_for_move(&self, chosen_move: Move) -> Option<&Self> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
            MinMaxReturn::Node { options, .. } => options
                .iter()
                .find(|(m, _ret)| m == &chosen_move)
                .map(|x| &x.1),
        }
    }

    /// Check if the move you want to pick is certain death or not
    pub fn your_move_is_death(&self, you_id: &GameType::SnakeIDType, potential_move: Move) -> bool {
        if let Some(options) = self.first_options_for_snake(you_id) {
            matches!(
                options
                    .iter()
                    .find(|(move_, _)| *move_ == potential_move)
                    .map(|(_, r)| r.score()),
                Some(WrappedScore::Lose(..)) | None,
            )
        } else {
            false
        }
    }

    /// Returns all the moves in the 'route' through the game tree that minimax took
    /// This is useful for debugging as it shows each of the moves we and our opponents made during
    /// the simulation
    pub fn chosen_route(&self) -> Vec<(GameType::SnakeIDType, Move)> {
        match self {
            MinMaxReturn::Leaf { .. } => vec![],
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                ..
            } => {
                if let Some(chosen) = options.first() {
                    let mut tail = chosen.1.chosen_route();
                    tail.insert(0, (moving_snake_id.clone(), chosen.0));
                    tail
                } else {
                    vec![]
                }
            }
        }
    }

    /// To dot graph to view elsewhere
    pub fn to_dot_graph(&self, you_id: &GameType::SnakeIDType) -> dotavious::Dot<'_> {
        let mut builder = GraphBuilder::new_named_directed("example");
        self.to_dot_graph_recursive(&mut builder, you_id);
        builder.add_attribute(
            dotavious::attributes::AttributeType::Graph,
            "overlap",
            "scale".into(),
        );
        builder.add_attribute(
            dotavious::attributes::AttributeType::Graph,
            "splines",
            "ortho".into(),
        );
        let graph = builder.build().unwrap();

        dotavious::Dot { graph }
    }

    fn to_dot_graph_recursive(
        &self,
        builder: &mut GraphBuilder,
        you_id: &GameType::SnakeIDType,
    ) -> String {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);

        match self {
            MinMaxReturn::Leaf { score, .. } => {
                let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let me_id = format!("{id}");
                let me_label = format!("{score:?}");
                let node = NodeBuilder::new(&*me_id)
                    .add_attribute("label", me_label.into())
                    .add_attribute("style", "filled".into())
                    .add_attribute("fillcolor", "lightgreen".into())
                    .build()
                    .unwrap();
                builder.add_node(node);

                me_id
            }
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                score,
                alpha_beta_cutoff,
                depth,
                alpha,
                beta,
                ..
            } => {
                let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let me_id = format!("{id}");
                let me_label = format!(
                    "{depth}\n{score:?}\n{alpha_beta_cutoff}\nAlpha: {alpha:?}\nBeta: {beta:?}",
                );
                let color = if moving_snake_id == you_id {
                    "lightblue"
                } else {
                    "lightcoral"
                };
                let node = NodeBuilder::new(&*me_id)
                    .add_attribute("label", me_label.into())
                    .add_attribute("style", "filled".into())
                    .add_attribute("fillcolor", color.into())
                    .build()
                    .unwrap();
                builder.add_node(node);
                for (m, result) in options {
                    let next_node_id = result.to_dot_graph_recursive(builder, you_id);

                    let edge = EdgeBuilder::new(&*me_id, &*next_node_id)
                        .add_attribute("xlabel", format!("{moving_snake_id:?} {m}").into())
                        .add_attribute("color", color.into())
                        .build()
                        .unwrap();
                    builder.add_edge(edge);
                }

                me_id
            }
        }
    }

    /// This returns a visual representation of the game tree that minimax generated
    /// It shows the chosen score, the moving snake and the chosen move at each level
    pub fn to_text_tree(&self) -> Option<String> {
        let tree_node = self.to_text_tree_node("".to_owned())?;
        Some(format!("{tree_node}"))
    }

    fn to_text_tree_node(&self, label: String) -> Option<StringTreeNode> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                score,
                ..
            } => {
                let mut node = StringTreeNode::new(format!("{label} {score:?}"));
                for (m, result) in options {
                    if let Some(next_node) =
                        result.to_text_tree_node(format!("{m} {moving_snake_id:?}"))
                    {
                        node.push_node(next_node);
                    }
                }

                Some(node)
            }
        }
    }
}
