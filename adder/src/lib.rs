pub fn add(left: usize, right: usize) -> usize {
    left + right
}

struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
}

pub struct Guess {
    val: i32,
}

impl Guess {
    pub fn new(val: i32) -> Guess {
        if val < 1 || val > 100 {
            panic!("Guess value must be between 1 and 100, got {}", val);
        }
        Guess { val }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exploration() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn larger_can_hold_smaller() {
        let rect1 = Rectangle {
            width: 8,
            height: 7
        };
        let rect2 = Rectangle {
            width: 4,
            height: 3
        };
        assert!(rect1.can_hold(&rect2));
    }

    #[test]
    fn smaller_cannot_hold_larger() {
        let rect1 = Rectangle {
            width: 8,
            height: 7
        };
        let rect2 = Rectangle {
            width: 4,
            height: 3
        };
        assert!(!rect2.can_hold(&rect1));
    }

    #[test]
    #[should_panic]
    fn greater_than_100 () {
        Guess::new(200);
    }
}
