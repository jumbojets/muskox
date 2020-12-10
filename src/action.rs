use std::cmp;
use std::mem;

use crate::error::ParseActionError;

// need lookup table for square index for next direction

/// Represents one of the four directions one can move in the game of checkers
#[derive(Debug, PartialEq)]
pub enum Direction {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

/// Represents one of the two types of moves that exist in checkers
#[derive(Debug, PartialEq)]
pub enum ActionType {
    Move,
    Jump,
}

// source: 5, destination: 5, jump length: 5, jump directions: 8 * 2 bits (four directions), unused: 1
/// Represents an action that can be made on a checkerboard
pub struct Action(u32);

impl Action {
    /// Creates a new checkers action from a vector of positions.
    ///
    /// # Arguments
    ///
    /// * `positions` - A vector of standard position numbers representing a move
    ///
    /// # Examples
    ///
    /// ```
    /// use muskox::Action;
    ///
    /// let action = Action::new_from_vector(vec![19, 24]).unwrap();
    /// assert_eq!(action.source(), 18);  // note that internal representation starts from 0, no longer 1.
    pub fn new_from_vector(positions: Vec<u8>) -> Result<Action, ParseActionError> {
        let positions: Vec<_> = positions.iter().map(|x| x - 1).collect();

        // check that all of the position numbers are in the right range
        if let Some(pos) = positions.iter().find(|&&x| x > 31) {
            return Err(ParseActionError::PositionValueError { position: pos.to_string() });
        }

        // check to see if it is a valid length of position vector with max number of moves is 8
        if positions.len() < 2 || positions.len() > 9 {
            return Err(ParseActionError::MoveQuantityError { quantity: positions.len() });
        }

        let source = positions[0];
        let destination = *positions.last().unwrap();

        let mut data = source as u32;                      // source
        data |= (destination as u32) << 5;                 // destination

        let abs_diff = cmp::max(source, destination) - cmp::min(source, destination);

        // check if this action has jumps in it
        if positions.len() > 2 || (abs_diff != 3 && abs_diff != 4 && abs_diff != 5) {
            data |= ((positions.len() - 1) << 10) as u32;  // jump length

            for i in 0..(positions.len() - 1) {
                let diff = (positions[i + 1] as i8) - (positions[i] as i8);
                let direction = match diff {
                    -9 => Direction::UpLeft,
                    -7 => Direction::UpRight,
                    7 => Direction::DownLeft,
                    9 => Direction::DownRight,
                    _ => return Err(ParseActionError::PositionValueError { position: positions[i].to_string() }),
                };

                let shift = i * 2 + 15;
                data |= (direction as u32) << shift;      // jump direction
            }
        }

        Ok(Action(data))
    }

    /// Creates a new checkers action from a string movetext according to Portable Draughts Notation.
    /// (PDN). Read more about the notation [here](https://en.wikipedia.org/wiki/Portable_Draughts_Notation).
    ///
    /// # Arguments
    ///
    /// * `movetext` - A string slice that that represents movetext written for PDN notation
    ///
    /// # Examples
    ///
    /// ```
    /// use muskox::Action;
    ///
    /// let action = Action::new_from_movetext("19-24").unwrap();
    /// assert_eq!(action.source(), 18);  // note that internal representation starts from 0, no longer 1.
    /// ```
    pub fn new_from_movetext(movetext: &str) -> Result<Action, ParseActionError> {
        let positions: Vec<_> = movetext.split("-")
            .map(|x| x.parse::<u8>()
                .or(Err(ParseActionError::PositionValueError { position: x.to_string() })))
            .collect::<Result<_, ParseActionError>>()?;

        Action::new_from_vector(positions)
    }

    /// Returns the starting location of a particular action
    #[inline]
    pub fn source(&self) -> u8 {
        (self.0 & 31) as u8
    }

    /// Returns the ending location of a particular action
    #[inline]
    pub fn destination(&self) -> u8 {
        ((self.0 >> 5) & 31) as u8
    }

    /// Returns how many leaps were made in a particular action
    #[inline]
    pub fn jump_len(&self) -> u8 {
        ((self.0 >> 10) & 15) as u8
    }

    /// Returns the direction of a particular jump
    ///
    /// This is wrapped in an option, because if no jumps were performed then
    /// no jump directions can be retrieved (`None`).
    ///
    /// # Arguments
    ///
    /// * `i` - The index of the jump to find the direction for
    ///
    #[inline]
    pub fn jump_direction(&self, i: u8) -> Option<Direction> {
        // maybe rename to jump_direction
        if i >= self.jump_len() {
            return None
        }
        let val = (self.0 >> (i * 2 + 15)) & 3;
        Some(unsafe { mem::transmute(val as u8) })
    }

    /// Returns the type of a particular action
    #[inline]
    pub fn action_type(&self) -> ActionType {
        match self.jump_len() {
            0 => ActionType::Move,
            _ => ActionType::Jump,
        }
    }

    /// Returns the direction of a move action.
    ///
    /// This is also wrapped in an option, because if the action represents a
    /// jump, then a notion of a move direction is not relevant.
    // currently too bit for inline. try to pare this down a bit
    // #[inline]
    pub fn move_direction(&self) -> Option<Direction> {
        // ideally would like to condense this method
        // also check logic again soon

        if self.action_type() == ActionType::Jump {
            return None;
        }

        let source = self.source();
        let destination = self.destination();
        let diff = (destination as i8) - (source as i8);
        // see if we can use shifting and bitmasks to make it most efficient!
        if source / 4 % 2 == 0 {  // even rows
            return match diff {
                -4 => Some(Direction::UpLeft),
                -3 => Some(Direction::UpRight),
                4 => Some(Direction::DownLeft),
                5 => Some(Direction::DownRight),
                _ => None
            };
        } else {                  // odd rows
            return match diff {
                -5 => Some(Direction::UpLeft),
                -4 => Some(Direction::UpRight),
                3 => Some(Direction::DownLeft),
                4 => Some(Direction::DownRight),
                _ => None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MOVE_1: &'static str = "1-10-17";
    static TEST_MOVE_2: &'static str = "1-6";
    static TEST_MOVE_3: &'static str = "10-19-12-3";
    static TEST_MOVE_4: &'static str = "15-11";

    #[test]
    fn action_overview_test() {
        let action = Action::new_from_movetext(TEST_MOVE_1).unwrap();
        assert_eq!(action.source(), 0);
        assert_eq!(action.destination(), 16);
        assert_eq!(action.jump_len(), 2);
        assert_eq!(action.action_type(), ActionType::Jump);

        let action = Action::new_from_movetext(TEST_MOVE_2).unwrap();
        assert_eq!(action.source(), 0);
        assert_eq!(action.destination(), 5);
        assert_eq!(action.jump_len(), 0);
        assert_eq!(action.action_type(), ActionType::Move);

        let action = Action::new_from_movetext(TEST_MOVE_3).unwrap();
        assert_eq!(action.source(), 9);
        assert_eq!(action.destination(), 2);
        assert_eq!(action.jump_len(), 3);
        assert_eq!(action.action_type(), ActionType::Jump);
    }

    #[test]
    fn direction_test() {
        let action = Action::new_from_movetext(TEST_MOVE_1).unwrap();
        assert_eq!(action.jump_direction(0), Some(Direction::DownRight));
        assert_eq!(action.jump_direction(1), Some(Direction::DownLeft));
        assert_eq!(action.jump_direction(2), None);

        let action = Action::new_from_movetext(TEST_MOVE_2).unwrap();
        assert_eq!(action.jump_direction(0), None);

        let action = Action::new_from_movetext(TEST_MOVE_3).unwrap();
        assert_eq!(action.jump_direction(1), Some(Direction::UpRight));
        assert_eq!(action.jump_direction(2), Some(Direction::UpLeft));
        assert_eq!(action.jump_direction(4), None);
    }

    #[test]
    fn move_direction_test() {
        let action = Action::new_from_movetext(TEST_MOVE_1).unwrap();
        assert_eq!(action.move_direction(), None);

        let action = Action::new_from_movetext(TEST_MOVE_2).unwrap();
        assert_eq!(action.move_direction(), Some(Direction::DownRight));

        let action = Action::new_from_movetext(TEST_MOVE_4).unwrap();
        assert_eq!(action.move_direction(), Some(Direction::UpRight));
    }
}