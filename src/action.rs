#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    Maximize,
}
