
#[derive(Clone, Copy, Debug)]
pub enum TouchEventType{
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Clone, Copy, Debug)]
pub struct TouchEvent {
    pub event_type: TouchEventType,
    pub timestamp: i64,
    pub p0: TouchPoint,
    pub p1: Option<TouchPoint>,
    pub p2: Option<TouchPoint>,
    pub p3: Option<TouchPoint>,
    pub flag: i32,
}

#[derive(Clone, Copy, Debug)]
pub enum TouchPointState{
    Released,
    Pressed,
    Moved,
    Stationary,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct TouchPoint {
    pub state: TouchPointState,
    pub x: f32,
    pub y: f32,
    pub id: i32,
    pub pressure: f32,
    pub vertical_radius: f32,
    pub horizontal_radius: f32,
    pub rotation_angle: f32,
}
