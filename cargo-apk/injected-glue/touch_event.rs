use ffi;

#[derive(Clone, Copy, Debug)]
pub enum TouchEventType{
    Start,
    Move,
    End,
    Cancel,
}

impl TouchEventType{
    fn from_input_event(event: *const ffi::AInputEvent) -> TouchEventType{
        let action = unsafe {ffi::AMotionEvent_getAction(event)} & ffi::AMOTION_EVENT_ACTION_MASK;
        if action == ffi::AMOTION_EVENT_ACTION_DOWN || action == ffi::AMOTION_EVENT_ACTION_POINTER_DOWN {
            TouchEventType::Start
        }else if action == ffi::AMOTION_EVENT_ACTION_UP || action == ffi::AMOTION_EVENT_ACTION_POINTER_UP{
            TouchEventType::End
        }else if action == ffi::AMOTION_EVENT_ACTION_MOVE{
            TouchEventType::Move
        }else{
            TouchEventType::Cancel
        }
    }
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

impl TouchEvent {

    pub fn from_input_event(event: *const ffi::AInputEvent) -> TouchEvent {

        let mut points: Vec<TouchPoint> = vec![];
        let n = unsafe {ffi::AMotionEvent_getPointerCount(event)};

        TouchEvent {
            timestamp: unsafe {ffi::AMotionEvent_getEventTime(event)},
            p0: TouchPoint::from_input_event(event, 0),
			p1: if n > 1 {Some(TouchPoint::from_input_event(event, 1))} else {None},
			p2: if n > 2 {Some(TouchPoint::from_input_event(event, 2))} else {None},
			p3: if n > 3 {Some(TouchPoint::from_input_event(event, 3))} else {None},
            event_type: TouchEventType::from_input_event(event),
            flag: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TouchPointState{
    Released,
    Pressed,
    Moved,
    Stationary,
    Cancelled,
}

impl TouchPointState {
    fn from_input_event(event: *const ffi::AInputEvent, pointer_idx: usize) -> TouchPointState {
        let action = unsafe {ffi::AMotionEvent_getAction(event)};

        // primary pointer;
        if action == ffi::AMOTION_EVENT_ACTION_DOWN {
            return TouchPointState::Pressed;
        }else if action == ffi::AMOTION_EVENT_ACTION_UP {
            return TouchPointState::Released;
        }

        // actions regardless of pointer index;
        if action == ffi::AMOTION_EVENT_ACTION_MOVE {
            return TouchPointState::Moved;
        }else if action == ffi::AMOTION_EVENT_ACTION_CANCEL {
            return TouchPointState::Cancelled;
        }

        // index where the action occured;
        let action_idx = (action & ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_MASK) >> ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT;
        if (pointer_idx as i32) != action_idx {
            return TouchPointState::Stationary;
        }

        let action_masked = action & ffi::AMOTION_EVENT_ACTION_MASK;
        if action_masked == ffi::AMOTION_EVENT_ACTION_POINTER_DOWN {
            return TouchPointState::Pressed;
        }else if action_masked == ffi::AMOTION_EVENT_ACTION_POINTER_UP {
            return TouchPointState::Released;
        }

        TouchPointState::Stationary
    }
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

impl TouchPoint {
	fn from_input_event(event: *const ffi::AInputEvent, idx:usize) -> TouchPoint {
		TouchPoint{
			state: TouchPointState::from_input_event(event, idx),
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
