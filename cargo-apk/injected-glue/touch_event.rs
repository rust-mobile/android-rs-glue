use ffi;

#[derive(Clone, Copy, Debug)]
pub enum TouchEventType{
    Down,
    PointerDown,
    Move,
    PointerUp,
    Up,
    Cancel,
}

impl TouchEventType{
    fn from_input_event(event: *const ffi::AInputEvent) -> TouchEventType{

        // let action = unsafe {ffi::AMotionEvent_getAction(event)} & ffi::AMOTION_EVENT_ACTION_MASK;

        match unsafe {ffi::AMotionEvent_getAction(event)} & ffi::AMOTION_EVENT_ACTION_MASK {
            // match action {
            ffi::AMOTION_EVENT_ACTION_DOWN => //| ffi::AMOTION_EVENT_ACTION_POINTER_DOWN =>
                TouchEventType::Down,
            ffi::AMOTION_EVENT_ACTION_MOVE =>
                TouchEventType::Move,
            ffi::AMOTION_EVENT_ACTION_POINTER_DOWN =>
                TouchEventType::PointerDown,
            ffi::AMOTION_EVENT_ACTION_POINTER_UP =>
                TouchEventType::PointerUp,
            ffi::AMOTION_EVENT_ACTION_UP => //| ffi::AMOTION_EVENT_ACTION_POINTER_UP =>
                TouchEventType::Up,
            _ => TouchEventType::Cancel
        }

        // if action == ffi::AMOTION_EVENT_ACTION_DOWN {
        //     TouchEventType::Down
        // } else if action == ffi::AMOTION_EVENT_ACTION_POINTER_DOWN {
        //     TouchEventType::PointerDown
        // } else if action == ffi::AMOTION_EVENT_ACTION_UP {
        //     TouchEventType::Up
        // } else if action == ffi::AMOTION_EVENT_ACTION_POINTER_UP {
        //     TouchEventType::PointerUp
        // } else if action == ffi::AMOTION_EVENT_ACTION_MOVE{
        //     TouchEventType::Move
        // } else{
        //     TouchEventType::Cancel
        // }
    }
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

impl TouchEvent {

    pub fn from_input_event(event: *const ffi::AInputEvent) -> TouchEvent {

        let n = unsafe {ffi::AMotionEvent_getPointerCount(event)};

        TouchEvent {
            timestamp: unsafe {ffi::AMotionEvent_getEventTime(event)},
            p0: Pointer::from_input_event(event, 0),
            num_pointers: n as u8,
            p1: if n > 1 {Some(Pointer::from_input_event(event, 1))} else {None},
            p2: if n > 2 {Some(Pointer::from_input_event(event, 2))} else {None},
            p3: if n > 3 {Some(Pointer::from_input_event(event, 3))} else {None},
            event_type: TouchEventType::from_input_event(event),
            flag: 0,
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub enum PointerState{
    Released,
    Pressed,
    Moved,
    Stationary,
    Cancelled,
}

impl PointerState {
    fn from_input_event(event: *const ffi::AInputEvent, pointer_idx: usize) -> PointerState {
        let action = unsafe {ffi::AMotionEvent_getAction(event)};

        // primary pointer;
        if action == ffi::AMOTION_EVENT_ACTION_DOWN {
            return PointerState::Pressed;
        }else if action == ffi::AMOTION_EVENT_ACTION_UP {
            return PointerState::Released;
        }

        // actions regardless of pointer index;
        if action == ffi::AMOTION_EVENT_ACTION_MOVE {
            return PointerState::Moved;
        }else if action == ffi::AMOTION_EVENT_ACTION_CANCEL {
            return PointerState::Cancelled;
        }

        // index where the action occured;
        let action_idx = (action & ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_MASK) >> ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT;
        if (pointer_idx as i32) != action_idx {
            return PointerState::Stationary;
        }

        let action_masked = action & ffi::AMOTION_EVENT_ACTION_MASK;

        if action_masked == ffi::AMOTION_EVENT_ACTION_POINTER_DOWN {
            return PointerState::Pressed;
        }else if action_masked == ffi::AMOTION_EVENT_ACTION_POINTER_UP {
            return PointerState::Released;
        }

        PointerState::Stationary
    }
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

impl Pointer {
    fn from_input_event(event: *const ffi::AInputEvent, idx:usize) -> Pointer {
        Pointer {
            state: PointerState::from_input_event(event, idx),
            id: unsafe {ffi::AMotionEvent_getPointerId(event, idx)},
            x: unsafe {ffi::AMotionEvent_getX(event, idx)},
            y: unsafe {ffi::AMotionEvent_getY(event, idx)},
            vertical_radius: unsafe {ffi::AMotionEvent_getTouchMajor(event, idx)} / 2.0,
            horizontal_radius: unsafe {ffi::AMotionEvent_getTouchMinor(event, idx)} / 2.0,
            pressure: 0.0,
			      rotation_angle:0.0,
        }
	  }
}
