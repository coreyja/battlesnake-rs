use battlesnake_game_types::types::Move;
use itertools::Itertools;
use nalgebra::{matrix, DMatrix, DVector};

#[allow(dead_code)]
fn idx_vector_to_xy_vector(idx_vector: DVector<u8>, width: i8) -> DMatrix<i8> {
    let width_vector = DVector::from_element(idx_vector.len(), width);

    let idx_vector: DVector<i8> = idx_vector.cast();
    let y = idx_vector.component_div(&width_vector);
    let x = {
        let mut x = y.clone();
        x.axpy(1, &idx_vector, -width);
        x
    };
    DMatrix::from_columns(&[x, y])
}

fn move_to_xy_add_matrix(m: Move, n: usize) -> DMatrix<i8> {
    let x_col = match m {
        Move::Up | Move::Down => DVector::from_element(n, 0),
        Move::Left => DVector::from_element(n, -1),
        Move::Right => DVector::from_element(n, 1),
    };
    let y_col = match m {
        Move::Right | Move::Left => DVector::from_element(n, 0),
        Move::Up => DVector::from_element(n, 1),
        Move::Down => DVector::from_element(n, -1),
    };

    DMatrix::from_columns(&[x_col, y_col])
}

#[allow(dead_code)]
pub fn multi_neighbor(positions: &[u8], width: u8) -> DMatrix<i8> {
    let width_i = width as i8;

    let n = positions.len();

    let positions: DVector<u8> = DVector::from_column_slice(positions);

    let xy_matrix = idx_vector_to_xy_vector(positions, width_i);
    let xy_to_idx_matrix = matrix!(1; width_i);

    DMatrix::from_columns(
        &Move::all_iter()
            .map(|m| {
                let add_matrix = move_to_xy_add_matrix(m, n);

                let direction = xy_matrix.clone() + add_matrix;
                let direction = direction.map(|x| x.rem_euclid(width_i));

                let direction = direction * xy_to_idx_matrix;
                direction
            })
            .collect_vec(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix() {
        let positions = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let expected: Vec<Vec<i8>> = vec![];

        assert_eq!(
            expected,
            multi_neighbor(&positions, 10)
                .row_iter()
                .map(|x| x.iter().cloned().collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
    }
}
