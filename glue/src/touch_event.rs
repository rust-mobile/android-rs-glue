#[derive(Clone, Copy, Debug)]
pub enum TouchEventType{
    Down,
    PointerDown,
    Move,
    PointerUp,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug)]
pub struct TouchEvent {
    pub event_type: TouchEventType,
    pub timestamp: i64,
    pub num_pointers: u8,
    pub p0: Pointer,
    pub p1: Option<Pointer>,
    pub p2: Option<Pointer>,
    pub p3: Option<Pointer>,
    pub flag: i32,
}

#[derive(Clone, Copy, Debug)]
pub enum PointerState{
    Released,
    Pressed,
    Moved,
    Stationary,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct Pointer {
    pub state: PointerState,
    pub x: f32,
    pub y: f32,
    pub id: i32,
    pub pressure: f32,
    pub vertical_radius: f32,
    pub horizontal_radius: f32,
    pub rotation_angle: f32,
}
